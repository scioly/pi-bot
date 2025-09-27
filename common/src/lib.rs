use std::{collections::HashMap, ops::DerefMut};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::{Executor, MySql, Transaction};
use thiserror::Error;

pub const OAUTH_HOST_URL: &str = "https://scioly.org";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessTokenResponse {
    #[serde(rename = "phpBBUserId")]
    pub phpbb_user_id: i32,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumsSubResponse {
    pub user_avatar: Option<String>,
    pub post_count: u32,
    pub thanks_received: u32,
    pub thanks_given: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiSubResponse {
    pub edit_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestExchangeSubResponse {
    pub upload_count: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GallerySubResponse {
    pub score: Option<u32>,
    pub post_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhoAmIResponse {
    #[serde(rename = "userId")]
    pub phpbb_user_id: u32,
    pub username: String,
    pub forums: ForumsSubResponse,
    pub wiki: WikiSubResponse,
    pub test_exchange: TestExchangeSubResponse,
    pub gallery: GallerySubResponse,
}

#[derive(Debug, Error)]
pub enum WhoAmiError {
    #[error("reqwest error")]
    ReqwestError(reqwest::Error),
    #[error("incorrect status code (expected {expected:?}, found {found:?})")]
    StatusCodeError {
        expected: reqwest::StatusCode,
        found: reqwest::StatusCode,
    },
}

impl From<reqwest::Error> for WhoAmiError {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError(value)
    }
}

#[derive(Debug, Error)]
pub enum AccessTokenError {
    #[error("reqwest error")]
    ReqwestError(reqwest::Error),
    #[error("incorrect status code (expected {expected:?}, found {found:?})")]
    StatusCodeError {
        expected: reqwest::StatusCode,
        found: reqwest::StatusCode,
    },
}

impl From<reqwest::Error> for AccessTokenError {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError(value)
    }
}

pub async fn refresh_access_token(
    client: &Client,
    refresh_token: &str,
    oauth_client_id: &str,
    oauth_client_secret: &str,
) -> Result<AccessTokenResponse, AccessTokenError> {
    let formdata = HashMap::from([
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", oauth_client_id),
        ("client_secret", oauth_client_secret),
    ]);
    let oauth_res = client
        .post(format!("{}/oauth/access_token/", OAUTH_HOST_URL))
        .form(&formdata)
        .send()
        .await?;

    const EXPECTED_STATUS_CODE: reqwest::StatusCode = reqwest::StatusCode::OK;
    let response_status_code = oauth_res.status();
    if response_status_code != EXPECTED_STATUS_CODE {
        return Err(AccessTokenError::StatusCodeError {
            expected: EXPECTED_STATUS_CODE,
            found: response_status_code,
        });
    }

    let body_res = oauth_res.json::<AccessTokenResponse>().await?;
    Ok(body_res)
}

pub async fn fetch_new_token(
    client: &Client,
    authorization_code: &str,
    oauth_client_id: &str,
    oauth_client_secret: &str,
) -> Result<AccessTokenResponse, AccessTokenError> {
    let formdata = HashMap::from([
        ("grant_type", "authorization_code"),
        ("code", authorization_code),
        ("client_id", oauth_client_id),
        ("client_secret", oauth_client_secret),
    ]);
    let oauth_res = client
        .post(format!("{}/oauth/access_token/", OAUTH_HOST_URL))
        .form(&formdata)
        .send()
        .await?;

    const EXPECTED_STATUS_CODE: reqwest::StatusCode = reqwest::StatusCode::OK;
    let response_status_code = oauth_res.status();
    if response_status_code != EXPECTED_STATUS_CODE {
        return Err(AccessTokenError::StatusCodeError {
            expected: EXPECTED_STATUS_CODE,
            found: response_status_code,
        });
    }

    let body_res = oauth_res.json::<AccessTokenResponse>().await?;
    Ok(body_res)
}

pub async fn update_db_access_token<'c>(
    tx: &mut Transaction<'c, MySql>,
    discord_user_id: u64,
    body_res: &AccessTokenResponse,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM scioly_tokens WHERE discord_user_id = ?",
        discord_user_id,
    )
    .execute(tx.deref_mut())
    .await?;
    sqlx::query!(
        "INSERT INTO scioly_tokens (discord_user_id, phpbb_user_id, access_token, refresh_token, access_expires_at) VALUES (?, ?, UNHEX(?), UNHEX(?), FROM_UNIXTIME(?))",
        discord_user_id,
        body_res.phpbb_user_id,
        body_res.access_token,
        body_res.refresh_token,
        body_res.expires_at
    ).execute(tx.deref_mut()).await?;
    Ok(())
}
pub async fn fetch_whoami(
    client: &Client,
    authorization_access_token: &str,
) -> Result<WhoAmIResponse, WhoAmiError> {
    let whoami_res = client
        .get(format!("{}/oauth/api/whoami/", OAUTH_HOST_URL))
        .header(
            "Authorization",
            format!("Bearer {}", authorization_access_token),
        )
        .send()
        .await?;
    const EXPECTED_STATUS_CODE: reqwest::StatusCode = reqwest::StatusCode::OK;
    let response_status_code = whoami_res.status();
    if response_status_code != EXPECTED_STATUS_CODE {
        return Err(WhoAmiError::StatusCodeError {
            expected: EXPECTED_STATUS_CODE,
            found: response_status_code,
        });
    }
    let whoami = whoami_res.json::<WhoAmIResponse>().await?;
    Ok(whoami)
}

pub async fn update_db_user_stats<'c>(
    conn: impl Executor<'c, Database = MySql>,
    whoami: &WhoAmIResponse,
    discord_user_id: u64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO scioly_user_stats (
            discord_user_id,
            phpbb_user_id,
            username,
            forums_avatar,
            forums_post_count,
            forums_thanks_received,
            forums_thanks_given,
            wiki_edit_count,
            test_ex_upload_count,
            gallery_score,
            gallery_post_count) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
            discord_user_id = ?,
            phpbb_user_id = ?,
            username = ?,
            forums_avatar = ?,
            forums_post_count = ?,
            forums_thanks_received = ?,
            forums_thanks_given = ?,
            wiki_edit_count = ?,
            test_ex_upload_count = ?,
            gallery_score = ?,
            gallery_post_count = ?",
        // on insert
        discord_user_id,
        whoami.phpbb_user_id,
        whoami.username,
        whoami.forums.user_avatar,
        whoami.forums.post_count,
        whoami.forums.thanks_received,
        whoami.forums.thanks_given,
        whoami.wiki.edit_count,
        whoami.test_exchange.upload_count.unwrap_or(0),
        whoami.gallery.score.unwrap_or(0),
        whoami.gallery.post_count,
        // on update
        discord_user_id,
        whoami.phpbb_user_id,
        whoami.username,
        whoami.forums.user_avatar,
        whoami.forums.post_count,
        whoami.forums.thanks_received,
        whoami.forums.thanks_given,
        whoami.wiki.edit_count,
        whoami.test_exchange.upload_count.unwrap_or(0),
        whoami.gallery.score.unwrap_or(0),
        whoami.gallery.post_count,
    )
    .execute(conn)
    .await?;
    Ok(())
}
