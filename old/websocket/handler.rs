use crate::{AppState, auth::Claims, errors::AppError};
use axum::{
    Extension,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, Utf8Bytes, WebSocket},
    },
    response::IntoResponse,
};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Extension(claims): Extension<Arc<Claims>>,
) -> Result<impl IntoResponse, AppError> {
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, claims.clone())))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, claims: Arc<Claims>) {
    let (tx, mut rx) = mpsc::unbounded_channel();

    info!(
        "Client {} - {} connected via WebSocket",
        claims.sub, claims.did
    );
    state.sockets.insert(claims.did, tx.clone());

    // Send messages from inbox to client
    match state.storage.activities.inbox_activities(claims.did).await {
        Ok(inbox_items) => {
            for item in inbox_items {
                if let Ok(message_json) = serde_json::to_string(&item)
                    && tx
                        .send(Message::Text(Utf8Bytes::from(message_json)))
                        .is_err()
                {
                    warn!(
                        "Failed to send offline message to {} - {}",
                        claims.sub, claims.did
                    );
                    break;
                }
            }
        }
        Err(e) => {
            warn!(
                "Failed to retrieve inbox for {} - {}: {:?}",
                claims.sub, claims.did, e
            );
        }
    }

    loop {
        tokio::select! {
            // Send messages from channel to WebSocket
            Some(msg) = rx.recv() => {
                if socket.send(msg).await.is_err() {
                    break;
                }
            }
            // Receive messages from WebSocket
            Some(msg) = socket.recv() => {
                match msg {
                    Ok(Message::Close(_)) => break,
                    Err(_) => break,
                    // Pings are automatically responded to by axum with pongs
                    // Ignore all other message types
                    _ => {}
                }
            }
            else => break,
        }
    }
    state.sockets.remove(&claims.did);
    info!("Client {} - {} disconnected", claims.sub, claims.did);
}
