use common::{
    OAUTH_HOST_URL, fetch_new_token, fetch_whoami, update_db_access_token, update_db_user_stats,
};
use log::error;

use actix_web::{
    App, HttpResponse, HttpServer,
    body::BoxBody,
    error::{ErrorInternalServerError, ErrorNotFound},
    get,
    middleware::Logger,
    web,
};

use dotenv::dotenv;
use serde::Deserialize;
use sqlx::MySqlPool;

#[derive(Debug, Clone, Deserialize)]
struct Env {
    database_url: String,
}

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

#[get("/authorize")]
async fn authorize(
    data: web::Data<ServerState>,
    query: web::Query<AuthorizeInputs>,
) -> actix_web::Result<HttpResponse> {
    let mut tx = data.db.begin().await.map_err(|err| {
        error!("{}", err);
        ErrorInternalServerError(err)
    })?;

    let row = match sqlx::query_as!(
        AuthRow,
        "SELECT id, discord_user_id, NOW() >= expires_at AS has_expired FROM authorization_request WHERE state_code = UNHEX(?)",
        query.state_hex
    ).fetch_one(&mut *tx).await {
        Err(sqlx::Error::RowNotFound) => { return Err(ErrorNotFound(sqlx::Error::RowNotFound))}
        Err(e) => {
            error!("{}", e);
            return Err(ErrorInternalServerError(e));
        }
        Ok(row) => row,
    };

    let delete_query = sqlx::query!("DELETE FROM authorization_request WHERE id = ?", row.id);
    if row.has_expired != 0 {
        delete_query.execute(&mut *tx).await.map_err(|err| {
            error!("{}", err);
            ErrorInternalServerError(err)
        })?;
        tx.commit().await.map_err(|err| {
            error!("{}", err);
            ErrorInternalServerError(err)
        })?;
        return Ok(HttpResponse::new(actix_web::http::StatusCode::NOT_FOUND));
    }

    let client = reqwest::Client::new();
    // TODO: turn client_id and client_secret into env vars
    let body_res = fetch_new_token(
        &client,
        &query.code_hex,
        "abcdef1234567890",
        "abcdef1234567890",
    )
    .await
    .map_err(|err| match err {
        common::AccessTokenError::StatusCodeError { expected: _, found } => match found {
            reqwest::StatusCode::NOT_FOUND => ErrorNotFound("authorization code not found"),
            _ => {
                error!("{}/oauth/access_token/: {}", OAUTH_HOST_URL, err);
                ErrorInternalServerError("error contacting scioly.org")
            }
        },
        _ => {
            error!("{}/oauth/access_token/: {}", OAUTH_HOST_URL, err);
            ErrorInternalServerError(err)
        }
    })?;

    update_db_access_token(&mut tx, row.discord_user_id, &body_res)
        .await
        .map_err(|err| {
            error!(
                "{}/oauth/access_token/: error on db update {}",
                OAUTH_HOST_URL, err
            );
            ErrorInternalServerError(err)
        })?;

    let whoami = fetch_whoami(&client, &body_res.access_token)
        .await
        .map_err(|err| {
            error!("{}/oauth/api/whoami/: {}", OAUTH_HOST_URL, err);
            ErrorInternalServerError(err)
        })?;

    update_db_user_stats(&mut *tx, &whoami, row.discord_user_id)
        .await
        .map_err(|err| {
            error!(
                "{}/oauth/api/whoami/: error on db update {}",
                OAUTH_HOST_URL, err
            );
            ErrorInternalServerError(err)
        })?;

    tx.commit().await.map_err(|err| {
        error!("{}", err);
        ErrorInternalServerError(err)
    })?;

    Ok(HttpResponse::new(actix_web::http::StatusCode::OK).set_body(BoxBody::new(format!("Thank you {} for successfully linking your Scioly.org account to Pi-Bot! You may now close this page.", whoami.username))))
}
