use axum::{
    Extension, Json, debug_handler,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use futures::future::join_all;
use tracing::{debug, info};
use uuid::Uuid;

use crate::{
    Activity, AppError, MessengerContext, RequestAuth,
    types::{activities::ActivityBase, actor_uid, objects::ObjectBase},
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
            if create.object.id().is_none() {
                let object_id = format!("{}/objects/{}", ctx.domain, Uuid::new_v4());
                create.object.as_base_mut().set_id(object_id);
            }

            let mut target_devices = Vec::new();
            let mut delivery_futures = Vec::new();
            for recipient_url in create.to() {
                let recipient_uid = actor_uid(recipient_url)?;
                let devices = ctx.storage.list_devices_for_user(&recipient_uid).await?;
                for device in devices {
                    delivery_futures.push(ctx.sockets.try_websocket_delivery(
                        Activity::Create(create.clone()),
                        recipient_uid.clone(),
                        device.did,
                    ));
                    target_devices.push(device.did);
                }
            }
            // NOTE blocking the websocket push until the create activity is stored to remove
            // the edge case where delivered is processed before the activity hits the inbox
            ctx.storage.insert_create(create, &target_devices).await?;
            join_all(delivery_futures).await;
        }
        Activity::Delivered(delivered) => {
            let mut target_devices = Vec::new();
            for recipient_url in delivered.to() {
                let recipient_uid = actor_uid(recipient_url)?;
                let devices = ctx.storage.list_devices_for_user(&recipient_uid).await?;
                for device in devices {
                    target_devices.push(device.did);
                }
            }

            ctx.storage
                .insert_delivered(delivered, &target_devices, claims.did)
                .await?;
        }
    }

    Ok((StatusCode::CREATED, Json(payload)).into_response())
}
