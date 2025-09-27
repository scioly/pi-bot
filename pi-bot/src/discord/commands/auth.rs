use std::ops::DerefMut;

use poise::{
    CreateReply,
    serenity_prelude::{CreateEmbed, UserId},
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
