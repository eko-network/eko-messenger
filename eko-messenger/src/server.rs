mod handlers;
use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use handlers::get_devices;
use serde_json::{Map, Value};

use crate::{server::handlers::post_to_outbox, storage::Storage};
pub const ACTIVITY_STREAMS_CONTEXT: &str = "https://www.w3.org/ns/activitystreams";
pub const ECP_CONTEXT: &str = "https://www.w3.org/ns/activitystreams";

pub const USERS_ENDPOINT: &str = "users";
pub const DEVICE_ENDPOINT: &str = "devices";
pub const DEVICE_KEYS_ENDPOINT: &str = "keys";
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

pub fn context() -> Value {
    let mut m = Map::new();
    m.insert("ecp".to_string(), Value::String(ECP_CONTEXT.to_string()));
    return Value::Array(vec![
        Value::String(ACTIVITY_STREAMS_CONTEXT.to_string()),
        Value::Object(m),
    ]);
}
