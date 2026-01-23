use crate::{AppState, auth::jwt::Claims, errors::AppError};
use axum::{Json, extract::State};
use std::sync::Arc;

/// Handler to get the approval status of the current device
pub async fn get_approval_status_handler(
    State(state): State<AppState>,
    claims: axum::Extension<Arc<Claims>>,
) -> Result<Json<bool>, AppError> {
    let status = state.storage.devices.get_device_status(claims.did).await?;
    Ok(Json(status))
}
