use indoc::formatdoc;
use poise::{
    ChoiceParameter, CreateReply,
    serenity_prelude::{
        ButtonStyle, ChannelType, Colour, ComponentInteractionDataKind, CreateActionRow,
        CreateButton, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
        EditChannel, FormattedTimestamp, FormattedTimestampStyle, GetMessages, GuildChannel,
        Member, Mentionable, PermissionOverwrite, PermissionOverwriteType, Permissions, Timestamp,
        futures::future::{Either, select},
    },
};
use std::{pin::pin, time::Duration};
use tokio::time::Instant;

use crate::discord::{
    Context, Error,
    utils::{
        CATEGORY_BETA, CATEGORY_COMMUNITY, CATEGORY_STAFF, EMOJI_LOADING, ROLE_BOTS, ROLE_EVERYONE,
        ROLE_MUTED, ROLE_STAFF, ROLE_VIP,
    },
};

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

#[derive(Debug, ChoiceParameter)]
enum MuteLength {
    #[name = "10 minutes"]
    Mins10,
    #[name = "20 minutes"]
    Mins20,
    #[name = "1 hour"]
    Hours1,
    #[name = "2 hours"]
    Hours2,
    #[name = "8 hours"]
    Hours8,
    #[name = "1 day"]
    Days1,
    #[name = "4 days"]
    Days4,
    #[name = "7 days"]
    Days7,
    #[name = "4 weeks"]
    Weeks4,
    // #[name = "1 month"]
    // Month1,
    // #[name = "1 year"]
    // Year1,
    #[name = "Indefinitely"]
    Indefinitely,
}

#[derive(Debug, ChoiceParameter)]
enum YesNo {
    Yes,
    No,
}

// TODO: add view to confirm mute
// TODO: use `reason` and `quiet`
/// Staff command. Mutes a user.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MANAGE_ROLES",
    required_bot_permissions = "MANAGE_ROLES"
)]
pub async fn mute(
    ctx: Context<'_>,
    #[description = "The user to mute."] mut member: Member,
    #[description = "The reason to mute the user."] _reason: Option<String>,
    #[description = "How long to mute the user for. Mutally exclusive to `until`."]
    mute_length: Option<MuteLength>,
    #[description = "Unix timestamp (in secs) for how long to keep user muted until. Mutally exclusive to `mute_length`."]
    until: Option<i64>,
    #[description = "Does not DM the user upon mute. Defaults to no."] _quiet: Option<YesNo>,
) -> Result<(), Error> {
    let guild = ctx.guild_id().unwrap();
    let now = Timestamp::now();
    let end_time =
        match (mute_length, until) {
            (Some(_), Some(_)) => return Err(
                "Provided `mute_length` and `until` options when only one of them should be sent"
                    .into(),
            ),
            (None, None) => return Err("One of `mute_length` and `until` should be sent".into()),
            (Some(duration_name), None) => {
                let duration = match duration_name {
                    MuteLength::Mins10 => Some(Duration::from_mins(10).as_secs()),
                    MuteLength::Mins20 => Some(Duration::from_mins(20).as_secs()),
                    MuteLength::Hours1 => Some(Duration::from_hours(1).as_secs()),
                    MuteLength::Hours2 => Some(Duration::from_hours(2).as_secs()),
                    MuteLength::Hours8 => Some(Duration::from_hours(8).as_secs()),
                    MuteLength::Days1 => Some(Duration::from_hours(24).as_secs()),
                    MuteLength::Days4 => Some(Duration::from_hours(4 * 24).as_secs()),
                    MuteLength::Days7 => Some(Duration::from_hours(7 * 24).as_secs()),
                    MuteLength::Weeks4 => Some(Duration::from_hours(28 * 24).as_secs()),
                    MuteLength::Indefinitely => None,
                };
                if let Some(duration) = duration {
                    Some(Timestamp::from_unix_timestamp(
                        now.unix_timestamp() + duration as i64,
                    )?)
                } else {
                    None
                }
            }
            (None, Some(until)) => Some(Timestamp::from_unix_timestamp(until)?),
        };
    if let Some(end_time) = end_time {
        member
            .disable_communication_until_datetime(ctx.http(), end_time)
            .await?;
        ctx.reply(format!(
            "{} was muted until {}.",
            member.mention(),
            FormattedTimestamp::new(end_time, Some(FormattedTimestampStyle::ShortDateTime))
        ))
        .await?;
    } else {
        let roles = guild.roles(ctx.http()).await?;
        let mute_role = roles
            .values()
            .find(|role| role.name == ROLE_MUTED)
            .ok_or(format!("Could not find `{}` role", ROLE_MUTED))?;
        member.add_role(ctx.http(), mute_role.id).await?;
        ctx.reply(format!("{} was muted indefinitely.", member.mention()))
            .await?;
    }
    Ok(())
}

/// Staff command. Locks a channel, preventing members from sending messages.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MANAGE_CHANNELS",
    required_bot_permissions = "MANAGE_CHANNELS"
)]
pub async fn lock(ctx: Context<'_>) -> Result<(), Error> {
    let reply_handler = ctx
        .reply(format!("{} Attempting to lock channel ...", EMOJI_LOADING))
        .await?;

    let mut channel = ctx.guild_channel().await.unwrap();
    let category = channel.parent_id;
    let is_priviledged_channel = match category {
        None => false,
        Some(category_id) => {
            let category = category_id
                .to_channel(ctx.http())
                .await
                .map(|channel| channel.guild())
                .unwrap();
            category.is_some_and(|category| {
                matches!(category.kind, ChannelType::Category)
                    && [CATEGORY_BETA, CATEGORY_STAFF, CATEGORY_COMMUNITY]
                        .contains(&category.name.as_str())
            })
        }
    };
    if is_priviledged_channel {
        reply_handler
            .edit(
                ctx,
                CreateReply::new().content(
                    "This command is not suitable for this channel because of its category.",
                ),
            )
            .await?;
        return Ok(());
    }

    let roles = channel.guild_id.roles(ctx.http()).await?;
    let everyone_role = roles
        .values()
        .find(|role| role.name == ROLE_EVERYONE)
        .ok_or(format!("Could not find `{}` role", ROLE_EVERYONE))?;
    let staff_role = roles
        .values()
        .find(|role| role.name == ROLE_STAFF)
        .ok_or(format!("Could not find `@{}` role", ROLE_STAFF))?;
    let vip_role = roles
        .values()
        .find(|role| role.name == ROLE_VIP)
        .ok_or(format!("Could not find `@{}` role", ROLE_VIP))?;
    let bot_role = roles
        .values()
        .find(|role| role.name == ROLE_BOTS)
        .ok_or(format!("Could not find `@{}` role", ROLE_BOTS))?;
    channel
        .edit(
            ctx.http(),
            EditChannel::new().permissions([
                PermissionOverwrite {
                    allow: Permissions::READ_MESSAGE_HISTORY,
                    deny: Permissions::ADD_REACTIONS | Permissions::SEND_MESSAGES,
                    kind: PermissionOverwriteType::Role(everyone_role.id),
                },
                PermissionOverwrite {
                    allow: Permissions::ADD_REACTIONS
                        | Permissions::SEND_MESSAGES
                        | Permissions::READ_MESSAGE_HISTORY,
                    deny: Permissions::empty(),
                    kind: PermissionOverwriteType::Role(staff_role.id),
                },
                PermissionOverwrite {
                    allow: Permissions::ADD_REACTIONS
                        | Permissions::SEND_MESSAGES
                        | Permissions::READ_MESSAGE_HISTORY,
                    deny: Permissions::empty(),
                    kind: PermissionOverwriteType::Role(vip_role.id),
                },
                PermissionOverwrite {
                    allow: Permissions::ADD_REACTIONS
                        | Permissions::SEND_MESSAGES
                        | Permissions::READ_MESSAGE_HISTORY,
                    deny: Permissions::empty(),
                    kind: PermissionOverwriteType::Role(bot_role.id),
                },
            ]),
        )
        .await?;

    reply_handler
        .edit(
            ctx,
            CreateReply::new().content("Locked the channel to public access."),
        )
        .await?;
    Ok(())
}

/// Staff command. Unlocks a channel, allowing members to speak after the channel was originally locked.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MANAGE_CHANNELS",
    required_bot_permissions = "MANAGE_CHANNELS"
)]
pub async fn unlock(ctx: Context<'_>) -> Result<(), Error> {
    let reply_handler = ctx
        .reply(format!(
            "{} Attempting to unlock channel ...",
            EMOJI_LOADING
        ))
        .await?;

    let mut channel = ctx.guild_channel().await.unwrap();
    let category = match channel.parent_id {
        None => None,
        Some(category_id) => {
            let category = category_id
                .to_channel(ctx.http())
                .await
                .map(|channel| channel.guild())
                .unwrap();
            category.and_then(|category| match category.kind {
                ChannelType::Category => Some(category),
                _ => None,
            })
        }
    };
    let roles = channel.guild_id.roles(ctx.http()).await?;
    let everyone_role = roles
        .values()
        .find(|role| role.name == ROLE_EVERYONE)
        .ok_or(format!("Could not find `{}` role", ROLE_EVERYONE))?;

    if let Some(category) = category
        && [CATEGORY_BETA, CATEGORY_STAFF, CATEGORY_COMMUNITY].contains(&category.name.as_str())
    {
        reply_handler
            .edit(
                ctx,
                CreateReply::new().content(
                    "This command is not suitable for this channel because of its category.",
                ),
            )
            .await?;
        return Ok(());
    }
    channel
        .edit(
            ctx.http(),
            EditChannel::new().permissions([PermissionOverwrite {
                allow: Permissions::empty(),
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Role(everyone_role.id),
            }]),
        )
        .await?;
    reply_handler.edit(ctx,CreateReply::new().
        content("Unlocked the channel to public access. Please check if permissions need to be synced.")
    ).await?;
    Ok(())
}
