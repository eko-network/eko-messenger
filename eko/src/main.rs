mod config;
mod storage;

use std::{net::SocketAddr, sync::Arc};

use axum::extract::Request;
use axum::response::Response;
use axum::{
    Router,
    middleware::{self, Next},
    response::Html,
    routing::get,
};
use eko_messenger::{
    MessengerContext, RequestAuth, devices::DeviceId, protocol_routes, public_routes,
};
use storage::Storage;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use crate::config::Config;
use crate::storage::pg_init;

/// Stand-in auth
async fn stub_auth(mut req: Request, next: Next) -> Response {
    req.extensions_mut().insert(RequestAuth {
        uid: "stub-user".into(),
        did: DeviceId::new(Uuid::new_v4()),
    });
    next.run(req).await
}

fn app(ctx: MessengerContext) -> Router {
    let protected = protocol_routes().route_layer(middleware::from_fn(stub_auth));

    public_routes()
        .route("/", get(|| async { Html("<h1>eko-messenger</h1>") }))
        .merge(protected)
        .with_state(ctx)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let cfg = Config::from_env();
    let addr: SocketAddr = cfg.get_addr()?;
    let ctx = MessengerContext {
        domain: Arc::new(cfg.domain),
        storage: Arc::new(Storage::new(pg_init(&cfg.supabase_db_url)?)),
    };

    info!("eko listening on http://{addr}");

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app(ctx)).await?;

    Ok(())
}
