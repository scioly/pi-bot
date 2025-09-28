use common::env;
use dotenv::dotenv;
use log::{LevelFilter, debug};
use poise::serenity_prelude as serenity;
use serde::Deserialize;
use simple_logger::SimpleLogger;
use sqlx::MySqlPool;

use crate::discord::{BotContext, commands::all_commands};

mod discord;

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

    let env_config = env::load_env::<Env>().expect("should load and parse expected config struct");

    debug!("{:?}", env_config);

    let token = env_config.discord_token.clone();
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                ..Default::default()
            },
            commands: all_commands(),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                let env = env_config;
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                let pool = MySqlPool::connect(&env.database_url).await?;
                Ok(BotContext { env, db: pool })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
