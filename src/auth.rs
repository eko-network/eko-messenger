use axum::{Json, extract::State};
use redis::aio::MultiplexedConnection;
use serde::{Deserialize, Serialize};

use crate::{AppState, errors::AppError};
#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
    device_name: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    user_id: String,
    access_token: String,
    device_id: String,
}

pub trait IdentityProvider {
    async fn login_with_email(&self, email: String, password: String) -> Result<String, AppError>;
}

pub struct Auth<T: IdentityProvider> {
    provider: T,
    redis: MultiplexedConnection,
}

impl<T: IdentityProvider> Auth<T> {
    pub fn new(provider: T, redis: MultiplexedConnection) -> Self {
        Self {
            provider: provider,
            redis,
        }
    }
    async fn login(&self, req: LoginRequest) -> Result<Json<LoginResponse>, AppError> {
        let uid = self
            .provider
            .login_with_email(req.email, req.password)
            .await?;

        let response = LoginResponse {
            user_id: uid,
            access_token: "<placeholder_access_token>".to_string(),
            device_id: "<placeholder_device_id>".to_string(),
        };

        Ok(Json(response))
    }
}

pub async fn login_handler(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    state.auth.login(req).await
}
