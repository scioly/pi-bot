use std::ops::DerefMut;

use poise::{
    CreateReply,
    serenity_prelude::{
        CreateAllowedMentions, CreateEmbed, CreateEmbedFooter, Member, Mentionable, Timestamp,
        UserId,
    },
};
use sqlx::{MySql, Transaction};

use crate::discord::{Context, Error};

struct AuthRequest {
    state_code: String,
    has_expired: u64,
}

async fn insert_auth_request_code(
    tx: &mut Transaction<'_, MySql>,
    user_id: UserId,
) -> Result<String, sqlx::Error> {
    let insert_result= sqlx::query!(
                "INSERT INTO authorization_request (discord_user_id, state_code) VALUES (?, RANDOM_BYTES(255))",
                user_id.get()
            ).execute(tx.deref_mut()).await?;
    Ok(sqlx::query!(
        r#"SELECT HEX(state_code) AS "state_code!" FROM authorization_request WHERE id = ?"#,
        insert_result.last_insert_id()
    )
    .fetch_one(tx.deref_mut())
    .await?
    .state_code)
}

#[poise::command(slash_command)]
pub async fn auth(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id;

    let db = &ctx.data().db;
    let mut tx = db.begin().await?;
    let state = sqlx::query_as!(
        AuthRequest,
        r#"SELECT HEX(state_code) AS "state_code!", CAST(NOW() >= expires_at AS UNSIGNED INT) AS has_expired FROM authorization_request WHERE discord_user_id = ?"#,
        user_id.get(),
    ).fetch_optional(&mut *tx).await?;

    let code = match state {
        Some(state) => {
            if state.has_expired != 0 {
                sqlx::query!(
                    r#"DELETE FROM authorization_request WHERE discord_user_id = ?"#,
                    user_id.get()
                )
                .execute(&mut *tx)
                .await?;
                insert_auth_request_code(&mut tx, user_id).await?
            } else {
                state.state_code
            }
        }
        None => insert_auth_request_code(&mut tx, user_id).await?,
    };

    tx.commit().await?;

    let client_id = "abcdef1234567890";
    let url = format!(
        "http://192.168.42.192/oauth/authorize?response_type=code&state={}&client_id={}",
        code, client_id
    );

    let embed = CreateEmbed::new()
        .title("Connecting your Scioly.org Account")
        .description(format!(
            "To link your Scioly.org account to your Discord account, please follow this link: [Authenticate on Scioly.org]({})",
            url
        ));
    let message = CreateReply::new().embed(embed).ephemeral(true);
    ctx.send(message).await?;
    Ok(())
}

#[derive(Debug)]
struct UserStats {
    phpbb_user_id: u32,
    phpbb_username: String,
    forums_avatar: Option<String>,
    forums_post_count: u32,
    forums_thanks_received: u32,
    forums_thanks_given: u32,
    wiki_edit_count: u32,
    updated_at: Timestamp,
}

#[poise::command(
    slash_command,
    required_permissions = "MODERATE_MEMBERS | KICK_MEMBERS | BAN_MEMBERS"
)]
pub async fn whois(ctx: Context<'_>, member: Member) -> Result<(), Error> {
    let mut tx = ctx.data().db.begin().await?;
    let stats = sqlx::query_as!(
        UserStats,
        "SELECT
            phpbb_user_id,
            username as phpbb_username,
            forums_avatar,
            forums_post_count,
            forums_thanks_received,
            forums_thanks_given,
            wiki_edit_count,
            updated_at
        FROM scioly_user_stats WHERE discord_user_id = ?",
        member.user.id.get()
    )
    .fetch_one(&mut *tx)
    .await?;

    // TODO: If updated_at is 1-5 min out of date, then refetch whoami data.

    let mut embed = CreateEmbed::new()
        .title(format!("User Stats for {}", member.user.display_name()))
        .field(
            "Discord Username",
            format!("{} `{}`", member.user.mention(), member.user.name),
            true,
        )
        .field("Discord ID", member.user.id.to_string(), true)
        .field("\u{200B}", "\u{200B}", true)
        .field("Scioly.org Username", stats.phpbb_username, true)
        .field("Scioly.org User ID", stats.phpbb_user_id.to_string(), true)
        .field("\u{200B}", "\u{200B}", true)
        .field(
            "Forums Post Count",
            stats.forums_post_count.to_string(),
            false,
        )
        .field(
            "Forums Thanks Given",
            stats.forums_thanks_given.to_string(),
            true,
        )
        .field(
            "Forums Post Count",
            stats.forums_thanks_received.to_string(),
            true,
        )
        .field("Wiki Edit Count", stats.wiki_edit_count.to_string(), true)
        .footer(CreateEmbedFooter::new("Last updated"))
        .timestamp(stats.updated_at);
    if let Some(url) = stats.forums_avatar {
        embed = embed.image(url);
    }
    ctx.send(
        CreateReply::default()
            .embed(embed)
            .allowed_mentions(CreateAllowedMentions::default().empty_users().empty_roles()),
    )
    .await?;
    Ok(())
}
