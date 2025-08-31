use sqlx::MySqlPool;

use crate::Env;

pub mod commands;
mod utils;

#[derive(Debug)]
pub struct BotContext {
    #[allow(dead_code)]
    pub env: Env,
    #[allow(dead_code)]
    pub db: MySqlPool,
}

pub type Command = poise::Command<BotContext, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, BotContext, Error>;
