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
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub struct OutboxPayload {
    pub recipient_username: String,
    pub content: String,
}

pub async fn post_to_outbox(
    State(state): State<AppState>,
    Extension(claims): Extension<Arc<Claims>>,
    Json(payload): Json<OutboxPayload>,
) -> Result<StatusCode, AppError> {
    let mut tx = state.pool.begin().await?;

    // Get user_id from device_id
    let device_id = &claims.sub;
    let user_record = sqlx::query!("SELECT user_id FROM devices WHERE id = $1", device_id)
        .fetch_one(&mut *tx)
        .await?;
    let user_id = user_record.user_id;

    // Get sender's actor_id from user_id
    let sender_actor_record = sqlx::query!(
        "SELECT id, actor_url FROM actors WHERE user_id = $1",
        user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    let sender_actor_id = sender_actor_record.id;
    let sender_actor_url = sender_actor_record.actor_url;

    // Get recipient's actor_id from their username
    // TODO: need to look up where to send if user is on different server
    let recipient_actor_url = format!(
        "https://{}/users/{}",
        &state.domain, &payload.recipient_username
    );
    let recipient_actor_record = sqlx::query!(
        "SELECT id FROM actors WHERE actor_url = $1",
        recipient_actor_url
    )
    .fetch_one(&mut *tx)
    .await?;
    let recipient_actor_id = recipient_actor_record.id;

    let note_id = format!("https://{}/notes/{}", &state.domain, Uuid::new_v4());
    let activity_id_url = format!("https://{}/activities/{}", &state.domain, Uuid::new_v4());

    let note = Note {
        id: note_id,
        type_field: "Note".to_string(),
        content: payload.content,
        attributed_to: sender_actor_url.clone(),
        to: vec![recipient_actor_url.clone()],
    };

    let activity = CreateActivity {
        context: "https://www.w3.org/ns/activitystreams".to_string(),
        id: activity_id_url.clone(),
        type_field: "Create".to_string(),
        actor: sender_actor_url,
        object: note,
    };
    let activity_json = json!(activity);

    let activity_record = sqlx::query!(
        r#"
        INSERT INTO activities (activity_id_url, actor_id, activity_type, activity_json)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
        activity_id_url,
        sender_actor_id,
        "Create",
        activity_json
    )
    .fetch_one(&mut *tx)
    .await?;
    let new_activity_id = activity_record.id;

    // TODO: send to other server if recipient not local
    sqlx::query!(
        r#"
        INSERT INTO inbox_entries (inbox_actor_id, activity_id)
        VALUES ($1, $2)
        "#,
        recipient_actor_id,
        new_activity_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(StatusCode::CREATED)
}

