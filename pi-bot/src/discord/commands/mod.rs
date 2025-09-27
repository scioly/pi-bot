use crate::discord::{
    Command,
    commands::{auth::auth, sync::sync},
};

mod auth;
mod sync;

pub fn all_commands() -> Vec<Command> {
    vec![auth(), sync()]
}
