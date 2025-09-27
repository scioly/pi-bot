use log::info;
use std::collections::HashMap;

use actix_web::{
    App, HttpResponse, HttpServer,
    body::BoxBody,
    error::{ErrorInternalServerError, ErrorNotFound},
    get,
    middleware::Logger,
    web,
};

use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

#[derive(Debug, Clone, Deserialize)]
struct Env {
    database_url: String,
}

const OAUTH_HOST_URL: &str = "http://192.168.42.192";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    dotenv().ok();

    let env_config = envy::from_env::<Env>().expect("should parse into expected config struct");

    let pool = MySqlPool::connect(&env_config.database_url)
        .await
        .expect("should construct new database pool");
    HttpServer::new(move || {
        let logger = Logger::default();
        App::new()
            .app_data(web::Data::new(ServerState { db: pool.clone() }))
            .wrap(logger)
            .service(authorize)
            .route("/", web::get().to(HttpResponse::Ok))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

#[derive(Debug)]
struct ServerState {
    pub db: MySqlPool,
}

#[derive(Debug, Deserialize)]
struct AuthorizeInputs {
    #[serde(rename = "state")]
    pub state_hex: String,
    #[serde(rename = "code")]
    pub code_hex: String,
}

#[derive(Debug, Deserialize)]
struct AuthRow {
    id: u32,
    discord_user_id: u64,
    has_expired: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccessTokenResponse {
    #[serde(rename = "phpBBUserId")]
    phpbb_user_id: i32,
    access_token: String,
    refresh_token: String,
    expires_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForumsSubResponse {
    user_avatar: Option<String>,
    post_count: u32,
    thanks_received: u32,
    thanks_given: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WikiSubResponse {
    edit_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestExchangeSubResponse {
    upload_count: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GallerySubResponse {
    score: Option<u32>,
    post_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WhoAmIResponse {
    #[serde(rename = "userId")]
    phpbb_user_id: i32,
    username: String,
    forums: ForumsSubResponse,
    wiki: WikiSubResponse,
    test_exchange: TestExchangeSubResponse,
    gallery: GallerySubResponse,
}

#[get("/authorize")]
async fn authorize(
    data: web::Data<ServerState>,
    query: web::Query<AuthorizeInputs>,
) -> actix_web::Result<HttpResponse> {
    let mut tx = data.db.begin().await.map_err(|err| {
        info!("{}", err);
        ErrorInternalServerError(err)
    })?;

    let row = match sqlx::query_as!(
        AuthRow,
        "SELECT id, discord_user_id, NOW() >= expires_at AS has_expired FROM authorization_request WHERE state_code = UNHEX(?)",
        query.state_hex
    ).fetch_one(&mut *tx).await {
        Err(sqlx::Error::RowNotFound) => { return Err(ErrorNotFound(sqlx::Error::RowNotFound))}
        Err(e) => {
            info!("{}", e);
            return Err(ErrorInternalServerError(e));
        }
        Ok(row) => row,
    };

    let delete_query = sqlx::query!("DELETE FROM authorization_request WHERE id = ?", row.id);
    if row.has_expired != 0 {
        delete_query.execute(&mut *tx).await.map_err(|err| {
            info!("{}", err);
            ErrorInternalServerError(err)
        })?;
        tx.commit().await.map_err(|err| {
            info!("{}", err);
            ErrorInternalServerError(err)
        })?;
        return Ok(HttpResponse::new(actix_web::http::StatusCode::NOT_FOUND));
    }

    let client = reqwest::Client::new();
    let formdata = HashMap::from([
        ("grant_type", "authorization_code".into()),
        ("code", query.code_hex.clone()),
        // TODO: turn these into env vars
        ("client_id", "abcdef1234567890".into()),
        ("client_secret", "abcdef1234567890".into()),
    ]);
    let oauth_res = client
        .post(format!("{}/oauth/access_token/", OAUTH_HOST_URL))
        .form(&formdata)
        .send()
        .await
        .map_err(|err| {
            info!("{}/oauth/access_token/: {}", OAUTH_HOST_URL, err);
            ErrorInternalServerError(err)
        })?;
    let status_code = oauth_res.status().as_u16();
    let actix_status_code = actix_web::http::StatusCode::from_u16(status_code) // disgusting
        .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
    match actix_status_code {
        actix_web::http::StatusCode::OK => {}
        code if code == actix_web::http::StatusCode::BAD_REQUEST.as_u16()
            || code == actix_web::http::StatusCode::INTERNAL_SERVER_ERROR.as_u16()
            || code == actix_web::http::StatusCode::NOT_FOUND.as_u16() =>
        {
            return Ok(HttpResponse::new(code));
        }
        _ => {
            return Ok(HttpResponse::new(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    }

    let body_res = oauth_res
        .json::<AccessTokenResponse>()
        .await
        .map_err(|err| {
            info!("{}/oauth/access_token/{}", OAUTH_HOST_URL, err);
            ErrorInternalServerError(err)
        })?;

    sqlx::query!(
        "DELETE FROM scioly_tokens WHERE discord_user_id = ?",
        row.discord_user_id,
    )
    .execute(&mut *tx)
    .await
    .map_err(|err| {
        info!("{}", err);
        ErrorInternalServerError(err)
    })?;

    let whoami_res = client
        .get(format!("{}/oauth/api/whoami/", OAUTH_HOST_URL))
        .header("Authorization", format!("Bearer {}", body_res.access_token))
        .send()
        .await
        .map_err(|err| {
            info!("{}", err);
            ErrorInternalServerError(err)
        })?;
    if whoami_res.status() != reqwest::StatusCode::OK {
        return Ok(HttpResponse::new(
            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        ));
    }
    let whoami = whoami_res.json::<WhoAmIResponse>().await.map_err(|err| {
        info!("{}", err);
        ErrorInternalServerError(err)
    })?;

    sqlx::query!(
        "INSERT INTO scioly_tokens (discord_user_id, phpbb_user_id, access_token, refresh_token, access_expires_at) VALUES (?, ?, UNHEX(?), UNHEX(?), FROM_UNIXTIME(?))",
        row.discord_user_id,
        body_res.phpbb_user_id,
        body_res.access_token,
        body_res.refresh_token,
        body_res.expires_at
    ).execute(&mut *tx).await.map_err(ErrorInternalServerError)?;

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
        row.discord_user_id,
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
        row.discord_user_id,
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
    .execute(&mut *tx)
    .await
    .map_err(|err| {
        info!("{}", err);
        ErrorInternalServerError(err)
    })?;

    tx.commit().await.map_err(|err| {
        info!("{}", err);
        ErrorInternalServerError(err)
    })?;

    Ok(HttpResponse::new(actix_web::http::StatusCode::OK).set_body(BoxBody::new(format!("Thank you {} for successfully linking your Scioly.org account to Pi-Bot! You may now close this page.", whoami.username))))
}
