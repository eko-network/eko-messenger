use std::sync::Arc;

use axum::{
    Extension, Json, debug_handler,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::{Activity, AppError, MessengerContext, RequestAuth};

#[debug_handler]
pub async fn post_to_outbox(
    State(state): State<MessengerContext>,
    Path(uid): Path<String>,
    Extension(claims): Extension<Arc<RequestAuth>>,
    Json(mut payload): Json<Activity>,
) -> Result<impl IntoResponse, AppError> {
    // Verify the authenticated user matches the outbox owner
    if claims.uid != uid {
        return Err(AppError::Forbidden(
            "Cannot post to another user's outbox".to_string(),
        ));
    }

    Ok((StatusCode::CREATED, Json(payload)).into_response())

    // // Extract the UID from the actor URL and compare with the authenticated user
    // let extracted_actor_uid = actor_uid(payload.as_base().actor())?;
    // if claims.sub != extracted_actor_uid {
    //     info!(
    //         "{} tried to send a message as {}",
    //         claims.sub,
    //         payload.as_base().actor()
    //     );
    //     return Err(AppError::Forbidden(
    //         "Messages may not be sent on behalf of other users".into(),
    //     ));
    // }
    //
    // if let Activity::Create(create) = &mut payload {
    //     let attributed_uid = actor_uid(&create.object.attributed_to)?;
    //     if claims.sub != attributed_uid {
    //         return Err(AppError::Forbidden(
    //             "Messages may not be sent on behalf of other users".into(),
    //         ));
    //     }
    //     // The message is valid, so we assign id to the inner
    //     let message_id = format!("{}/messages/{}", state.domain, Uuid::new_v4());
    //     create.object.id = Some(message_id);
    // }
    // // all activities get an ID
    // let activity_id = format!("{}/activities/{}", state.domain, Uuid::new_v4());
    // payload.as_base_mut().set_id(activity_id);
    //
    // if let Activity::Take(take) = &mut payload {
    //     if !take.to.ends_with(KEY_COLLECTION_URL) {
    //         return Err(AppError::BadRequest("Invalid target URL".into()));
    //     }
    //
    //     let device_url = take.to.trim_end_matches(KEY_COLLECTION_URL);
    //     let target_did = DeviceId::from_url(device_url)?;
    //     let bundle = state
    //         .storage
    //         .devices
    //         .get_prekey_bundle(target_did)
    //         .await?
    //         .ok_or_else(|| {
    //             AppError::NotFound("PreKey bundle not available for this device".into())
    //         })?;
    //     take.result = Some(bundle);
    // }
    //
    // MessagingService::process_outgoing_message(&state, &payload, &claims.did).await?;
    // Ok((StatusCode::CREATED, Json(payload)).into_response())
}
