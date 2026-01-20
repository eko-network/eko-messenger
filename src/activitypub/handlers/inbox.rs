use crate::{
    AppState,
    activitypub::{CreateActivity, NoId, actor_url, types::generate_create},
    auth::Claims,
    errors::AppError,
};
use axum::{
    Json, debug_handler,
    extract::{Extension, State},
};
use std::sync::Arc;
use tracing::info;

#[debug_handler]
pub async fn get_inbox(
    State(state): State<AppState>,
    Extension(claims): Extension<Arc<Claims>>,
) -> Result<Json<Vec<CreateActivity<NoId>>>, AppError> {
    let uid = &claims.sub;
    let did = claims.did;
    let actor_id = actor_url(&state.domain, uid);

    // TODO check to see if the actor url is NOT local
    info!("GET FOR {}, {}", actor_id, did);
    let items = state
        .storage
        .inbox
        .inbox_activities(&actor_id.clone(), did)
        .await?;

    info!("returned {} items to {}", items.len(), uid);

    // TODO probably need to do some checks, and deletions here when sent to the user

    // TODO make this a little less jank

    Ok(Json(
        items
            .into_iter()
            .map(|i| {
                generate_create(
                    actor_id.clone(),
                    i.actor_id.clone(),
                    did,
                    i.from_did,
                    i.content,
                )
            })
            .collect(),
    ))
}
