use axum::{
    Json, debug_handler,
    extract::MatchedPath,
    extract::{Path, State},
};
use url::Url;

use crate::{
    activitypub::{collection::Collection, eko_types::Device},
    errors::AppError,
    server::MessengerContext,
};

#[debug_handler]
pub async fn get_devices(
    State(ctx): State<MessengerContext>,
    Path(uid): Path<String>,
    path: MatchedPath,
) -> Result<Json<Collection<Device>>, AppError> {
    let path = Url::parse(path.as_str())?;
    let fetched_items: Vec<Device> = ctx
        .storage
        .list_devices_for_user(&uid)
        .await?
        .into_iter()
        .map(|v| {
            let id = path.join(&v.did.to_string())?;
            let collection = id.join("keyCollection")?;
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
