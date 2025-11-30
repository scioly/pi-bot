use poise::serenity_prelude::{EditChannel, GuildChannel, Mentionable};

use crate::discord::{Context, Error};

/// Manages slowmode for a channel.
#[poise::command(
    slash_command,
    subcommands("set", "remove"),
    default_member_permissions = "MANAGE_CHANNELS",
    required_bot_permissions = "MANAGE_CHANNELS"
)]
pub async fn slowmode(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Sets the slowmode for a particular channel.
#[poise::command(slash_command, guild_only)]
pub async fn set(
    ctx: Context<'_>,
    #[description = "Optional. How long the slowmode delay should be, in seconds. If none, assumed to be 20 seconds."]
    delay: Option<u16>,
    #[description = "Optional. The channel to enable the slowmode in. If none, assumed in the current channel."]
    channel: Option<GuildChannel>,
) -> Result<(), Error> {
    let delay = delay.unwrap_or(20);
    let mut channel = channel.unwrap_or(ctx.guild_channel().await.unwrap());
    channel
        .edit(
            ctx.http(),
            EditChannel::default().rate_limit_per_user(delay),
        )
        .await?;
    ctx.reply(format!(
        "Enabled a slowmode delay of {} seconds in {}.",
        delay,
        channel.mention()
    ))
    .await?;
    Ok(())
}

/// Removes the slowmode set on a given channel.
#[poise::command(slash_command, guild_only)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Optional. The channel to enable the slowmode in. If none, assumed in the current channel."]
    channel: Option<GuildChannel>,
) -> Result<(), Error> {
    let mut channel = channel.unwrap_or(ctx.guild_channel().await.unwrap());
    channel
        .edit(ctx.http(), EditChannel::default().rate_limit_per_user(0))
        .await?;
    ctx.reply(format!(
        "Removed the slowmode delay in {}.",
        channel.mention()
    ))
    .await?;
    Ok(())
}
