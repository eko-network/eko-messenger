mod config;
mod jwt;
mod storage;

use std::{net::SocketAddr, sync::Arc};

use axum::Extension;
use axum::extract::{MatchedPath, Request, State};
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{
    Router,
    middleware::{self, Next},
    response::Html,
    routing::get,
};
use eko_messenger::notification_tokens;
use eko_messenger::server::{INBOX_ROUTE, OUTBOX_ROUTE, WebSocketService};
use eko_messenger::{
    AppError, MessengerContext, RequestAuth, outbox_routes, protocol_routes, public_routes,
};
use storage::Storage;
use tokio::net::TcpListener;
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::jwt::JWTVerifier;
use crate::storage::{pg_init, pg_init_with_migration};

async fn require_device_approval(req: Request, next: Next) -> Response {
    match req.extensions().get::<RequestAuth>() {
        Some(auth) if auth.device_approved => next.run(req).await,
        Some(_) => AppError::DevicePending("Device not yet approved".into()).into_response(),
        None => (StatusCode::UNAUTHORIZED, "Missing Authorization Header").into_response(),
    }
}

async fn auth(
    State(ctx): State<MessengerContext>,
    Extension(jwt): Extension<JWTVerifier>,
    mut req: Request,
    next: Next,
) -> Response {
    if let Some(auth) = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
    {
        let verified = match notification_tokens::hash(auth) {
            Some(hash) => {
                let allowed = notification_service_route(&req);
                notification_service_auth(&ctx, allowed, &hash).await
            }
            None => jwt.verify(auth).await,
        };
        return match verified {
            Ok(mut claims) => {
                // a token outlives changes to its device, which can be removed, e.g. by a device
                // reset, or lose its approval, so the device record decides
                match ctx.storage.device_approved(&claims.uid, claims.did).await {
                    Ok(Some(approved)) => claims.device_approved = approved,
                    Ok(None) => {
                        return AppError::DeviceRemoved(format!("{} was removed", claims.did))
                            .into_response();
                    }
                    Err(e) => return e.into_response(),
                }
                req.extensions_mut().insert(claims);
                next.run(req).await
            }
            Err(e) if notification_tokens::hash(auth).is_some() => e.into_response(),
            Err(e) => {
                debug!(
                    jwt_error = %e,
                    jwt_token = %jwt::debug_token(auth).unwrap_or_else(|| auth.to_string()),
                );
                (StatusCode::UNAUTHORIZED, e).into_response()
            }
        };
    }
    (StatusCode::UNAUTHORIZED, "Missing Authorization Header").into_response()
}

/// A notification token only reads the inbox and sends to the outbox, where only
/// `Delivered` activities are accepted from it.
fn notification_service_route(req: &Request) -> bool {
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str);
    match *req.method() {
        Method::GET => route == Some(INBOX_ROUTE),
        Method::POST => route == Some(OUTBOX_ROUTE),
        _ => false,
    }
}

async fn notification_service_auth(
    ctx: &MessengerContext,
    allowed: bool,
    token_hash: &[u8],
) -> Result<RequestAuth, AppError> {
    if !allowed {
        return Err(AppError::Forbidden(
            "A notification token is only good for the inbox and outbox".into(),
        ));
    }
    let (uid, did) = ctx
        .storage
        .notification_token_device(token_hash)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Unknown notification token".into()))?;
    Ok(RequestAuth {
        uid,
        did,
        // the device record decides
        device_approved: false,
        notification_service: true,
    })
}

fn app(ctx: MessengerContext, jwt: JWTVerifier) -> Router {
    let approved = protocol_routes()
        .route_layer(middleware::from_fn(require_device_approval))
        .route_layer(middleware::from_fn_with_state(ctx.clone(), auth))
        .layer(Extension(jwt.clone()));

    let outbox = outbox_routes()
        .route_layer(middleware::from_fn_with_state(ctx.clone(), auth))
        .layer(Extension(jwt));

    public_routes()
        .route("/", get(|| async { Html("<h1>eko-messenger</h1>") }))
        .merge(approved)
        .merge(outbox)
        .with_state(ctx)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive("info".parse()?)
                .from_env_lossy(),
        )
        .init();

    let cfg = Config::from_env();
    let addr: SocketAddr = cfg.get_addr()?;
    let jwt = JWTVerifier::new(&cfg.jwt_jwks_url).await?;
    let domain = Arc::new(cfg.domain);
    let ctx = MessengerContext {
        domain: domain.clone(),
        storage: Arc::new(Storage::new(
            pg_init(&cfg.supabase_db_url)?,
            pg_init_with_migration(&cfg.db_url).await?,
        )),
        sockets: Arc::new(WebSocketService::new()),
    };

    info!("eko listening on {}", domain);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app(ctx, jwt)).await?;

    Ok(())
}
