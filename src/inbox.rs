use crate::{AppState, errors::AppError, jwt_helper::Claims};
use axum::{
    Json,
    extract::{Extension, State},
};
use std::sync::Arc;

pub async fn get_inbox(
    State(state): State<AppState>,
    Extension(claims): Extension<Arc<Claims>>,
) -> Result<Json<Vec<String>>, AppError> {
    let mut con = state.redis.clone();

    let username = &claims.sub;

    let inbox_key = format!("inbox:{}", username);

    let messages: Vec<String> = redis::cmd("lrange")
        .arg(inbox_key)
        .arg(0)
        .arg(-1)
        .query_async(&mut con)
        .await?;

    Ok(Json(messages))
}
