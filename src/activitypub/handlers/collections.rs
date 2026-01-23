use crate::{
    AppState, activitypub::types::eko_types::DeviceAction, devices::DeviceService, errors::AppError,
};
use axum::{
    Json,
    extract::{Path, State},
};

/// Returns the key bundles for a user
pub async fn get_devices(
    State(state): State<AppState>,
    Path(uid): Path<String>,
) -> Result<Json<Vec<DeviceAction>>, AppError> {
    let actions = DeviceService::get_device_actions_for_user(&state, &uid).await?;
    Ok(Json(actions))
}
