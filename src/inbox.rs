use crate::{AppState, errors::AppError, jwt_helper::Claims};
use axum::{
    Json,
    extract::{Extension, State},
};
use serde_json::Value;
use std::sync::Arc;

pub async fn get_inbox(
    State(state): State<AppState>,
    Extension(claims): Extension<Arc<Claims>>,
) -> Result<Json<Vec<Value>>, AppError> {
    // Get user_id from device_id
    let device_id = &claims.sub;
    let user_record = sqlx::query!("SELECT user_id FROM devices WHERE id = $1", device_id)
        .fetch_one(&state.pool)
        .await?;
    let user_id = user_record.user_id;

    // Get actor_id from user_id
    let actor_record = sqlx::query!("SELECT id FROM actors WHERE user_id = $1", user_id)
        .fetch_one(&state.pool)
        .await?;
    let actor_id = actor_record.id;

    // Get activities
    let activities = sqlx::query!(
        r#"
        SELECT a.activity_json
        FROM activities a
        INNER JOIN inbox_entries ie ON a.id = ie.activity_id
        WHERE ie.inbox_actor_id = $1
        ORDER BY a.created_at DESC
        "#,
        actor_id
    )
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|row| row.activity_json)
    .collect();

    Ok(Json(activities))
}

