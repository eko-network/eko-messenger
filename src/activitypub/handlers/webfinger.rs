use crate::{AppState, activitypub::actor_url, errors::AppError, server_address};
use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct WebFingerQuery {
    resource: String,
}

pub async fn webfinger_handler(
    State(state): State<AppState>,
    Query(query): Query<WebFingerQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let resource = query.resource;
    if !resource.starts_with("acct:") {
        return Err(AppError::BadRequest("Invalid resource format".to_string()));
    }

    let parts: Vec<&str> = resource.trim_start_matches("acct:").split('@').collect();
    if parts.len() != 2 {
        return Err(AppError::BadRequest("Invalid resource format".to_string()));
    }
    let username = parts[0];
    let domain = parts[1];

    let trimmed_domain = server_address()
        .trim_start_matches("http://")
        .trim_start_matches("https://");

    if domain != trimmed_domain {
        return Err(AppError::NotFound(format!(
            "User not found on this domain. Requested {}, expected {}",
            domain, trimmed_domain
        )));
    }

    let uid = state.auth.provider.uid_from_username(username).await?;

    let actor_url = actor_url(&uid);

    let jrd = serde_json::json!({
        "subject": resource,
        "links": [
            {
                "rel": "self",
                "type": "application/activity+json",
                "href": actor_url,
            }
        ]
    });

    Ok(Json(jrd))
}
