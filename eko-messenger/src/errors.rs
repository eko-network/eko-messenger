use axum::{
    http::StatusCode,
    response::{IntoResponse, Json as AxumJson, Response},
};
use serde_json::json;
use tracing::error;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    DevicePending(String),
    InternalError(anyhow::Error),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            AppError::DevicePending(msg) => write!(f, "Device Pending: {}", msg),
            AppError::InternalError(e) => write!(f, "Internal Error: {}", e),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::BadRequest(msg) => {
                error!("Bad request: {}", msg);
                (StatusCode::BAD_REQUEST, msg)
            }
            AppError::Unauthorized(msg) => {
                error!("Unauthorized: {}", msg);
                (StatusCode::UNAUTHORIZED, msg)
            }
            AppError::Forbidden(msg) => {
                error!("Forbidden: {}", msg);
                (StatusCode::FORBIDDEN, msg)
            }
            AppError::NotFound(msg) => {
                error!("Not found: {}", msg);
                (StatusCode::NOT_FOUND, msg)
            }
            AppError::DevicePending(msg) => {
                error!("Device pending approval: {}", msg);
                (StatusCode::FORBIDDEN, msg)
            }
            AppError::InternalError(e) => {
                error!("Internal server error: {:#}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        let body = AxumJson(json!({ "error": error_message }));

        (status, body).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        AppError::InternalError(err.into())
    }
}
