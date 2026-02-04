use std::collections::HashSet;

use crate::{
    AppState,
    activitypub::{Activity, EncryptedMessageEntry, handlers::outbox::KEY_COLLECTION_URL},
    devices::DeviceId,
    errors::AppError,
};
use axum::extract::ws::{Message, Utf8Bytes};
use futures::future::join_all;
use serde::Serialize;
use tracing::{info, warn};

pub trait ActivityData: Serialize {
    fn id(&self) -> Option<&str>;
    fn actor(&self) -> &str;
    fn to(&self) -> &str;
}

#[derive(Serialize)]
struct CreateView<'a> {
    #[serde(rename = "@context")]
    context: &'a serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<&'a str>,
    actor: &'a str,
    object: EncryptedMessageView<'a>,
    to: &'a str,
    #[serde(rename = "type")]
    type_field: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EncryptedMessageView<'a> {
    #[serde(rename = "@context")]
    context: &'a serde_json::Value,
    #[serde(rename = "type")]
    type_field: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<&'a str>,
    content: &'a [EncryptedMessageEntry],
    attributed_to: &'a str,
    to: &'a [String],
}

impl ActivityData for Activity {
    fn id(&self) -> Option<&str> {
        self.as_base().id().map(|s| s.as_str())
    }
    fn actor(&self) -> &str {
        self.as_base().actor()
    }
    fn to(&self) -> &str {
        self.as_base().to()
    }
}

impl<'a> ActivityData for CreateView<'a> {
    fn id(&self) -> Option<&str> {
        self.id
    }
    fn actor(&self) -> &str {
        self.actor
    }
    fn to(&self) -> &str {
        self.to
    }
}

/// Main service for orchestrating message delivery
pub struct MessagingService;

impl MessagingService {
    /// Process an outgoing message envelope from a local user
    /// Handles routing to local or remote recipients
    pub async fn process_outgoing_message(
        state: &AppState,
        activity: &Activity,
        from_did: &DeviceId,
    ) -> Result<(), AppError> {
        if state
            .storage
            .actors
            .is_local_actor(activity.as_base().to())
            .await?
        {
            Self::deliver_local(state, activity, from_did).await?;
        } else {
            Self::deliver_remote(state, activity).await?;
        }

        Ok(())
    }

    /// Deliver message to a local recipient
    async fn deliver_local(
        state: &AppState,
        activity: &Activity,
        from_did: &DeviceId,
    ) -> Result<(), AppError> {
        match activity {
            Activity::Create(create) => {
                let is_sync_message = activity.as_base().actor() == activity.as_base().to();

                let mut fanout = crate::devices::DeviceService::list_device_ids(
                    state,
                    &crate::activitypub::actor_uid(&activity.as_base().to())?,
                )
                .await?;
                let from_dids: HashSet<&String> =
                    create.object.content.iter().map(|e| &e.from).collect();

                let to_dids: HashSet<&String> =
                    create.object.content.iter().map(|e| &e.to).collect();

                if from_dids.len() != 1 {
                    return Err(AppError::BadRequest(
                        "Message should be sent from a single device".to_string(),
                    ));
                }

                let from_did_url = from_did.to_url(&state.domain);

                if from_did_url != *from_dids.into_iter().next().unwrap() {
                    return Err(AppError::BadRequest(
                        "Message sender does not match from".to_string(),
                    ));
                }

                if is_sync_message {
                    fanout.remove(&from_did_url);
                }

                if to_dids.len() != fanout.len() || !!to_dids.iter().all(|&id| fanout.contains(id))
                {
                    //TODO Reject activity
                    return Err(AppError::BadRequest("device_list_mismatch".into()));
                }

                state.storage.activities.insert_create(&create).await?;

                join_all(create.object.content.iter().map(|entry| async move {
                    let activity_view = CreateView {
                        context: &create.context,
                        id: create.id.as_deref(),
                        actor: &create.actor,
                        object: EncryptedMessageView {
                            context: &create.object.context,
                            type_field: &create.object.type_field,
                            id: create.object.id.as_deref(),
                            content: std::slice::from_ref(entry),
                            attributed_to: &create.object.attributed_to,
                            to: &create.object.to,
                        },
                        to: &create.to,
                        type_field: "Create",
                    };

                    if let Ok(did) = DeviceId::from_url(&entry.to) {
                        if !Self::try_websocket_delivery(&state, &activity_view, did).await {
                            if let Err(e) = state.notification_service.notify(did).await {
                                warn!("Tried to notify {} Error: {:?}", entry.to, e);
                            }
                        }
                    } else {
                        warn!("Tried to notify {}, url malformed", entry.to);
                    }
                }))
                .await;
            }
            Activity::Take(take) => {
                // this redoes compute from prev function (a little bad)
                let device_url = take.to.trim_end_matches(KEY_COLLECTION_URL);
                let target_did = DeviceId::from_url(device_url)?;
                // try to send over socket, if it fails write to db
                if !Self::try_websocket_delivery(&state, activity, target_did).await {
                    state
                        .storage
                        .activities
                        .insert_non_create(activity, &vec![target_did])
                        .await?;
                }
            }
            Activity::Delivered(delivered) => {
                let is_sync_message = activity.as_base().actor() == activity.as_base().to();

                let fanout = crate::devices::DeviceService::list_device_ids(
                    state,
                    &crate::activitypub::actor_uid(&activity.as_base().to())?,
                )
                .await?;

                let create_id = &delivered.object;

                // Delete the delivery request for this Create and device
                // Returns true if found and deleted, false if not found
                let was_deleted = state
                    .storage
                    .activities
                    .delete_delivery(create_id, from_did)
                    .await?;

                if !was_deleted {
                    // The delivery request doesn't exist - either already delivered or not a Create
                    warn!(
                        "Delivered activity {} references non-existent delivery for create {} and device {}",
                        delivered.id.as_deref().unwrap_or("unknown"),
                        create_id,
                        from_did
                    );
                    return Ok(());
                }

                // Check if this is the first delivery for this message
                // The server still tracks all deliveries, but only notifies the sender once
                let is_first_delivery = state
                    .storage
                    .activities
                    .claim_first_delivery(create_id)
                    .await?;

                // don't sync deliveries to yourself and don't send duplicates for the same message
                if !is_sync_message && is_first_delivery {
                    let mut failed_dids = Vec::new();

                    for device_url in fanout {
                        if let Ok(target_did) = DeviceId::from_url(&device_url) {
                            // Try to send via websocket
                            if !Self::try_websocket_delivery(state, activity, target_did).await {
                                failed_dids.push(target_did);
                            }
                        }
                    }

                    // If any devices failed to receive via websocket, insert the activity
                    if !failed_dids.is_empty() {
                        state
                            .storage
                            .activities
                            .insert_non_create(activity, &failed_dids)
                            .await?;
                    }
                }
            }
        };

        Ok(())
    }

    /// Try to deliver message via WebSocket to online recipient
    /// Returns true if successfully delivered via WebSocket
    async fn try_websocket_delivery<T: ActivityData>(
        state: &AppState,
        activity: &T,
        did: DeviceId,
    ) -> bool {
        // Check if the recipient device is online
        if let Some(sender) = state.sockets.get(&did) {
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

    /// TODO Deliver message to a remote recipient
    async fn deliver_remote(state: &AppState, activity: &Activity) -> Result<(), AppError> {
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
            activity.as_base().to()
        );
        let _ = (state, activity); // Suppress unused warnings for now
        Ok(())
    }
}
