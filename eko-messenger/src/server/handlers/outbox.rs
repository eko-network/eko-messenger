use axum::{
    Extension, Json, debug_handler,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use tracing::{debug, info};
use uuid::Uuid;

use crate::{
    Activity, AppError, DeviceId, MessengerContext, RequestAuth,
    server::DEVICE_KEYS_ENDPOINT,
    types::{
        KeyPackage,
        activities::{ActivityBase, ActivityBaseMut},
        actor_uid,
        objects::{ObjectBase, ObjectBaseMut},
    },
};

#[debug_handler]
pub async fn post_to_outbox(
    State(ctx): State<MessengerContext>,
    Path(uid): Path<String>,
    Extension(claims): Extension<RequestAuth>,
    Json(mut payload): Json<Activity>,
) -> Result<impl IntoResponse, AppError> {
    debug!("{:?}", payload);
    if claims.uid != uid {
        return Err(AppError::Forbidden(
            "Cannot post to another user's outbox".to_string(),
        ));
    }

    let extracted_actor_uid = actor_uid(payload.actor())?;
    if claims.uid != extracted_actor_uid {
        info!(
            "Stopped {} from sending a message as {}",
            claims.uid,
            payload.actor()
        );
        return Err(AppError::Forbidden(
            "Messages may not be sent on behalf of other users".into(),
        ));
    }

    if payload.id().is_none() {
        let activity_id = format!("{}/activities/{}", ctx.domain, Uuid::new_v4());
        payload.as_mut().set_id(activity_id);
    }

    match &mut payload {
        Activity::Create(create) => {
            debug!("Recived Create");
            // Assign an ID to the object if it doesn't have one
            if create.object.id().is_none() {
                let object_id = format!("{}/objects/{}", ctx.domain, Uuid::new_v4());
                create.object.as_base_mut().set_id(object_id);
            }

            // Resolve recipients to devices
            let mut target_devices = Vec::new();
            for recipient_url in create.to() {
                let recipient_uid = actor_uid(recipient_url)?;
                let devices = ctx.storage.list_devices_for_user(&recipient_uid).await?;
                for device in devices {
                    target_devices.push(device.did);
                }
            }

            // Store the create activity
            ctx.storage.insert_create(create, &target_devices).await?;
        }
        Activity::Take(take) => {
            let to = take.to();
            if to.len() != 1 {
                return Err(AppError::BadRequest(
                    "Take activity must have exactly one recipient".into(),
                ));
            }
            let to_url = to.first().unwrap();
            if !to_url.ends_with(DEVICE_KEYS_ENDPOINT) {
                return Err(AppError::BadRequest("Invalid target URL for Take".into()));
            }

            let device_url = to_url
                .strip_suffix(&format!("/{DEVICE_KEYS_ENDPOINT}"))
                .unwrap_or(to_url);
            let target_did = DeviceId::from_url(device_url)?;
            let bytes = ctx.storage.take_key_package(target_did).await?;
            let package = KeyPackage::new(target_did, bytes);
            take.result = Some(package);
        }
        Activity::Delivered(_) => {
            // For now, we don't have specific logic for Delivered in the outbox
            // but we might want to store it or trigger side effects later.
        }
    }

    Ok((StatusCode::CREATED, Json(payload)).into_response())
}
