use crate::{AppState, errors::AppError, types::PreKeyBundle};
use axum::{
    Json,
    extract::{Path, State},
};

pub async fn get_bundle(
    State(state): State<AppState>,
    Path(uid): Path<String>,
) -> Result<Json<Vec<PreKeyBundle>>, AppError> {
    let bundles = get_key_bundles_for_user(&state.pool, uid).await?;
    return Ok(Json(bundles));
}
async fn get_key_bundles_for_user(
    pool: &sqlx::PgPool,
    user: String,
) -> Result<Vec<PreKeyBundle>, AppError> {
    // TODO: this should be one transaction
    let devices = sqlx::query!(
        "SELECT did, identity_key, registration_id FROM devices WHERE uid = $1",
        user
    )
    .fetch_all(pool)
    .await?;

    let mut bundles = Vec::new();

    for device in devices {
        let pre_key = sqlx::query!(
            "DELETE FROM pre_keys WHERE did = $1 RETURNING key_id, key",
            device.did
        )
        .fetch_optional(pool)
        .await?;

        let signed_pre_key = sqlx::query!(
            "SELECT key_id, key, signature FROM signed_pre_keys WHERE did = $1",
            device.did
        )
        .fetch_one(pool)
        .await?;

        if let Some(pre_key) = pre_key {
            bundles.push(PreKeyBundle {
                did: device.did,
                identity_key: device.identity_key.clone(),
                registration_id: device.registration_id,
                pre_key_id: pre_key.key_id,
                pre_key: pre_key.key,
                signed_pre_key_id: signed_pre_key.key_id,
                signed_pre_key: signed_pre_key.key,
                signed_pre_key_signature: signed_pre_key.signature,
            });
        }
    }

    Ok(bundles)
}
