use crate::{
    AppState,
    activitypub::{Create, EncryptedMessage, actor_url, types::activity::Activity},
    auth::Claims,
    devices::DeviceId,
    errors::AppError,
    messaging::MessagingService,
};
use axum::{
    Json, debug_handler,
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

#[debug_handler]
pub async fn post_to_outbox(
    State(state): State<AppState>,
    Extension(claims): Extension<Arc<Claims>>,
    Json(payload): Json<Activity>,
) -> Result<impl IntoResponse, AppError> {
    match payload {
        Activity::Take(payload) => {
            if !payload.target.ends_with("/keyCollection") {
                return Err(AppError::BadRequest("Invalid target URL".into()));
            }

            let device_url = payload.target.trim_end_matches("/keyCollection");
            let target_did = DeviceId::from_url(device_url)?;
            let bundle = state
                .storage
                .devices
                .get_prekey_bundle(target_did)
                .await?
                .ok_or_else(|| {
                    AppError::NotFound("PreKey bundle not available for this device".into())
                })?;

            Ok((StatusCode::OK, Json(bundle)).into_response())
        }
        Activity::Create(payload) => {
            // TODO: message verification
            if payload.actor != actor_url(&state.domain, &claims.sub) {
                info!("{} sent device as {}", claims.sub, payload.actor);
                return Err(AppError::Forbidden(
                    "Messages may not be sent on behalf of other users".into(),
                ));
            }
            // These are now unused
            let message_id = format!("https://{}/messages/{}", state.domain, Uuid::new_v4());
            let activity_id = format!("https://{}/activities/{}", state.domain, Uuid::new_v4());
            let payload = Create {
                context: payload.context,
                id: Some(activity_id.clone()),
                actor: payload.actor,
                object: EncryptedMessage {
                    context: payload.object.context,
                    type_field: payload.object.type_field,
                    id: Some(message_id),
                    content: payload.object.content,
                    attributed_to: payload.object.attributed_to,
                    to: payload.object.to,
                },
            };
            MessagingService::process_outgoing_message(
                &state,
                &payload,
                &payload.actor,
                &claims.did,
            )
            .await?;

            Ok((StatusCode::CREATED, Json(payload)).into_response())
        }
    }
}
