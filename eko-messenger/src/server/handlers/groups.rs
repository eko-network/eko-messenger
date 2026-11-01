use axum::{
    Extension, Json, debug_handler,
    extract::{Path, State},
    http::StatusCode,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};

use crate::{AppError, MessengerContext, RequestAuth};

#[derive(Debug, Serialize, Deserialize)]
pub struct EpochClaim {
    pub epoch: u64,
}

/// Claims the epoch a group is at for the calling device, which then has a
/// short while to send its commit to it. `group_id` is the MLS group id,
/// base64url without padding. Group ids are random, so only the group's
/// members and the server know them.
#[debug_handler]
pub async fn claim_epoch(
    State(ctx): State<MessengerContext>,
    Path(group_id): Path<String>,
    Extension(claims): Extension<RequestAuth>,
    Json(claim): Json<EpochClaim>,
) -> Result<Json<EpochClaim>, AppError> {
    let group_id = decode_group_id(&group_id)?;
    ctx.storage
        .claim_epoch(&group_id, claim.epoch, claims.did)
        .await?;
    Ok(Json(claim))
}

/// Gives up the calling device's claim on a group's epoch.
#[debug_handler]
pub async fn release_epoch(
    State(ctx): State<MessengerContext>,
    Path(group_id): Path<String>,
    Extension(claims): Extension<RequestAuth>,
) -> Result<StatusCode, AppError> {
    let group_id = decode_group_id(&group_id)?;
    ctx.storage.release_epoch(&group_id, claims.did).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn decode_group_id(group_id: &str) -> Result<Vec<u8>, AppError> {
    URL_SAFE_NO_PAD
        .decode(group_id)
        .map_err(|e| AppError::BadRequest(format!("invalid group id: {e}")))
}
