use crate::discord::{Command, commands::sync::sync};

mod sync;

pub fn all_commands() -> Vec<Command> {
    vec![sync()]
}
