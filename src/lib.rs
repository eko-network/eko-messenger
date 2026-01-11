pub mod activitypub;
pub mod auth;
pub mod config;
pub mod crypto;
pub mod errors;
pub mod middleware;
pub mod notifications;
pub mod storage;
pub mod websocket;

use crate::{
    activitypub::{
        Person,
        capabilities::{NOTIF_URL, SOCKET_URL},
        capabilities_handler, get_inbox, post_to_outbox, webfinger_handler,
    },
    auth::{Auth, FirebaseAuth, login_handler, logout_handler, refresh_token_handler},
    config::storage_config,
    crypto::get_bundle,
    errors::AppError,
    middleware::auth_middleware,
    notifications::{NotificationService, register_handler},
    storage::Storage,
    websocket::{WebSockets, ws_handler},
};
use axum::middleware::from_fn_with_state;
use axum::{
    Router,
    extract::{Path, State},
    response::{Html, Json},
    routing::{get, post},
};
use axum_client_ip::ClientIpSource;
use dashmap::DashMap;
use std::{
    env::{self, var},
    net::SocketAddr,
    sync::Arc,
};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub auth: Arc<Auth>,
    pub domain: Arc<String>,
    pub storage: Arc<Storage>,
    pub sockets: WebSockets,
    pub notification_service: Arc<NotificationService>,
}

pub fn app(app_state: AppState, ip_source_str: String) -> anyhow::Result<Router> {
    let protected_routes = Router::new()
        .route("/auth/v1/logout", post(logout_handler))
        .route(&format!("{}/register", NOTIF_URL), post(register_handler))
        .route("/users/{uid}/outbox", post(post_to_outbox))
        .route("/users/{uid}/inbox", get(get_inbox))
        .route("/users/{uid}/keys/bundle.json", get(get_bundle))
        .route(SOCKET_URL, get(ws_handler))
        // you can add more routes here
        .route_layer(from_fn_with_state(app_state.clone(), auth_middleware));
    let ip_source: ClientIpSource = ip_source_str.parse()?;
    Ok(Router::new()
        .route("/", get(root_handler))
        .route("/auth/v1/login", post(login_handler))
        .route("/auth/v1/refresh", post(refresh_token_handler))
        .route("/.well-known/webfinger", get(webfinger_handler))
        .route("/users/{uid}", get(actor_handler))
        .route("/.well-known/ecp", get(capabilities_handler))
        .merge(protected_routes)
        .layer(ip_source.into_extension())
        .with_state(app_state))
}
fn port_from_env() -> u16 {
    var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000)
}
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let ip_source = env::var("IP_SOURCE").expect("IP_SOURCE environment variable must be set");
    let storage = Arc::new(storage_config().await?);
    let port = port_from_env();

    let domain = var("DOMAIN").unwrap_or_else(|_| format!("http://127.0.0.1:{}", port));
    let client = reqwest::Client::new();
    let firebase_auth = FirebaseAuth::new_from_env(domain.clone(), client.clone()).await?;
    let notification_service = NotificationService::new(storage.clone()).await?;

    let app_state = AppState {
        auth: Arc::new(Auth::new(firebase_auth, storage.clone())),
        sockets: Arc::new(DashMap::new()),
        notification_service: Arc::new(notification_service),
        domain: Arc::new(domain),
        storage,
    };

    let app = app(app_state, ip_source)?;

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("Server listening on http://{}", addr);

    let listener = TcpListener::bind(addr).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

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

async fn actor_handler(
    State(state): State<AppState>,
    Path(uid): Path<String>,
) -> Result<Json<Person>, AppError> {
    let actor = state.auth.provider.person_from_uid(&uid).await?;
    Ok(Json(actor))
}
