use std::ops::Deref;

use poise::CreateReply;
use poise::serenity_prelude::{EditRole, Guild};

use crate::discord::utils::{EMOJI_LOADING, is_staff};
use crate::discord::{Context, Error};

#[derive(Debug)]
enum EnableRoleResponse {
    RoleCreated,
    RoleAlreadyEnabled,
}

#[derive(Debug)]
enum DisableRoleResponse {
    RoleRemoved,
    RoleAlreadyDisabled,
}

#[derive(Debug, poise::ChoiceParameter)]
enum EnableDisable {
    #[name = "Enable"]
    Enable,
    #[name = "Disable"]
    Disable,
}

/// Updates the bot's list of events.
#[poise::command(
    slash_command,
    subcommands("batch", "add", "role"),
    default_member_permissions = "MANAGE_ROLES",
    required_bot_permissions = "MANAGE_ROLES"
)]
pub async fn event(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Batch operation commands that update the bot's list of events.
#[poise::command(slash_command, /* TODO: subcommands() */)]
pub async fn batch(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Staff command. Adds a new event.
#[poise::command(slash_command, guild_only, check = "is_staff")]
pub async fn add(
    ctx: Context<'_>,
    #[description = "The name of the new event."] event_name: String,
    // #[description = "The aliases for the new event. Format as 'alias1, alias2'."]
    // event_aliases: Option<String>,
    #[description = "Whether event role should be enabled immediately. Default is True."]
    should_enable_role: Option<bool>,
) -> Result<(), Error> {
    let should_enable_role = should_enable_role.unwrap_or(true);
    let guild = ctx.guild().unwrap().deref().clone();
    let reply_message = ctx
        .reply(format!(
            "{} Attempting to add `{}` as a new event...",
            EMOJI_LOADING, event_name
        ))
        .await?;

    let mut tx = ctx.data().db.begin().await?;
    let event_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM event WHERE name = ?",
        event_name
    )
    .fetch_one(&mut *tx)
    .await?;

    if event_count.count != 0 {
        // reply_message
        //     .edit(
        //         ctx,
        //         CreateReply::default().content(format!(
        //             "The `{}` event has already been added.",
        //             event_name
        //         )),
        //     )
        //     .await?;
        if should_enable_role {
            match enable_event_role(&ctx, &guild, &event_name).await? {
                EnableRoleResponse::RoleCreated => {
                    reply_message.edit(ctx, CreateReply::default().content(format!("The event `{}` has already been added to the database. Enabled the event role since it was not active.", event_name))).await?;
                }
                EnableRoleResponse::RoleAlreadyEnabled => {
                    reply_message.edit(ctx, CreateReply::default().content(format!("The event `{}` has already been added to the database, and the event role was already enabled. No action was taken.", event_name))).await?;
                }
            }
        } else {
            reply_message
                .edit(
                    ctx,
                    CreateReply::default().content(format!(
                        "The event `{}` has already been added. No action was taken.",
                        event_name
                    )),
                )
                .await?;
        }
        return Ok(());
    }

    if guild.role_by_name(&event_name).is_some() {
        reply_message
            .edit(
                ctx,
                CreateReply::default().content(format!(
                    "A role with the name `{}` already exists. The event cannot be added until that role has been manually deleted.",
                    event_name
                )),
            )
            .await?;
        return Ok(());
    }

    sqlx::query!("INSERT INTO event (name) VALUES (?)", event_name)
        .execute(&mut *tx)
        .await?;

    if should_enable_role {
        guild
            .create_role(ctx.http(), EditRole::new().name(&event_name))
            .await?;
        reply_message.edit(ctx, CreateReply::default().content(format!("The event `{}` has been added to the database, and the event role was created.", event_name))).await?;
    } else {
        reply_message
            .edit(
                ctx,
                CreateReply::default()
                    .content(format!("The `{}` event has been added. The event role can be enabled with `/{} enable.", event_name, role().qualified_name)),
            )
            .await?;
    }
    Ok(())
}

/// Add or remove an event's role.
#[poise::command(slash_command, check = "is_staff")]
pub async fn role(ctx: Context<'_>, mode: EnableDisable, event_name: String) -> Result<(), Error> {
    let guild = ctx.guild().unwrap().deref().clone();
    match mode {
        EnableDisable::Enable => {
            let reply_message = ctx
                .reply(format!(
                    "{} Attempting to enable role for `{}`",
                    EMOJI_LOADING, event_name
                ))
                .await?;
            match enable_event_role(&ctx, &guild, &event_name).await? {
                EnableRoleResponse::RoleCreated => {
                    reply_message
                        .edit(
                            ctx,
                            CreateReply::default()
                                .content(format!("Role for `{}` has been added.", event_name)),
                        )
                        .await?;
                }
                EnableRoleResponse::RoleAlreadyEnabled => {
                    reply_message
                        .edit(
                            ctx,
                            CreateReply::default().content(format!(
                                "Role for `{}` has already been added.",
                                event_name
                            )),
                        )
                        .await?;
                }
            }
        }
        EnableDisable::Disable => {
            let reply_message = ctx
                .reply(format!(
                    "{} Attempting to disable role for `{}`",
                    EMOJI_LOADING, event_name
                ))
                .await?;
            match disable_event_role(&ctx, &guild, &event_name).await? {
                DisableRoleResponse::RoleRemoved => {
                    reply_message
                        .edit(
                            ctx,
                            CreateReply::default()
                                .content(format!("Role for `{}` has been deleted.", event_name)),
                        )
                        .await?;
                }
                DisableRoleResponse::RoleAlreadyDisabled => {
                    reply_message
                        .edit(
                            ctx,
                            CreateReply::default().content(format!(
                                "Role for `{}` has already been deleted.",
                                event_name
                            )),
                        )
                        .await?;
                }
            }
        }
    }
    Ok(())
}

async fn enable_event_role(
    ctx: &Context<'_>,
    guild: &Guild,
    event_name: &str,
) -> Result<EnableRoleResponse, Error> {
    if guild.role_by_name(event_name).is_some() {
        Ok(EnableRoleResponse::RoleAlreadyEnabled)
    } else {
        guild
            .create_role(ctx.http(), EditRole::new().name(event_name))
            .await?;
        Ok(EnableRoleResponse::RoleCreated)
    }
}

async fn disable_event_role(
    ctx: &Context<'_>,
    guild: &Guild,
    event_name: &str,
) -> Result<DisableRoleResponse, Error> {
    if let Some(role) = guild.role_by_name(event_name) {
        guild.delete_role(ctx.http(), role.id).await?;
        Ok(DisableRoleResponse::RoleRemoved)
    } else {
        Ok(DisableRoleResponse::RoleAlreadyDisabled)
    }
}
