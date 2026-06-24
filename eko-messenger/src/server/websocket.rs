use axum::{
    Extension,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, Utf8Bytes, WebSocket},
    },
    response::IntoResponse,
};
use dashmap::DashMap;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::{Activity, AppError, DeviceId, MessengerContext, RequestAuth};

pub struct WebSocketService {
    sockets: DashMap<DeviceId, mpsc::UnboundedSender<Message>>,
}

impl Default for WebSocketService {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSocketService {
    pub fn new() -> Self {
        WebSocketService {
            sockets: DashMap::new(),
        }
    }

    pub fn insert(
        &self,
        did: DeviceId,
        tx: mpsc::UnboundedSender<Message>,
    ) -> Option<mpsc::UnboundedSender<Message>> {
        self.sockets.insert(did, tx)
    }
    pub fn remove(&self, did: &DeviceId) -> Option<(DeviceId, mpsc::UnboundedSender<Message>)> {
        self.sockets.remove(did)
    }

    /// Try to deliver message via WebSocket to online recipient
    /// Returns true if successfully delivered via WebSocket
    pub async fn try_websocket_delivery(
        &self,
        activity: Activity,
        target_uid: String,
        target_did: DeviceId,
    ) -> bool {
        // Check if the recipient device is online
        if let Some(sender) = self.sockets.get(&target_did) {
            info!(
                "{} - {} online, trying to send via socket",
                target_uid, target_did
            );

            if let Ok(message_json) = serde_json::to_string(&activity) {
                if let Err(e) = sender.send(Message::Text(Utf8Bytes::from(message_json))) {
                    warn!(
                        "Failed to send to online client {}, falling back to inbox: {}",
                        target_did, e
                    );
                    return false;
                }

                return true;
            }
        }

        false
    }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(ctx): State<MessengerContext>,
    Extension(auth): Extension<RequestAuth>,
) -> Result<impl IntoResponse, AppError> {
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, ctx, auth)))
}

async fn handle_socket(mut socket: WebSocket, ctx: MessengerContext, auth: RequestAuth) {
    let (tx, mut rx) = mpsc::unbounded_channel();

    info!("Client {} - {} connected via WebSocket", auth.uid, auth.did);
    ctx.sockets.insert(auth.did, tx.clone());

    // Send messages from inbox to client
    match ctx.storage.inbox_activities(auth.did).await {
        Ok(inbox_items) => {
            for item in inbox_items {
                if let Ok(message_json) = serde_json::to_string(&item)
                    && tx
                        .send(Message::Text(Utf8Bytes::from(message_json)))
                        .is_err()
                {
                    warn!(
                        "Failed to send offline message to {} - {}",
                        auth.uid, auth.did
                    );
                    break;
                }
            }
        }
        Err(e) => {
            warn!(
                "Failed to retrieve inbox for {} - {}: {:?}",
                auth.uid, auth.did, e
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
    ctx.sockets.remove(&auth.did);
    info!("Client {} - {} disconnected", auth.uid, auth.did);
}
