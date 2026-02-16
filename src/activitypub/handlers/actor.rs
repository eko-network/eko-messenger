use axum::{
    Json,
    extract::{Path, State},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};

use crate::{
    AppState,
    activitypub::{Endpoints, Person},
    errors::AppError,
};

/// Fetch an Actor profile. Public route, but if the requester is the
/// authenticated owner we include the  endpoints
pub async fn actor_handler(
    State(state): State<AppState>,
    Path(uid): Path<String>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
) -> Result<Json<Person>, AppError> {
    let mut actor = state.auth.provider.person_from_uid(&uid).await?;

    // If a valid Bearer token is present and belongs to this actor, attach private endpoints
    if let Some(TypedHeader(auth)) = auth_header {
        if let Ok(claims) = state.auth.verify_access_token(auth.token()) {
            if claims.sub == uid {
                actor.endpoints = Some(Endpoints {
                    groups: format!("{}/users/{}/groups", state.domain, uid),
                });
            }
        }
    }

    Ok(Json(actor))
}
