use axum::{Json, extract::State};
use serde::Serialize;

use crate::{AppState, server_address};

pub const SOCKET_URL: &str = "/ws";
pub const NOTIF_URL: &str = "/push";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitiesResponse<'a> {
    spec: &'a str,
    protocol: &'a str,
    websocket: WebSocketCapability<'a>,
    webpush: WebPushCapability,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketCapability<'a> {
    auth: &'a str,
    endpoint: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebPushCapability {
    vapid: Vapid,
    endpoints: WebPushEndpoints,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Vapid {
    public_key: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebPushEndpoints {
    register: String,
    revoke: String,
}

pub async fn capabilities_handler(
    State(state): State<AppState>,
) -> Json<CapabilitiesResponse<'static>> {
    let address = server_address();
    // Derive from domain
    let ws = address
        .replace("https://", "wss://")
        .replace("http://", "ws://")
        + SOCKET_URL;

    Json(CapabilitiesResponse {
        spec: "https://example.chat/specs/ecp/1.0",
        protocol: "eko-chat",
        websocket: WebSocketCapability {
            auth: "bearer",
            endpoint: ws,
        },

        webpush: WebPushCapability {
            vapid: Vapid {
                public_key: state.notification_service.public_key.clone(),
            },
            endpoints: WebPushEndpoints {
                register: format!("{}{}/register", address, NOTIF_URL),
                revoke: format!("{}{}/revoke", address, NOTIF_URL),
            },
        }, // state.domain.to_string() + NOTIF_URL,
    })
}
