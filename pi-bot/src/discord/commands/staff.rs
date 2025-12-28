use indoc::formatdoc;
use poise::{
    CreateReply,
    serenity_prelude::{
        ButtonStyle, Colour, ComponentInteractionDataKind, CreateActionRow, CreateButton,
        CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, EditChannel,
        GetMessages, GuildChannel, Mentionable,
        futures::future::{Either, select},
    },
};
use std::{pin::pin, time::Duration};
use tokio::time::Instant;

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

/// Staff command. Nukes a certain amount of messages.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MANAGE_MESSAGES",
    required_bot_permissions = "MANAGE_MESSAGES"
)]
pub async fn nuke(
    ctx: Context<'_>,
    #[description = "The amount of messages to nuke."]
    #[min = 1]
    #[max = 100]
    count: u8,
) -> Result<(), Error> {
    let channel = ctx.guild_channel().await.unwrap();
    let messages_to_delete = channel
        .messages(ctx.http(), GetMessages::new().limit(count))
        .await?;

    const NUKE_CANCEL_BUTTON_ID: &str = "nuke-cancel";
    const COUNTDOWN_START_SECS: u64 = 10;
    let cancel_button = CreateButton::new(NUKE_CANCEL_BUTTON_ID)
        .label("Cancel")
        .style(ButtonStyle::Danger);
    let embed = CreateEmbed::new()
        .title("NUKE COMMAND PANEL")
        .color(Colour::RED)
        .description(formatdoc!(
            "
                {} messages will be deleted from {} in **{} seconds**...

                To stop this nuke, press the red button below!
            ",
            count,
            channel.mention(),
            COUNTDOWN_START_SECS
        ));
    let components = vec![CreateActionRow::Buttons(vec![cancel_button])];
    let reply = CreateReply::new()
        .embed(embed.clone())
        .components(components.clone());

    let reply_handler = ctx.send(reply.clone()).await?;
    let reply_handler_countdown = reply_handler.clone();
    let countdown = async move {
        let reply_handler = reply_handler_countdown;
        let embed = embed;

        let mut ticker = tokio::time::interval_at(
            Instant::now() + Duration::from_secs(1),
            Duration::from_secs(1),
        );
        ticker.tick().await;
        for i in (1..COUNTDOWN_START_SECS).rev() {
            let updated_description = formatdoc!(
                "
                {} messages will be deleted from {} in **{} second{}**...

                To stop this nuke, press the red button below!
            ",
                count,
                channel.mention(),
                i,
                if i > 1 { "s" } else { "" }
            );
            reply_handler
                .edit(
                    ctx,
                    CreateReply::new().embed(embed.clone().description(updated_description)),
                )
                .await?;
            ticker.tick().await;
        }

        Ok::<(), Error>(())
    };

    let button_interaction = reply_handler
        .message()
        .await?
        .await_component_interaction(ctx)
        .into_future();
    match select(pin!(countdown), button_interaction).await {
        Either::Left((countdown, _)) => {
            countdown?;
        }
        Either::Right((component_interaction, _)) => {
            if let Some(component_interaction) = component_interaction
                && matches!(
                    component_interaction.data.kind,
                    ComponentInteractionDataKind::Button
                )
                && component_interaction.data.custom_id == NUKE_CANCEL_BUTTON_ID
            {
                component_interaction
                    .create_response(
                        ctx.http(),
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::new()
                                .content("Message nuke cancelled")
                                .embeds(vec![])
                                .components(vec![]),
                        ),
                    )
                    .await?;
            }
            return Ok(());
        }
    }

    let reply = CreateReply::new()
        .content(format!(
            "Now nuking {} messages from the channel ...",
            count
        ))
        .components(vec![]);
    reply_handler.edit(ctx, reply).await?;

    for message in messages_to_delete {
        message.delete(ctx.http()).await?;
    }

    let reply = CreateReply::new()
        .content(format!("Nuked {} messages.", count))
        .components(vec![]);
    reply_handler.edit(ctx, reply).await?;
    Ok(())
}
