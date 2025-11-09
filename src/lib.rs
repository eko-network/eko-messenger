pub mod activitypub;
pub mod auth;
pub mod errors;
pub mod firebase_auth;
pub mod outbox;

use crate::{
    activitypub::Person,
    auth::{Auth, login_handler, logout_handler, refresh_token_handler},
    errors::AppError,
    firebase_auth::FirebaseAuth,
    outbox::post_to_outbox,
};
use anyhow::Context;
use axum::{
    Router,
    extract::{Path, Query, State},
    response::{Html, Json},
    routing::{get, post},
};
use redis::aio::MultiplexedConnection;
use serde::Deserialize;
use std::{env::var, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {
    redis: MultiplexedConnection,
    auth: Arc<Auth<FirebaseAuth>>,
    domain: String,
}

#[derive(Deserialize)]
pub struct WebFingerQuery {
    resource: String,
}

async fn redis_from_env() -> anyhow::Result<redis::aio::MultiplexedConnection> {
    let url = var("REDIS_URL").context("REDIS_URL not found in environment")?;

    let client = redis::Client::open(url).context("Failed to connect to redis")?;

    let redis_conn = client
        .get_multiplexed_tokio_connection()
        .await
        .context("Failed to connect to redis")?;
    Ok(redis_conn)
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let redis = redis_from_env().await?;
    let firebase_auth = FirebaseAuth::new_from_env()?;
    let auth = Auth::new(firebase_auth, redis.clone());
    let domain = var("DOMAIN").unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    let app_state = AppState {
        redis: redis.clone(),
        auth: Arc::new(auth),
        domain,
    };

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/api/v1/login", post(login_handler))
        .route("/api/v1/refresh", post(refresh_token_handler))
        .route("/api/v1/logout", post(logout_handler))
        .route("/api/v1/outbox", post(post_to_outbox))
        .route("/.well-known/webfinger", get(webfinger_handler))
        .route("/users/{username}", get(actor_handler))
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server listening on http://{}", addr);

    let listener = TcpListener::bind(addr).await?;

    axum::serve(listener, app).await?;

    Ok(())
}

async fn root_handler() -> Html<&'static str> {
    Html("
        <!DOCTYPE html>
        <html lang='en'>
        <head>
            <meta charset='UTF-8'>
            <meta name='viewport' content='width=device-width, initial-scale=1.0'>
            <title>Rust Web Server</title>
            <style>
                body { font-family: sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background-color: #f0f4f8; }
                .container { background-color: white; padding: 40px; border-radius: 12px; box-shadow: 0 10px 25px rgba(0, 0, 0, 0.1); text-align: center; }
                h1 { color: #cc6633; margin-bottom: 10px; }
                p { color: #333; }
            </style>
        </head>
        <body>
            <div class='container'>
                <h1>Hello from Rust!</h1>
                <p>This server is running on Axum and Tokio.</p>
            </div>
        </body>
        </html>
    ")
}

async fn webfinger_handler(
    State(state): State<AppState>,
    Query(query): Query<WebFingerQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let resource = query.resource;
    if !resource.starts_with("acct:") {
        return Err(AppError::BadRequest("Invalid resource format".to_string()));
    }

    let parts: Vec<&str> = resource.trim_start_matches("acct:").split('@').collect();
    if parts.len() != 2 {
        return Err(AppError::BadRequest("Invalid resource format".to_string()));
    }
    let username = parts[0];
    let domain = parts[1];

    if domain != state.domain {
        return Err(AppError::NotFound(
            "User not found on this domain".to_string(),
        ));
    }

    let actor_url = format!("http://{}/users/{}", state.domain, username);

    let jrd = serde_json::json!({
        "subject": resource,
        "links": [
            {
                "rel": "self",
                "type": "application/activity+json",
                "href": actor_url,
            }
        ]
    });

    Ok(Json(jrd))
}

async fn actor_handler(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<Person>, AppError> {
    let actor = activitypub::create_actor(&username, &state.domain);

    Ok(Json(actor))
}
