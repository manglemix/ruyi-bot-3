use std::env;

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;
use tracing::error;
use tracing_subscriber::EnvFilter;

struct Handler;

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
        if msg.content == "!ping" {
            unwrap!(msg.channel_id.say(&ctx.http, "Pong!").await);
        }
    }
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    search_master::initialize();

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        // | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot.
    let mut client =
        Client::builder(&token, intents).event_handler(Handler).await.expect("Err creating client");

    // Start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        error!("Client error: {why:?}");
    }
}