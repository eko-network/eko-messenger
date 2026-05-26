use axum::{
    Json, debug_handler,
    extract::{Path, State},
};
use url::Url;

use crate::{
    errors::AppError,
    server::{DEVICE_KEYS_ENDPOINT, USERS_ENDPOINT, DEVICE_ENDPOINT, MessengerContext},
    types::{collection::Collection, eko_types::Device},
};

#[debug_handler]
pub async fn get_devices(
    State(ctx): State<MessengerContext>,
    Path(uid): Path<String>,
) -> Result<Json<Collection<Device>>, AppError> {
    let base = Url::parse(&ctx.domain)?;
    let path = base.join(&format!("{USERS_ENDPOINT}/{uid}/{DEVICE_ENDPOINT}"))?;
    let fetched_items: Vec<Device> = ctx
        .storage
        .list_devices_for_user(&uid)
        .await?
        .into_iter()
        .map(|v| {
            let id = path.join(&v.did.to_string())?;
            let collection = id.join(DEVICE_KEYS_ENDPOINT)?;
            Ok(Device::new(
                id.to_string(),
                v.did,
                collection.to_string(),
                v.public_key,
            ))
        })
        .collect::<Result<Vec<Device>, url::ParseError>>()?;
    let items = (!fetched_items.is_empty()).then(|| fetched_items);
    Ok(Json(Collection::new(path.to_string(), items)))
}
