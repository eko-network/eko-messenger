pub mod activitypub;
pub mod auth;
pub mod errors;
pub mod firebase_auth;
mod jwt_helper;
pub mod middleware;

use crate::{
    auth::{Auth, login_handler, logout_handler, refresh_token_handler},
    errors::AppError,
    firebase_auth::FirebaseAuth,
    middleware::auth_middleware,
};
use anyhow::{Context, anyhow};
use axum::middleware::from_fn_with_state;
use axum::{
    Router,
    extract::{Path, Query},
    response::{Html, Json},
    routing::{get, post},
};
use serde::Deserialize;
use std::{env::var_os, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {
    auth: Arc<Auth<FirebaseAuth>>,
}

#[derive(Deserialize)]
pub struct WebFingerQuery {
    resource: String,
}

async fn redis_from_env() -> anyhow::Result<redis::aio::MultiplexedConnection> {
    let url = var_os("REDIS_URL")
        .expect("REDIS_URL not found in enviroment")
        .into_string()
        .map_err(|_| anyhow!("Failed to convert from OsString to String"))?;

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
    let app_state = AppState {
        auth: Arc::new(auth),
    };

    let protected_routes = Router::new()
        .route("/auth/logout", post(logout_handler))
        // you can add more routes here
        .route_layer(from_fn_with_state(app_state.clone(), auth_middleware));

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/auth/login", post(login_handler))
        .route("/auth/refresh", post(refresh_token_handler))
        .route("/.well-known/webfinger", get(webfinger_handler))
        .route("/users/{username}", get(actor_handler))
        .merge(protected_routes)
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

    let server_domain = "127.0.0.1:3000";
    if domain != server_domain {
        return Err(AppError::NotFound(
            "User not found on this domain".to_string(),
        ));
    }

    let actor_url = format!("http://{}/users/{}", server_domain, username);

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

async fn actor_handler(Path(username): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let server_domain = "127.0.0.1:3000";
    let actor = activitypub::create_actor(&username, server_domain);
    Ok(Json(actor))
}
