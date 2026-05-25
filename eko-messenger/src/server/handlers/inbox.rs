use axum::{Extension, Json, debug_handler, extract::State};
use tracing::info;

use crate::{
    activitypub::{Activity, OrderedCollection, actor_url},
    errors::AppError,
    server::{MessengerContext, RequestAuth},
};

#[debug_handler]
pub async fn get_inbox(
    State(ctx): State<MessengerContext>,
    Extension(auth): Extension<RequestAuth>,
) -> Result<Json<OrderedCollection<Activity>>, AppError> {
    let actor_id = actor_url(&ctx.domain, &auth.uid);
    info!("GET inbox for {}, {}", actor_id, auth.did);

    let items = ctx.storage.inbox_activities(auth.did).await?;
    let inbox_url = format!("{}/inbox", actor_id);
    Ok(Json(OrderedCollection::new(inbox_url, items)))
}
