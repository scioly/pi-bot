use std::ops::Deref;

use poise::CreateReply;
use poise::serenity_prelude::{EditRole, Guild};
use sqlx::{Executor, MySql};

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
#[poise::command(slash_command, subcommands("batch_role"))]
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

    if check_event_exist(&mut *tx, &event_name).await? {
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
pub async fn role(
    ctx: Context<'_>,
    #[description = "Whether to enable or disable the event role."] mode: EnableDisable,
    #[description = "The name of the event."] event_name: String,
) -> Result<(), Error> {
    let guild = ctx.guild().unwrap().deref().clone();

    if !check_event_exist(&ctx.data().db, &event_name).await? {
        ctx.reply(format!("`{}` is not an event!", event_name))
            .await?;
        return Ok(());
    }

    match mode {
        EnableDisable::Enable => {
            let reply_message = ctx
                .reply(format!(
                    "{} Attempting to enable role for `{}`",
                    EMOJI_LOADING, event_name
                ))
                .await?;
            let status = enable_event_role(&ctx, &guild, &event_name).await?;
            reply_message
                .edit(
                    ctx,
                    CreateReply::default().content(role_enable_message(&status, &event_name)),
                )
                .await?;
        }
        EnableDisable::Disable => {
            let reply_message = ctx
                .reply(format!(
                    "{} Attempting to disable role for `{}`",
                    EMOJI_LOADING, event_name
                ))
                .await?;
            let status = disable_event_role(&ctx, &guild, &event_name).await?;
            reply_message
                .edit(
                    ctx,
                    CreateReply::default().content(role_disable_message(&status, &event_name)),
                )
                .await?;
        }
    }
    Ok(())
}

/// Add or remove multiple events' role.
#[poise::command(slash_command, rename = "role", check = "is_staff")]
pub async fn batch_role(
    ctx: Context<'_>,
    #[description = "Whether to enable or disable the event roles."] mode: EnableDisable,
    #[description = "A comma separated list of all event roles to add. Format as 'event name 1,event name 2'."]
    event_name_csv_list: String,
) -> Result<(), Error> {
    let guild = ctx.guild().unwrap().deref().clone();

    ctx.defer().await?;
    let event_list = event_name_csv_list.split(',');

    batch_process_roles(&ctx, &guild, &mode, event_list).await?;
    Ok(())
}

async fn batch_process_roles(
    ctx: &Context<'_>,
    guild: &Guild,
    mode: &EnableDisable,
    event_names: impl Iterator<Item = &str>,
) -> Result<(), Error> {
    let mut responses = vec![];
    let mut tx = ctx.data().db.begin().await?;
    for event_name in event_names {
        if !check_event_exist(&mut *tx, event_name).await? {
            responses.push(format!("`{}` is not an event!", event_name));
            continue;
        }
        match mode {
            EnableDisable::Enable => {
                let result = enable_event_role(ctx, guild, event_name).await;
                let message = match result {
                    Ok(status) => role_enable_message(&status, event_name),
                    Err(err) => format!("{}", err),
                };
                responses.push(message);
            }
            EnableDisable::Disable => {
                let result = disable_event_role(ctx, guild, event_name).await;
                let message = match result {
                    Ok(status) => role_disable_message(&status, event_name),
                    Err(err) => format!("{}", err),
                };
                responses.push(message);
            }
        }
    }
    Ok(())
}

async fn check_event_exist<'c>(
    conn: impl Executor<'c, Database = MySql>,
    event_name: &str,
) -> Result<bool, sqlx::Error> {
    let event_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM event WHERE name = ?",
        event_name
    )
    .fetch_one(conn)
    .await?;

    Ok(event_count.count != 0)
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

fn role_enable_message(status: &EnableRoleResponse, event_name: &str) -> String {
    match status {
        EnableRoleResponse::RoleCreated => format!("Role for `{}` has been added.", event_name),
        EnableRoleResponse::RoleAlreadyEnabled => {
            format!("Role for `{}` has already been added.", event_name)
        }
    }
}

fn role_disable_message(status: &DisableRoleResponse, event_name: &str) -> String {
    match status {
        DisableRoleResponse::RoleRemoved => format!("Role for `{}` has been deleted.", event_name),
        DisableRoleResponse::RoleAlreadyDisabled => {
            format!("Role for `{}` has already been deleted.", event_name)
        }
    }
}
