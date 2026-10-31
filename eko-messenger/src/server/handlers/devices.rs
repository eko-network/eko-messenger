use crate::{
    DeviceId,
    errors::AppError,
    server::{DEVICE_ENDPOINT, DEVICE_KEYS_ENDPOINT, MessengerContext, USERS_ENDPOINT},
    types::{KeyPackage, collection::OrderedCollection},
};
use axum::{
    Json, debug_handler,
    extract::{Path, State},
};
use serde_json::Value;

/// The user's device chain. Clients verify it and only trust the devices it
/// ends with, the server annotates AddDevice entries with where to take the
/// device's key packages from, which is not part of what is signed.
#[debug_handler]
pub async fn get_devices(
    State(ctx): State<MessengerContext>,
    Path(uid): Path<String>,
) -> Result<Json<OrderedCollection<Value>>, AppError> {
    let path = format!(
        "{}/{}/{}/{}",
        &ctx.domain, &USERS_ENDPOINT, &uid, &DEVICE_ENDPOINT
    );
    let mut entries = ctx.storage.device_chain_for_user(&uid).await?;
    for entry in &mut entries {
        let Some(fields) = entry.as_object_mut() else {
            continue;
        };
        if fields.get("type").and_then(Value::as_str) != Some("AddDevice") {
            continue;
        }
        if let Some(did) = fields.get("did").and_then(Value::as_str) {
            let collection = format!("{}/{}/{}", &path, did, &DEVICE_KEYS_ENDPOINT);
            fields.insert("keyCollection".to_string(), Value::String(collection));
        }
    }
    let items = (!entries.is_empty()).then_some(entries);
    Ok(Json(OrderedCollection::new(path, items)))
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
