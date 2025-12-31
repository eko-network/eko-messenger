use crate::{
    AppState,
    activitypub::{CreateActivity, EncryptedMessage, EncryptedMessageEntry, NoId, actor_url},
    errors::AppError,
    auth::Claims,
};
use axum::{
    Json, debug_handler,
    extract::{Extension, State},
};
use serde_json::Value;
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
            .map(|i| CreateActivity {
                context: Value::String("placeholder".to_string()),
                type_field: "Create".to_string(),
                id: NoId,
                actor: i.actor_id.clone(),
                object: EncryptedMessage {
                    context: Value::String("placeholder".to_string()),
                    attributed_to: i.actor_id,
                    content: vec![EncryptedMessageEntry {
                        to: did,
                        from: i.from_did,
                        content: i.content,
                    }],
                    id: NoId,
                    to: vec![actor_id.clone()],
                    type_field: "Note".to_string(),
                },
            })
            .collect(),
    ))
}
