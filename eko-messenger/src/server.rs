mod handlers;
use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use handlers::get_devices;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::{
    server::handlers::{capabilities_handler, get_inbox, post_to_outbox, take_key},
    storage::Storage,
};
pub const ACTIVITY_STREAMS_CONTEXT: &str = "https://www.w3.org/ns/activitystreams";
pub const ECP_CONTEXT: &str = "https://www.w3.org/ns/activitystreams";

pub const USERS_ENDPOINT: &str = "users";
pub const DEVICE_ENDPOINT: &str = "devices";
pub const DEVICE_KEYS_ENDPOINT: &str = "keys";

#[derive(Clone)]
pub struct MessengerContext {
    pub domain: Arc<String>,
    pub storage: Arc<dyn Storage>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RequestAuth {
    pub uid: String,
    pub did: crate::devices::DeviceId,
}

pub fn protocol_routes() -> Router<MessengerContext> {
    Router::new()
        .route("/users/{uid}/devices", get(get_devices))
        .route("/users/{uid}/devices/{did}/keys", post(take_key))
        .route("/users/{uid}/outbox", post(post_to_outbox))
        .route("/users/{uid}/inbox", get(get_inbox))
}

pub fn public_routes() -> Router<MessengerContext> {
    Router::new().route("/.well-known/ecp", get(capabilities_handler))
}

pub fn context() -> Value {
    let mut m = Map::new();
    m.insert("ecp".to_string(), Value::String(ECP_CONTEXT.to_string()));
    return Value::Array(vec![
        Value::String(ACTIVITY_STREAMS_CONTEXT.to_string()),
        Value::Object(m),
    ]);
}
