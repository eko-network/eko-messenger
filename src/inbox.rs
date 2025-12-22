use crate::{
    AppState,
    activitypub::actor_url,
    errors::AppError,
    jwt_helper::Claims,
};
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
    let uid = &claims.sub;
    let actor_id = actor_url(&state.domain, uid);

    let activities = state
        .storage
        .inbox
        .inbox_activities(&actor_id)
        .await?;

    Ok(Json(activities))
}

