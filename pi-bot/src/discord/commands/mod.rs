use crate::discord::{
    Command,
    commands::{auth::auth, event::event, sync::sync},
};

mod auth;
mod event;
mod sync;

pub fn all_commands() -> Vec<Command> {
    vec![auth(), event(), sync()]
}
