mod handlers;
pub mod notification;
mod websocket;
use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use handlers::get_devices;
use serde::Deserialize;
use serde_json::{Map, Value};
pub use websocket::WebSocketService;

use crate::{
    server::{
        handlers::{capabilities_handler, get_inbox, post_to_outbox, take_key},
        websocket::ws_handler,
    },
    storage::Storage,
};
pub const ACTIVITY_STREAMS_CONTEXT: &str = "https://www.w3.org/ns/activitystreams";
pub const ECP_CONTEXT: &str = "https://www.w3.org/ns/activitystreams";

pub const USERS_ENDPOINT: &str = "users";
pub const DEVICE_ENDPOINT: &str = "devices";
pub const DEVICE_KEYS_ENDPOINT: &str = "keys";

pub const SOCKET_URL: &str = "/ws";

#[derive(Clone)]
pub struct MessengerContext {
    pub domain: Arc<String>,
    pub storage: Arc<dyn Storage>,
    pub sockets: Arc<WebSocketService>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RequestAuth {
    pub uid: String,
    pub did: crate::devices::DeviceId,
    pub device_approved: bool,
}

impl RequestAuth {
    pub fn require_device_approval(&self) -> Result<(), crate::AppError> {
        if self.device_approved {
            Ok(())
        } else {
            Err(crate::AppError::DevicePending(
                "Device not yet approved".into(),
            ))
        }
    }
}

pub fn protocol_routes() -> Router<MessengerContext> {
    Router::new()
        .route("/users/{uid}/devices", get(get_devices))
        .route("/users/{uid}/devices/{did}/keys", post(take_key))
        .route("/users/{uid}/inbox", get(get_inbox))
        .route(SOCKET_URL, get(ws_handler))
}

pub fn outbox_routes() -> Router<MessengerContext> {
    Router::new()
        .route("/users/{uid}/outbox", post(post_to_outbox))
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
