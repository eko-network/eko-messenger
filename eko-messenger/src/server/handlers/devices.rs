use axum::{
    Json, debug_handler,
    extract::MatchedPath,
    extract::{Path, State},
};

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
    let fetched_items = ctx.storage.list_devices_for_user(&uid).await?;
    let items = (!fetched_items.is_empty()).then(|| fetched_items);
    Ok(Json(Collection::new(path.as_str().to_string(), items)))
}
