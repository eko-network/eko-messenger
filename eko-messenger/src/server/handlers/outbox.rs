use axum::{
    Extension, Json, debug_handler,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use tracing::{debug, info};

use crate::{
    Activity, AppError, DeviceId, MessengerContext, RequestAuth,
    server::DEVICE_KEYS_ENDPOINT,
    types::{KeyPackage, actor_uid},
};

#[debug_handler]
pub async fn post_to_outbox(
    State(ctx): State<MessengerContext>,
    Path(uid): Path<String>,
    Extension(claims): Extension<RequestAuth>,
    Json(mut payload): Json<Activity>,
) -> Result<impl IntoResponse, AppError> {
    debug!("{:?}", payload);
    // Verify the authenticated user matches the outbox owner
    if claims.uid != uid {
        return Err(AppError::Forbidden(
            "Cannot post to another user's outbox".to_string(),
        ));
    }

    // // Extract the UID from the actor URL and compare with the authenticated user
    let extracted_actor_uid = actor_uid(payload.as_base().actor())?;
    if claims.uid != extracted_actor_uid {
        info!(
            "Stopped {} from sending a message as {}",
            claims.uid,
            payload.as_base().actor()
        );
        return Err(AppError::Forbidden(
            "Messages may not be sent on behalf of other users".into(),
        ));
    }
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
    if let Activity::Take(take) = &mut payload {
        if !take.to.ends_with(DEVICE_KEYS_ENDPOINT) {
            return Err(AppError::BadRequest("Invalid target URL".into()));
        }

        let device_url = take
            .to
            .strip_suffix(&format!("/{DEVICE_KEYS_ENDPOINT}"))
            .unwrap_or(&take.to);
        let target_did = DeviceId::from_url(device_url)?;
        let bytes = ctx.storage.take_key_package(target_did).await?;
        let package = KeyPackage::new(target_did, bytes);
        take.result = Some(package);
    }
    Ok((StatusCode::CREATED, Json(payload)).into_response())
    //
    // MessagingService::process_outgoing_message(&state, &payload, &claims.did).await?;
    // Ok((StatusCode::CREATED, Json(payload)).into_response())
}
