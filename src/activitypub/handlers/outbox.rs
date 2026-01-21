use crate::{
    AppState,
    activitypub::{CreateActivity, EncryptedMessage, NoId, WithId},
    auth::Claims,
    errors::AppError,
    messaging::MessagingService,
    storage::models::StoredOutboxActivity,
};
use axum::{
    Json, debug_handler,
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use std::sync::Arc;
use time::OffsetDateTime;
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

    MessagingService::process_outgoing_message(&state, &payload, &payload.actor).await?;

    Ok((StatusCode::CREATED, Json(payload)).into_response())
}
