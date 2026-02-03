use crate::{
    AppState, activitypub::types::activity::Activity, auth::Claims, devices::DeviceId,
    errors::AppError, messaging::MessagingService,
};
use axum::{
    Json, debug_handler,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

pub const KEY_COLLECTION_URL: &str = "/keyCollection";
#[debug_handler]
pub async fn post_to_outbox(
    State(state): State<AppState>,
    Path(uid): Path<String>,
    Extension(claims): Extension<Arc<Claims>>,
    Json(mut payload): Json<Activity>,
) -> Result<impl IntoResponse, AppError> {
    // Verify the authenticated user matches the outbox owner
    if claims.sub != uid {
        return Err(AppError::Forbidden(
            "Cannot post to another user's outbox".to_string(),
        ));
    }

    if claims.sub != *payload.as_base().actor() {
        info!(
            "{} tried to send a message as {}",
            claims.sub,
            payload.as_base().actor()
        );
        return Err(AppError::Forbidden(
            "Messages may not be sent on behalf of other users".into(),
        ));
    }

    if let Activity::Create(create) = &mut payload {
        if claims.sub != create.object.attributed_to {
            return Err(AppError::Forbidden(
                "Messages may not be sent on behalf of other users".into(),
            ));
        }
        // The message is valid, so we assign id to the inner
        let message_id = format!("https://{}/messages/{}", state.domain, Uuid::new_v4());
        create.object.id = Some(message_id);
    }
    // all activities get an ID
    let activity_id = format!("https://{}/activities/{}", state.domain, Uuid::new_v4());
    payload.as_base_mut().set_id(activity_id);

    if let Activity::Take(take) = &mut payload {
        if !take.to.ends_with(KEY_COLLECTION_URL) {
            return Err(AppError::BadRequest("Invalid target URL".into()));
        }

        let device_url = take.to.trim_end_matches(KEY_COLLECTION_URL);
        let target_did = DeviceId::from_url(device_url)?;
        let bundle = state
            .storage
            .devices
            .get_prekey_bundle(target_did)
            .await?
            .ok_or_else(|| {
                AppError::NotFound("PreKey bundle not available for this device".into())
            })?;
        take.result = Some(bundle);
    }

    MessagingService::process_outgoing_message(&state, &payload, &claims.did).await?;
    Ok((StatusCode::CREATED, Json(payload)).into_response())
}
