use crate::discord::utils::is_staff;
use crate::discord::{Context, Error};
use log::info;

#[poise::command(prefix_command, check = "is_staff")]
pub async fn sync(ctx: Context<'_>) -> Result<(), Error> {
    info!("Running sync command");
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}
