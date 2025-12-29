use crate::discord::Command;

mod auth;
mod event;
mod fun;
mod general;
mod staff;
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
        general::invite(),
        general::link(),
        general::random(),
        general::coach(),
        general::info(),
        staff::slowmode(),
        staff::nuke(),
        staff::mute(),
        sync::sync(),
    ]
}
