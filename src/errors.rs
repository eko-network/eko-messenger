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
    InternalError(anyhow::Error),
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
