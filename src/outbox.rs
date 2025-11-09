use crate::{
    AppState,
    activitypub::{CreateActivity, Note},
    errors::AppError,
};
use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct OutboxPayload {
    pub sender_username: String,
    pub recipient_username: String,
    pub content: String,
}

pub async fn post_to_outbox(
    State(state): State<AppState>,
    Json(payload): Json<OutboxPayload>,
) -> Result<StatusCode, AppError> {
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

    let _: () = redis::cmd("LPUSH")
        .arg(outbox_key)
        .arg(activity_json)
        .query_async(&mut con)
        .await?;

    Ok(StatusCode::CREATED)
}
