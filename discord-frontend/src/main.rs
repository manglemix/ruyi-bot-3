use std::collections::VecDeque;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use files_module::RUYI_FILES;
use rustc_hash::FxHashSet;
use search_master_interface::{invalidate_message_author_id, send_new_searchable_message, SearchableMessage};
use serenity::all::{CreateAttachment, CreateMessage, User};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;
use tokio::io::{AsyncWriteExt, BufWriter};
use tracing::error;
use tracing_subscriber::EnvFilter;

struct Handler {
    opt_in_users: RwLock<FxHashSet<u64>>,
    this_user: OnceLock<User>,
}

macro_rules! unwrap {
    ($expr: expr, $else: expr) => {
        match $expr {
            Ok(x) => x,
            Err(e) => {
                tracing::error!("{e}");
                return $else;
            }
        }
    };
    ($expr: expr) => {
        unwrap!($expr, ())
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let Some(msg_str) = msg.content.strip_prefix('!') else {
            let author_id = msg.author.id.get();
            if msg.author == *self.this_user.get().unwrap() || self.opt_in_users.read().await.contains(&author_id) {
                send_new_searchable_message(SearchableMessage::new(author_id, msg.content));
            }
            return;
        };
        if &msg.author == self.this_user.get().unwrap() {
            return;
        }
        if unwrap!(msg.channel_id.name(&ctx.http).await) != "ruyi" {
            return;
        }
        let mut msg_iter = msg_str.split_whitespace();
        let cmd = msg_iter.next().unwrap();
        match cmd {
            "ping" | "Ping" => {
                unwrap!(msg.channel_id.say(&ctx.http, "Pong!").await);
            }
            "file" => {
                let filename = msg_str.strip_prefix("file ").unwrap().trim();
                if filename.is_empty() {
                    unwrap!(msg.channel_id.say(&ctx.http, "`ERROR Expected: !file <filename>`").await);
                    return;
                };
                if filename.contains("..") {
                    unwrap!(msg.channel_id.say(&ctx.http, "`Invalid filename`").await);
                    return;
                };
                let filepath = Path::new(RUYI_FILES).join(filename);

                if msg.attachments.is_empty() {
                    // Reading a file
                    if !filepath.exists() {
                        unwrap!(msg.channel_id.say(&ctx.http, "`Does not exist`").await);
                        return;
                    }
                    if filepath.is_dir() {
                        unwrap!(msg.channel_id.say(&ctx.http, "`Is a folder`").await);
                        return;
                    }
                    let data = unwrap!(tokio::fs::read(filepath).await);
                    unwrap!(msg.channel_id.send_files(&ctx.http, [CreateAttachment::bytes(data, filename)], CreateMessage::new().content("Here it is")).await);
                } else if msg.attachments.len() > 1 {
                    unwrap!(msg.channel_id.say(&ctx.http, "`Expected 0 or 1 attachments`").await);
                    return;
                } else {
                    let file = msg.attachments.first().unwrap();
                    unwrap!(tokio::fs::create_dir_all(filepath.parent().unwrap()).await);
                    unwrap!(tokio::fs::write(filepath, unwrap!(file.download().await)).await);
                    unwrap!(msg.channel_id.say(&ctx.http, "`Saved`").await);
                }
            }
            "files" => {
                let page_idx_str = msg_iter.next().unwrap_or("0");
                let Ok(page_idx) = page_idx_str.parse::<usize>() else {
                    unwrap!(msg.channel_id.say(&ctx.http, "`Invalid index`").await);
                    return;
                };
                let mut out = String::from("```\n");
                let mut remaining = 10usize;
                let mut skip = remaining * page_idx;
                let mut queue = VecDeque::from(vec![PathBuf::from(RUYI_FILES)]);

                'main: while let Some(next) = queue.pop_front() {
                    let mut read_dir = unwrap!(tokio::fs::read_dir(&next).await);
                    while let Some(child) = unwrap!(read_dir.next_entry().await) {
                        if child.path().is_dir() {
                            queue.push_back(child.path());
                        } else if skip > 0 {
                            skip -= 1;
                        } else {
                            out.push_str(&child.path().strip_prefix(RUYI_FILES).unwrap().to_string_lossy());
                            out.push('\n');
                            remaining -= 1;
                            if remaining == 0 {
                                break 'main;
                            }
                        }
                    }
                }

                out.push_str("```");
                unwrap!(msg.channel_id.say(&ctx.http, out).await);
            }
            "opt-in" => {
                {
                    let mut guard = self.opt_in_users.write().await;
                    if guard.insert(msg.author.id.get()) {
                        write_opt_in_ids(guard.iter().copied()).await;
                    }
                }
                unwrap!(msg.channel_id.say(&ctx.http, "`Added user id to opt in list`").await);
            }
            "opt-out" => {
                {
                    let mut guard = self.opt_in_users.write().await;
                    if guard.remove(&msg.author.id.get()) {
                        write_opt_in_ids(guard.iter().copied()).await;
                    }
                }
                invalidate_message_author_id(msg.author.id.get());
                unwrap!(msg.channel_id.say(&ctx.http, "`Removed user id from opt in list`").await);
            }
            _ => {
                unwrap!(msg.channel_id.say(&ctx.http, "`Unknown command`").await);
            }
        }
    }

    async fn ready(&self, _ctx: Context, data_about_bot: serenity::all::Ready) {
        let _ = self.this_user.set(data_about_bot.user.into());
    }
}

async fn write_opt_in_ids(ids: impl Iterator<Item = u64>) {
    let mut file = BufWriter::new(tokio::fs::File::create("opt-in.txt").await.unwrap());
    for id in ids {
        file.write_all(id.to_string().as_bytes()).await.unwrap();
        file.write_all(b"\n").await.unwrap();
    }
    file.flush().await.unwrap();
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_line_number(true)
        .init();

    search_master::initialize();

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        // | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut opt_in_users = FxHashSet::<u64>::default();
    if Path::new("opt-in.txt").exists() {
        let contents = tokio::fs::read_to_string("opt-in.txt").await.unwrap();
        for line in contents.lines() {
            opt_in_users.insert(line.parse().unwrap());
        }
    }

    // Create a new instance of the Client, logging in as a bot.
    let mut client =
        Client::builder(&token, intents).event_handler(Handler {
            opt_in_users: RwLock::new(opt_in_users),
            this_user: OnceLock::new()
        }).await.expect("Err creating client");

    // Start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        error!("Client error: {why:?}");
    }
}