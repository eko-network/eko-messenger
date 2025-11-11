use std::sync::Arc;

use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};

use crate::{AppState, errors::AppError};

pub async fn auth_middleware(
    State(state): State<AppState>,
    TypedHeader(auth_header): TypedHeader<Authorization<Bearer>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let token = auth_header.token();
    let claims = state.auth.verify_access_token(token)?;
    request.extensions_mut().insert(Arc::new(claims));

    let response = next.run(request).await;

    Ok(response)
}
