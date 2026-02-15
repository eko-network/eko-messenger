use std::sync::Arc;

use axum::{
    Json, debug_handler,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState, auth::Claims, errors::AppError, groups::GroupService,
    storage::models::StoredGroupState,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertGroupStateRequest {
    pub epoch: i64,
    pub encrypted_content: Vec<u8>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupStateResponse {
    pub id: String,
    pub group_id: Uuid,
    pub epoch: i64,
    pub encrypted_content: Vec<u8>,
    pub media_type: String,
    pub encoding: String,
}

impl From<StoredGroupState> for GroupStateResponse {
    fn from(s: StoredGroupState) -> Self {
        Self {
            id: s.id,
            group_id: s.group_id,
            epoch: s.epoch,
            encrypted_content: s.encrypted_content,
            media_type: s.media_type,
            encoding: s.encoding,
        }
    }
}

/// PUT /users/{uid}/groups/{group_id}
#[debug_handler]
pub async fn upsert_group_state_handler(
    State(state): State<AppState>,
    Path((uid, group_id)): Path<(String, Uuid)>,
    Extension(claims): Extension<Arc<Claims>>,
    Json(req): Json<UpsertGroupStateRequest>,
) -> Result<impl IntoResponse, AppError> {
    if claims.sub != uid {
        return Err(AppError::Forbidden(
            "Cannot modify another user's group state".to_string(),
        ));
    }

    let written =
        GroupService::upsert_group_state(&state, &uid, group_id, req.epoch, req.encrypted_content)
            .await?;

    if written {
        Ok(StatusCode::OK)
    } else {
        Err(AppError::BadRequest(
            "Epoch must be higher than the stored epoch".to_string(),
        ))
    }
}

/// GET /users/{uid}/groups/{group_id}
#[debug_handler]
pub async fn get_group_state_handler(
    State(state): State<AppState>,
    Path((uid, group_id)): Path<(String, Uuid)>,
    Extension(claims): Extension<Arc<Claims>>,
) -> Result<Json<GroupStateResponse>, AppError> {
    if claims.sub != uid {
        return Err(AppError::Forbidden(
            "Cannot read another user's group state".to_string(),
        ));
    }

    let group_state = GroupService::get_group_state(&state, &uid, &group_id).await?;

    match group_state {
        Some(gs) => Ok(Json(gs.into())),
        None => Err(AppError::NotFound("Group state not found".to_string())),
    }
}

/// GET /users/{uid}/groups
#[debug_handler]
pub async fn get_all_group_states_handler(
    State(state): State<AppState>,
    Path(uid): Path<String>,
    Extension(claims): Extension<Arc<Claims>>,
) -> Result<Json<Vec<GroupStateResponse>>, AppError> {
    if claims.sub != uid {
        return Err(AppError::Forbidden(
            "Cannot read another user's group states".to_string(),
        ));
    }

    let states = GroupService::get_all_group_states(&state, &uid).await?;
    Ok(Json(states.into_iter().map(|s| s.into()).collect()))
}

/// DELETE /users/{uid}/groups/{group_id}
#[debug_handler]
pub async fn delete_group_state_handler(
    State(state): State<AppState>,
    Path((uid, group_id)): Path<(String, Uuid)>,
    Extension(claims): Extension<Arc<Claims>>,
) -> Result<impl IntoResponse, AppError> {
    if claims.sub != uid {
        return Err(AppError::Forbidden(
            "Cannot delete another user's group state".to_string(),
        ));
    }

    let deleted = GroupService::delete_group_state(&state, &uid, &group_id).await?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound("Group state not found".to_string()))
    }
}
