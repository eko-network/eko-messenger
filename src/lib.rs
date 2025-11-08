pub mod auth;
pub mod errors;
pub mod firebase_auth;
use crate::{
    auth::{login_handler, logout_handler, refresh_token_handler, Auth},
    firebase_auth::FirebaseAuth,
};
use anyhow::{anyhow, Context};
use axum::{
    response::Html,
    routing::{get, post},
    Router,
};
use std::{env::var_os, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {
    auth: Arc<Auth<FirebaseAuth>>,
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
    // premptive clone
    let auth = Auth::new(firebase_auth, redis.clone());
    let app_state = AppState {
        auth: Arc::new(auth),
    };

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/api/v1/login", post(login_handler))
        .route("/api/v1/refresh", post(refresh_token_handler))
        .route("/api/v1/logout", post(logout_handler))
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
