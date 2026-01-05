use common::env;
use std::sync::Arc;

use dotenv::dotenv;
use log::{LevelFilter, debug};
use poise::serenity_prelude::{self as serenity, Token};
use serde::Deserialize;
use simple_logger::SimpleLogger;
use sqlx::MySqlPool;

use crate::discord::{BotContext, commands::all_commands};

mod discord;
pub mod version;

#[derive(Debug, Clone, Deserialize)]
struct Env {
    pub discord_token: String,
    pub database_url: String,
    pub oauth_client_id: String,
    pub oauth_client_secret: String,
}

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Off)
        .with_module_level("pi_bot", LevelFilter::Info)
        .init()
        .expect("should initialize logger");
    dotenv().ok();

    let env = env::load_env::<Env>().expect("should load and parse expected config struct");

    debug!("{:?}", env);

    let token = env
        .discord_token
        .parse::<Token>()
        .expect("should parse valid Discord token");
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    // poise::builtins::register_globally(ctx, &framework.options().commands).await?;
    let pool = MySqlPool::connect(&env.database_url)
        .await
        .expect("should instantiate database pool and connect to database");
    let data = BotContext { env, db: pool };
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                ..Default::default()
            },
            commands: all_commands(),
            ..Default::default()
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .data(Arc::new(data))
        .framework(Box::new(framework))
        .await;
    client.unwrap().start().await.unwrap();
}
