use std::collections::HashSet;

use crate::{
    AppState,
    activitypub::{Create, EncryptedMessage, types::generate_create},
    devices::DeviceId,
    errors::AppError,
    storage::models::StoredInboxEntry,
};
use axum::extract::ws::{Message, Utf8Bytes};
use tracing::{debug, info, warn};

/// Main service for orchestrating message delivery
pub struct MessagingService;

impl MessagingService {
    /// Process an outgoing message envelope from a local user
    /// Handles routing to local or remote recipients
    pub async fn process_outgoing_message(
        state: &AppState,
        activity: &Create,
        sender_actor: &str,
        sender_did: &DeviceId,
    ) -> Result<(), AppError> {
        // Iterate through all recipients
        for recipient_actor_id in &activity.object.to {
            // TODO: Check if recipient is local or remote
            if state
                .storage
                .actors
                .is_local_actor(recipient_actor_id)
                .await?
            {
                Self::deliver_local(
                    state,
                    &activity.object,
                    sender_actor,
                    recipient_actor_id,
                    sender_actor,
                    sender_did,
                )
                .await?;
            } else {
                Self::deliver_remote(state, activity, recipient_actor_id).await?;
            }
        }

        Ok(())
    }

    async fn validate_message_recipients(
        to: &str,
        from: &str,
        from_did: &DeviceId,
        message: &EncryptedMessage,
        state: &AppState,
    ) -> Result<(), AppError> {
        let mut recipient_devices = crate::devices::DeviceService::list_device_ids(
            state,
            &crate::activitypub::actor_uid(to)?,
        )
        .await?;
        let envelope_device_ids: HashSet<String> =
            message.content.iter().map(|e| e.to.clone()).collect();

        // if the message is from yourself it shouldn't be addressed to yourself
        if to == from {
            recipient_devices.remove(&from_did.to_url(&state.domain));
        }

        if envelope_device_ids != recipient_devices {
            return Err(AppError::BadRequest("device_list_mismatch".to_string()));
        }

        Ok(())
    }

    /// Deliver message to a local recipient
    async fn deliver_local(
        state: &AppState,
        message: &EncryptedMessage,
        sender_actor: &str,
        recipient_actor_id: &str,
        sender_actor_id: &str,
        sender_did: &DeviceId,
    ) -> Result<(), AppError> {
        // Validate envelope has correct device count for recipient
        Self::validate_message_recipients(
            recipient_actor_id,
            sender_actor_id,
            sender_did,
            message,
            state,
        )
        .await?;

        let mut did_to_notif: Vec<_> = Vec::with_capacity(message.content.len());

        for entry in &message.content {
            info!("SEND for {}, {}", recipient_actor_id, entry.to);

            let did = DeviceId::from_url(&entry.to)?;
            // Try to deliver via WebSocket if recipient is online
            if Self::try_websocket_delivery(state, sender_actor, recipient_actor_id, entry, did)
                .await?
            {
                debug!("Delivered via WebSocket to {}", recipient_actor_id);
                continue;
            }

            // Recipient offline or WebSocket failed, store in inbox
            state
                .storage
                .inbox
                .insert_inbox_entry(
                    recipient_actor_id,
                    did,
                    StoredInboxEntry {
                        actor_id: sender_actor.to_string(),
                        from_did: entry.from.clone(),
                        content: entry.content.clone(),
                    },
                )
                .await?;

            // Queue for push notification
            did_to_notif.push(did);
        }

        // Send push notifications for offline devices
        if !did_to_notif.is_empty() {
            state.notification_service.notify(&did_to_notif).await?;
        }

        Ok(())
    }

    /// Try to deliver message via WebSocket to online recipient
    /// Returns true if successfully delivered via WebSocket
    async fn try_websocket_delivery(
        state: &AppState,
        sender_actor: &str,
        recipient_actor_id: &str,
        entry: &crate::activitypub::EncryptedMessageEntry,
        did: DeviceId,
    ) -> Result<bool, AppError> {
        // Check if the recipient device is online
        if let Some(sender) = state.sockets.get(&did) {
            info!(
                "{} - {} online, trying to send via socket",
                recipient_actor_id, entry.to
            );

            // Create message for WebSocket
            let ws_message = generate_create(
                recipient_actor_id.to_string(),
                sender_actor.to_string(),
                entry.to.clone(),
                entry.from.clone(),
                entry.content.clone(),
            );

            let message_json = serde_json::to_string(&ws_message)?;

            // Try to send via WebSocket
            if let Err(e) = sender.send(Message::Text(Utf8Bytes::from(message_json))) {
                warn!(
                    "Failed to send to online client {}, falling back to inbox: {}",
                    recipient_actor_id, e
                );
                return Ok(false);
            }

            return Ok(true);
        }

        Ok(false)
    }

    /// TODO Deliver message to a remote recipient
    async fn deliver_remote(
        state: &AppState,
        activity: &Create,
        recipient_actor_id: &str,
    ) -> Result<(), AppError> {
        // FIXME AI generated. Needs to be fixed.

        // 1. Fetch remote actor to get inbox URL
        // let remote_actor = crate::activitypub::client::fetch_actor(recipient_actor_id).await?;

        // 2. Validate remote actor has devices (for device count validation)
        // let remote_devices = DeviceService::get_remote_device_count(&remote_actor).await?;
        // crate::messaging::envelope::validate_envelope_for_recipient(
        //     activity.object.content.len(),
        //     remote_devices,
        // )?;

        // 3. Send via ActivityPub client (or queue for async delivery)
        // let ap_client = crate::activitypub::client::ActivityPubClient::new(
        //     state.domain.to_string()
        // );
        // ap_client.post_to_inbox(&remote_actor.inbox, activity).await?;

        // OR maybe we queue for delivery worker so it can retry
        // state.delivery_queue.enqueue(DeliveryJob {
        //     activity: activity.clone(),
        //     recipient_inbox: remote_actor.inbox,
        //     attempts: 0,
        //     next_retry: Utc::now(),
        // }).await?;

        tracing::info!(
            "Remote delivery to {} not yet implemented",
            recipient_actor_id
        );
        let _ = (state, activity); // Suppress unused warnings for now
        Ok(())
    }
}
