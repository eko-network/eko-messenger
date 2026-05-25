use axum::{Json, extract::State};
use serde::Serialize;

use crate::MessengerContext;

pub const SOCKET_URL: &str = "/ws";
pub const NOTIF_URL: &str = "/push";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitiesResponse<'a> {
    spec: &'a str,
    protocol: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    websocket: Option<WebSocketCapability<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    webpush: Option<WebPushCapability>,
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
    State(ctx): State<MessengerContext>,
) -> Json<CapabilitiesResponse<'static>> {
    // Derive from domain
    // let ws = ctx
    //     .domain
    //     .replace("https://", "wss://")
    //     .replace("http://", "ws://")
    //     + SOCKET_URL;

    Json(CapabilitiesResponse {
        spec: "https://example.chat/specs/ecp/1.0",
        protocol: "eko-chat",
        websocket: None,
        webpush: None,
    })
}
