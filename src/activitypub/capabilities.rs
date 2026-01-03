use axum::{Json, extract::State};
use serde::Serialize;

use crate::AppState;

#[derive(Serialize)]
pub struct CapabilitiesResponse<'a> {
    spec: &'a str,
    protocol: &'a str,
    websocket: String,
}
pub fn ws_url(domain: &str) -> String {
    format!(
        "{}/ws",
        domain
            .replace("https://", "wss://")
            .replace("http://", "ws://")
    )
}
pub async fn capabilities_handler(
    State(state): State<AppState>,
) -> Json<CapabilitiesResponse<'static>> {
    // Derive from domain
    Json(CapabilitiesResponse {
        spec: "https://example.chat/specs/ecp/1.0",
        protocol: "eko-chat",
        websocket: ws_url(&state.domain),
    })
}
