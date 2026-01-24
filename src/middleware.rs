use std::sync::Arc;

use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};

use crate::{AppState, auth::jwt::Claims, errors::AppError};

pub async fn auth_middleware(
    State(state): State<AppState>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let token = if let Some(TypedHeader(auth_header)) = auth_header {
        auth_header.token().to_string()
    } else {
        return Err(AppError::Unauthorized(
            "Missing Authorization header".to_string(),
        ));
    };

    let claims = state.auth.verify_access_token(&token)?;
    request.extensions_mut().insert(Arc::new(claims));

    let response = next.run(request).await;

    Ok(response)
}

/// Middleware to check if a device is in "active" status
pub async fn require_active_device(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    // Extract claims from the request extensions (set by auth_middleware)
    let claims = request
        .extensions()
        .get::<Arc<Claims>>()
        .ok_or_else(|| AppError::Unauthorized("No claims found in request".to_string()))?;

    let is_approved = state.storage.devices.get_device_status(claims.did).await?;

    if is_approved {
        Ok(next.run(request).await)
    } else {
        Err(AppError::DevicePending(
            "Device is pending approval. Please approve from another device.".to_string(),
        ))
    }
}
