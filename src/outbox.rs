use crate::{
    AppState,
    activitypub::{CreateActivity, Note},
    errors::AppError,
    jwt_helper::Claims,
};
use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct OutboxPayload {
    pub sender_username: String,
    pub recipient_username: String,
    pub content: String,
}

pub async fn post_to_outbox(
    State(state): State<AppState>,
    Extension(claims): Extension<Arc<Claims>>,
    Json(payload): Json<OutboxPayload>,
) -> Result<StatusCode, AppError> {
    if claims.sub != payload.sender_username {
        return Err(AppError::Unauthorized(
            "Sender username does not match token".to_string(),
        ));
    }
    let mut con = state.redis.clone();

    let domain = &state.domain;
    let sender_username = &payload.sender_username;
    let recipient_username = &payload.recipient_username;

    let note_id = format!("http://{}/notes/{}", domain, Uuid::new_v4());
    let activity_id = format!("http://{}/activities/{}", domain, Uuid::new_v4());

    let sender_actor_url = format!("http://{}/users/{}", domain, sender_username);
    let recipient_actor_url = format!("http://{}/users/{}", domain, recipient_username);

    let note = Note {
        id: note_id,
        type_field: "Note".to_string(),
        content: payload.content,
        attributed_to: sender_actor_url.clone(),
        to: vec![recipient_actor_url],
    };

    let activity = CreateActivity {
        context: "https://www.w3.org/ns/activitystreams".to_string(),
        id: activity_id,
        type_field: "Create".to_string(),
        actor: sender_actor_url,
        object: note,
    };

    let outbox_key = format!("outbox:{}", sender_username);
    let activity_json = serde_json::to_string(&activity)?;

    // TODO: outbox needs to be activitystreams ordered collection
    let _: () = redis::cmd("LPUSH")
        .arg(outbox_key)
        .arg(&activity_json)
        .query_async(&mut con)
        .await?;

    // TODO: check if recipient exists on local server
    // TODO: inbox also needs to be activitystreams ordered collection
    let inbox_key = format!("inbox:{}", recipient_username);
    let _: () = redis::cmd("LPUSH")
        .arg(inbox_key)
        .arg(activity_json)
        .query_async(&mut con)
        .await?;

    Ok(StatusCode::CREATED)
}
