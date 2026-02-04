use crate::{
    activitypub::{
        Activity,
        types::activity::{ActivityBase, CreateView},
    },
    devices::DeviceId,
};
use axum::extract::ws::{Message, Utf8Bytes};
use dashmap::DashMap;
use serde::Serialize;
use tokio::sync::mpsc;
use tracing::{info, warn};

pub trait ActivityData: ActivityBase + Serialize {}

impl ActivityData for Activity {}
impl<'a> ActivityData for CreateView<'a> {}

pub struct WebSocketService {
    sockets: DashMap<DeviceId, mpsc::UnboundedSender<Message>>,
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
    pub async fn try_websocket_delivery<T: ActivityData>(
        &self,
        activity: &T,
        did: DeviceId,
    ) -> bool {
        // Check if the recipient device is online
        if let Some(sender) = self.sockets.get(&did) {
            info!(
                "{} - {} online, trying to send via socket",
                activity.to(),
                did
            );

            // Create message for WebSocket
            if let Ok(message_json) = serde_json::to_string(&activity) {
                // Try to send via WebSocket
                if let Err(e) = sender.send(Message::Text(Utf8Bytes::from(message_json))) {
                    warn!(
                        "Failed to send to online client {}, falling back to inbox: {}",
                        activity.to(),
                        e
                    );
                    return false;
                }

                return true;
            }
        }

        false
    }
}
