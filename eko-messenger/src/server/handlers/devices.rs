use crate::{
    DeviceId,
    errors::AppError,
    server::{DEVICE_ENDPOINT, DEVICE_KEYS_ENDPOINT, MessengerContext, USERS_ENDPOINT},
    types::{KeyPackage, collection::Collection, eko_types::Device},
};
use axum::{
    Json, debug_handler,
    extract::{Path, State},
};
use tracing::debug;

#[debug_handler]
pub async fn get_devices(
    State(ctx): State<MessengerContext>,
    Path(uid): Path<String>,
) -> Result<Json<Collection<Device>>, AppError> {
    let path = format!(
        "{}/{}/{}/{}",
        &ctx.domain, &USERS_ENDPOINT, &uid, &DEVICE_ENDPOINT
    );
    let fetched_items: Vec<Device> = ctx
        .storage
        .list_devices_for_user(&uid)
        .await?
        .into_iter()
        .map(|v| {
            let id = format!("{}/{}", &path, &v.did.to_string());
            let collection = format!("{}/{}", &id, &DEVICE_KEYS_ENDPOINT);
            debug!("{}", collection.to_string());
            Ok(Device::new(id, v.did, collection, v.public_key))
        })
        .collect::<Result<Vec<Device>, url::ParseError>>()?;
    let items = (!fetched_items.is_empty()).then(|| fetched_items);
    Ok(Json(Collection::new(path.to_string(), items)))
}

#[debug_handler]
pub async fn take_key(
    State(ctx): State<MessengerContext>,
    Path((_uid, did)): Path<(String, DeviceId)>,
) -> Result<Json<KeyPackage>, AppError> {
    let bytes = ctx.storage.take_key_package(did).await?;
    let package = KeyPackage::new(did, bytes);
    Ok(Json(package))
}
