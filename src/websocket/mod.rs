use crate::{AppState, auth::Claims, errors::AppError};
use axum::{
    Extension,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

pub type WsSender = mpsc::UnboundedSender<Message>;
pub type WebSockets = Arc<DashMap<(String, i32), WsSender>>;

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
    state.sockets.insert((claims.sub.clone(), claims.did), tx);

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
    state.sockets.remove(&(claims.sub.clone(), claims.did));
    info!("Client {} - {} disconnected", claims.sub, claims.did);
}
