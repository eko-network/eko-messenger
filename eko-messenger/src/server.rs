mod handlers;
use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use handlers::get_devices;

use crate::{server::handlers::post_to_outbox, storage::Storage};
// ---------------------------------------------------------------------------
// Context — protocol services shared by all handlers
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct MessengerContext {
    pub domain: Arc<String>,
    pub storage: Arc<dyn Storage>,
}

// ---------------------------------------------------------------------------
// Per-request identity — injected by YOUR auth middleware
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct RequestAuth {
    pub uid: String,
    pub did: crate::devices::DeviceId,
}

pub fn protocol_routes() -> Router<MessengerContext> {
    Router::new()
        .route("/users/{uid}/devices", get(get_devices))
        .route("/users/{uid}/outbox", post(post_to_outbox))
}

pub fn public_routes() -> Router<MessengerContext> {
    Router::new()
}
