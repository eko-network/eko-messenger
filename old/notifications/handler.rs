use std::sync::Arc;

use axum::{Extension, Json, extract::State};
use reqwest::StatusCode;
use web_push::SubscriptionInfo;

use crate::{AppState, auth::Claims, errors::AppError};

pub async fn register_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Arc<Claims>>,
    Json(req): Json<SubscriptionInfo>,
) -> Result<StatusCode, AppError> {
    tracing::info!("Recived Registration for {}", req.endpoint);
    state
        .notification_service
        .register(claims.did, &req)
        .await?;
    Ok(StatusCode::OK)
}
