use crate::{AppState, activitypub::actor_url, errors::AppError, jwt_helper::Claims};
use axum::{
    debug_handler,
    Json,
    extract::{Extension, State},
};
use serde_json::Value;
use std::sync::Arc;

#[debug_handler]
pub async fn get_inbox(
    State(state): State<AppState>,
    Extension(claims): Extension<Arc<Claims>>,
) -> Result<Json<Vec<Value>>, AppError> {
    // Get user_id from device_id
    let uid = &claims.sub;

    // Get actor_id from user_id
    let actor_id = actor_url(&state.domain, uid);

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
