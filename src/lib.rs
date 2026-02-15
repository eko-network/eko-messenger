pub mod activitypub;
pub mod auth;
pub mod config;
pub mod devices;
pub mod errors;
pub mod groups;
pub mod messaging;
pub mod middleware;
pub mod notifications;
pub mod storage;
pub mod websocket;

use crate::{
    activitypub::{
        actor_handler, capabilities_handler, get_devices, get_inbox,
        handlers::capabilities::{NOTIF_URL, SOCKET_URL},
        post_to_outbox, webfinger_handler,
    },
    auth::{
        Auth, OidcProviderState, add_oidc_routes, build_auth, login_handler, logout_handler,
        refresh_token_handler, signup_handler,
    },
    config::storage_config,
    devices::get_approval_status_handler,
    groups::{
        delete_group_state_handler, get_all_group_states_handler, get_group_state_handler,
        upsert_group_state_handler,
    },
    middleware::auth_middleware,
    notifications::{NotificationService, register_handler},
    storage::Storage,
    websocket::{WebSocketService, handler::ws_handler},
};
use axum::middleware::from_fn_with_state;
use axum::{
    Router,
    response::Html,
    routing::{get, post},
};
use axum_client_ip::ClientIpSource;
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
    pub domain: Arc<String>,
    pub auth: Arc<Auth>,
    pub storage: Arc<Storage>,
    pub sockets: Arc<WebSocketService>,
    pub notification_service: Arc<NotificationService>,
    pub oidc_provider: OidcProviderState,
}

pub fn app(app_state: AppState, ip_source_str: String) -> anyhow::Result<Router> {
    let protected_routes = Router::new()
        .route("/auth/v1/logout", post(logout_handler))
        .route(&format!("{}/register", NOTIF_URL), post(register_handler))
        .route("/users/{uid}/outbox", post(post_to_outbox))
        .route("/users/{uid}/inbox", get(get_inbox))
        .route("/users/{uid}/deviceActions", get(get_devices))
        // .route("/devices/{did}/keyCollection", get())
        .route(
            "/devices/{did}/approval-status",
            get(get_approval_status_handler),
        )
        .route(SOCKET_URL, get(ws_handler))
        .route("/users/{uid}/groups", get(get_all_group_states_handler))
        .route(
            "/users/{uid}/groups/{group_id}",
            get(get_group_state_handler)
                .put(upsert_group_state_handler)
                .delete(delete_group_state_handler),
        )
        .route_layer(from_fn_with_state(app_state.clone(), auth_middleware));
    let ip_source: ClientIpSource = ip_source_str.parse()?;

    let router = Router::new()
        .route("/", get(root_handler))
        .route("/auth/v1/login", post(login_handler))
        .route("/auth/v1/signup", post(signup_handler))
        .route("/auth/v1/refresh", post(refresh_token_handler))
        .route("/.well-known/webfinger", get(webfinger_handler))
        .route("/users/{uid}", get(actor_handler))
        .route("/.well-known/ecp", get(capabilities_handler));
    let router = add_oidc_routes(router);

    Ok(router
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
    let port = port_from_env();
    let domain = Arc::new(var("DOMAIN").unwrap_or_else(|_| format!("http://127.0.0.1:{}", port)));
    let storage = Arc::new(storage_config(domain.clone()).await?);

    let (auth, oidc_provider) = build_auth(domain.clone(), storage.clone()).await?;

    let notification_service = NotificationService::new(storage.clone()).await?;

    let app_state = AppState {
        domain,
        auth: Arc::new(auth),
        sockets: Arc::new(WebSocketService::new()),
        notification_service: Arc::new(notification_service),
        storage,
        oidc_provider,
    };

    let app = app(app_state, ip_source)?;

    let listen_addr = var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0".to_string());
    let addr: SocketAddr = format!("{}:{}", listen_addr, port).parse()?;

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
