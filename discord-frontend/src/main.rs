use std::env;
use std::path::Path;
use std::sync::OnceLock;

use rustc_hash::FxHashSet;
use search_master_interface::{
    SearchableMessage, invalidate_message_author_id, send_new_searchable_message,
};
use serenity::all::User;
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
    };
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let Some(msg_str) = msg.content.strip_prefix('!') else {
            let author_id = msg.author.id.get();
            if msg.author == *self.this_user.get().unwrap()
                || self.opt_in_users.read().await.contains(&author_id)
            {
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
            "opt-in" => {
                {
                    let mut guard = self.opt_in_users.write().await;
                    if guard.insert(msg.author.id.get()) {
                        write_opt_in_ids(guard.iter().copied()).await;
                    }
                }
                unwrap!(
                    msg.channel_id
                        .say(&ctx.http, "`Added user id to opt in list`")
                        .await
                );
            }
            "opt-out" => {
                {
                    let mut guard = self.opt_in_users.write().await;
                    if guard.remove(&msg.author.id.get()) {
                        write_opt_in_ids(guard.iter().copied()).await;
                    }
                }
                invalidate_message_author_id(msg.author.id.get());
                unwrap!(
                    msg.channel_id
                        .say(&ctx.http, "`Removed user id from opt in list`")
                        .await
                );
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
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler {
            opt_in_users: RwLock::new(opt_in_users),
            this_user: OnceLock::new(),
        })
        .await
        .expect("Err creating client");

    // Start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        error!("Client error: {why:?}");
    }
}
