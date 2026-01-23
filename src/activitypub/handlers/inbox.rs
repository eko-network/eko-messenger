use crate::{
    AppState,
    activitypub::{Activity, actor_url, types::generate_create},
    auth::Claims,
    errors::AppError,
};
use axum::{
    Json,
    body::Bytes,
    debug_handler,
    extract::{Extension, State},
    http::{HeaderMap, StatusCode},
};
use std::sync::Arc;
use tracing::info;

/// GET /users/:uid/inbox
/// Get inbox activities for the authenticated user
#[debug_handler]
pub async fn get_inbox(
    State(state): State<AppState>,
    Extension(claims): Extension<Arc<Claims>>,
) -> Result<Json<Vec<Activity>>, AppError> {
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
                    did.to_url(&state.domain),
                    i.from_did,
                    i.content,
                )
            })
            .collect(),
    ))
}

/// POST /users/:uid/inbox
/// TODO Receive federated activities from remote servers
#[debug_handler]
#[allow(dead_code)]
pub async fn post_to_inbox(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, AppError> {
    // FIXME AI generated. Needs to be fixed.

    // Verify HTTP signature from remote server
    // let remote_actor = crate::activitypub::validation::verify_http_signature(
    //     &headers,
    //     "POST",
    //     request_path,
    //     &body,
    // ).await?;

    // Parse the activity
    // let activity: Value = serde_json::from_slice(&body)?;

    // Validate activity structure
    // crate::activitypub::validation::validate_activity(&activity)?;

    // Check if local user exists
    // let recipient_uid = extract_uid_from_path()?;
    // let recipient = state.storage.actors.get_by_uid(&recipient_uid).await?;

    // Route based on activity type
    // match activity.get("type").and_then(|t| t.as_str()) {
    //     Some("Create") => {
    //         // Process incoming message with device validation
    //         crate::activitypub::validation::validate_create_activity(&activity)?;
    //         // Process the message...
    //     }
    //     Some("Follow") => {
    //         // Process follow request
    //     }
    //     Some("Add") => {
    //         // Remote user adding a device
    //     }
    //     Some("Remove") => {
    //         // Remote user removing a device
    //     }
    //     _ => return Err(AppError::BadRequest("Unsupported activity type".into()))
    // }

    let _ = (state, headers, body); // Suppress unused warnings
    tracing::info!("POST to inbox not yet implemented");
    Ok(StatusCode::ACCEPTED)
}
