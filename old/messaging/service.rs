use std::{collections::HashSet, sync::Arc};

use crate::{
    AppState,
    activitypub::{
        Activity,
        handlers::outbox::KEY_COLLECTION_URL,
        types::{activity::CreateView, eko_types::EncryptedMessageView},
    },
    devices::DeviceId,
    errors::AppError,
};
use futures::future::join_all;
use tokio::task::yield_now;
use tracing::warn;

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
                    &crate::activitypub::actor_uid(activity.as_base().to())?,
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
                let message_from_did = from_dids.into_iter().next().unwrap();

                if from_did_url != *message_from_did {
                    return Err(AppError::BadRequest(format!(
                        "Message sender does not match from: ({} != {})",
                        from_did_url, message_from_did
                    )));
                }

                if is_sync_message {
                    fanout.remove(&from_did_url);
                }

                if to_dids.len() != fanout.len() || !to_dids.iter().all(|&id| fanout.contains(id)) {
                    //TODO Reject activity
                    return Err(AppError::BadRequest("device_list_mismatch".into()));
                }

                state.storage.activities.insert_create(create).await?;

                tokio::spawn({
                    // try to delay a little
                    yield_now().await;
                    let state = state.clone();
                    let create = Arc::new(create.clone());
                    async move {
                        let mut futures = Vec::new();
                        for entry in create.object.content.iter() {
                            let state = state.clone();
                            let create = Arc::clone(&create);
                            let entry = entry.clone();

                            futures.push(async move {
                                let activity_view = CreateView {
                                    context: &create.context,
                                    id: create.id.as_deref(),
                                    actor: &create.actor,
                                    object: EncryptedMessageView {
                                        context: &create.object.context,
                                        type_field: &create.object.type_field,
                                        id: create.object.id.as_deref(),
                                        content: std::slice::from_ref(&entry),
                                        attributed_to: &create.object.attributed_to,
                                        to: &create.object.to,
                                    },
                                    to: &create.to,
                                    type_field: "Create",
                                };

                                if let Ok(did) = DeviceId::from_url(&entry.to) {
                                    if !state
                                        .sockets
                                        .try_websocket_delivery(&activity_view, did)
                                        .await
                                        && let Err(e) = state.notification_service.notify(did).await
                                    {
                                        warn!("Tried to notify {} Error: {:?}", entry.to, e);
                                    }
                                } else {
                                    warn!("Tried to notify {}, url malformed", entry.to);
                                }
                            });
                        }
                        join_all(futures).await;
                    }
                });
            }
            Activity::Take(take) => {
                // this re-does compute from prev function (a little bad)
                let device_url = take.to.trim_end_matches(KEY_COLLECTION_URL);
                let target_did = DeviceId::from_url(device_url)?;
                // try to send over socket, if it fails write to db
                if !state
                    .sockets
                    .try_websocket_delivery(activity, target_did)
                    .await
                {
                    state
                        .storage
                        .activities
                        .insert_non_create(activity, &[target_did])
                        .await?;
                }
            }
            Activity::Delivered(delivered) => {
                let is_sync_message = activity.as_base().actor() == activity.as_base().to();

                let fanout = crate::devices::DeviceService::list_device_ids(
                    state,
                    &crate::activitypub::actor_uid(activity.as_base().to())?,
                )
                .await?;

                let create_id = &delivered.object;

                let is_first_delivery = state
                    .storage
                    .activities
                    .claim_first_delivery(create_id)
                    .await?;

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

                // don't sync deliveries to yourself and don't send duplicates for the same message
                if !is_sync_message && is_first_delivery {
                    let mut failed_dids = Vec::new();

                    for device_url in fanout {
                        if let Ok(target_did) = DeviceId::from_url(&device_url) {
                            // Try to send via websocket
                            if !state
                                .sockets
                                .try_websocket_delivery(activity, target_did)
                                .await
                            {
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
