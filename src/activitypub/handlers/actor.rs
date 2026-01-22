use crate::{AppState, activitypub::Person, errors::AppError};
use axum::{
    Json,
    extract::{Path, State},
};

pub async fn actor_handler(
    State(state): State<AppState>,
    Path(uid): Path<String>,
) -> Result<Json<Person>, AppError> {
    let actor = state.auth.provider.person_from_uid(&uid).await?;
    Ok(Json(actor))
}
