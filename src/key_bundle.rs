use crate::{AppState, errors::AppError, types::PreKeyBundle};
use axum::{
    Json,
    extract::{Path, State},
};

pub async fn get_bundle(
    State(state): State<AppState>,
    Path(uid): Path<String>,
) -> Result<Json<Vec<PreKeyBundle>>, AppError> {
    let bundles = state
        .storage
        .devices
        .key_bundles_for_user(&uid)
        .await?;

    Ok(Json(bundles))
}
