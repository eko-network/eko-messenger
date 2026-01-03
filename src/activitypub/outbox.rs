use crate::{
    AppState,
    activitypub::{
        CreateActivity, EncryptedMessage, NoId, WithId,
        types::{actor_uid, generate_create},
    },
    auth::Claims,
    errors::AppError,
    storage::models::{StoredInboxEntry, StoredOutboxActivity},
};
use axum::{
    Json, debug_handler,
    extract::{
        Extension, State,
        ws::{Message, Utf8Bytes},
    },
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::{info, warn};
use uuid::Uuid;

#[debug_handler]
pub async fn post_to_outbox(
    State(state): State<AppState>,
    Extension(_claims): Extension<Arc<Claims>>,
    Json(payload): Json<CreateActivity<NoId>>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: message verification
    // These are now unused
    let message_id = format!("https://{}/messages/{}", &state.domain, Uuid::new_v4());
    let activity_id = format!("https://{}/activities/{}", &state.domain, Uuid::new_v4());
    let payload = CreateActivity {
        context: payload.context,
        type_field: payload.type_field,
        id: WithId(activity_id.clone()),
        actor: payload.actor,
        object: EncryptedMessage {
            context: payload.object.context,
            type_field: payload.object.type_field,
            id: WithId(message_id),
            content: payload.object.content,
            attributed_to: payload.object.attributed_to,
            to: payload.object.to,
        },
    };
    let _stored = StoredOutboxActivity {
        activity_id: activity_id.clone(),
        actor_id: payload.actor.clone(),
        activity_type: payload.type_field.clone(),
        activity: json!(payload),
        created_at: OffsetDateTime::now_utc(),
    };

    // TODO: do we verify the outbox activity (make sure it has all the devices, etc) here?
    // or in the insert_inbox_entry function probably instead? so all inserts only succeed if valid?

    //This maybe shouldn't be a loop, group messages are differernt
    for recipient_actor_id in &payload.object.to {
        // If receipient in our server, put entry directly in inbox (can also make a combined function?)
        // if state
        //     .storage
        //     .actors
        //     .is_local_actor(recipient_actor_id)
        //     .await?
        // {
        for entry in &payload.object.content {
            info!("SEND for {}, {}", recipient_actor_id, entry.to);

            // Check if the recipient is online via WebSocket
            if let Some(sender) = state
                .sockets
                .get(&(actor_uid(recipient_actor_id)?, entry.to))
            {
                // Client is online - push directly via WebSocket
                let message = generate_create(
                    recipient_actor_id.clone(),
                    payload.actor.clone(),
                    entry.to,
                    entry.from,
                    entry.content.clone(),
                );

                let message_json = serde_json::to_string(&message)?;

                if let Err(e) = sender.send(Message::Text(Utf8Bytes::from(message_json))) {
                    warn!(
                        "Failed to send to online client {}, falling back to inbox: {}",
                        recipient_actor_id, e
                    );
                    // Fall through to insert in inbox
                }
            }

            // Client is offline or WebSocket send failed, insert into inbox
            state
                .storage
                .inbox
                .insert_inbox_entry(
                    recipient_actor_id,
                    entry.to,
                    StoredInboxEntry {
                        actor_id: payload.actor.clone(),
                        from_did: entry.from,
                        content: entry.content.clone(),
                    },
                )
                .await?;
        }
        // } else {
        //     info!("Forign id {}", recipient_actor_id);
        //     // Save activity
        //     state.storage.outbox.insert_activity(&stored).await?;
        //     // TODO: federate the message. im thinking we need to do the following:
        //     // - Resolve recipient inbox URL (WebFinger/actor fetch), then enqueue some delivery job keyed by (activity_id, target_inbox_url) for retries & idempotency.
        //     // - The delivery worker signs and POSTs the activity to the remote inbox.
        //     // - Need to figure out what we want to do for activity storage. I guess keep it in there until successful response from server?
        // }
    }

    Ok((StatusCode::CREATED, Json(payload)).into_response())
}
