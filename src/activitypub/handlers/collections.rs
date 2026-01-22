use crate::{AppState, activitypub::PreKeyBundle, devices::DeviceService, errors::AppError};
use axum::{
    Json,
    extract::{Path, State},
};

/// Returns the key bundles for a user
pub async fn get_key_bundles(
    State(state): State<AppState>,
    Path(uid): Path<String>,
) -> Result<Json<Vec<PreKeyBundle>>, AppError> {
    let bundles = DeviceService::get_key_bundles_for_user(&state, &uid).await?;
    Ok(Json(bundles))
}
