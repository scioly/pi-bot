use crate::discord::Command;

mod auth;
mod event;
mod fun;
mod general;
mod sync;

pub fn all_commands() -> Vec<Command> {
    vec![
        auth::auth(),
        auth::whois(),
        event::event(),
        fun::fish(),
        fun::stealfish(),
        fun::trout(),
        fun::treat(),
        fun::dogbomb(),
        fun::shibabomb(),
        fun::magic8ball(),
        fun::xkcd(),
        general::about(),
        sync::sync(),
    ]
}
