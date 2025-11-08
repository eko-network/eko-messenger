use anyhow;
use axum::{Json, extract::State, http::StatusCode};
use redis::aio::MultiplexedConnection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppState, errors::AppError};

const ACCESS_TOKEN_EXPIRATION_SECONDS: u64 = 60 * 15; // 15 minutes
use crate::errors::AppError;
use crate::AppState;
#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub device_name: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub user_id: String,
    pub access_token: String,
    pub refresh_token: String,
    pub device_id: String,
    pub expires_in: u64,
}

pub trait IdentityProvider {
    fn login_with_email(
        &self,
        email: String,
        password: String,
    ) -> impl std::future::Future<Output = Result<String, AppError>> + Send;
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

    fn generate_token() -> String {
        Uuid::new_v4().to_string()
    }

    async fn store_token(
        &self,
        token: &str,
        user_id: &str,
        device_id: &str,
        expiration_seconds: u64,
    ) -> Result<(), AppError> {
        let mut con = self.redis.clone();
        let key = format!("token:{}", token);
        let _: () = redis::cmd("SET")
            .arg(&key)
            .arg(format!("{}.{}", user_id, device_id))
            .arg("EX")
            .arg(expiration_seconds)
            .query_async(&mut con)
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    async fn delete_token(&self, token: &str) -> Result<(), AppError> {
        let mut con = self.redis.clone();
        let key = format!("token:{}", token);
        let _: () = redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut con)
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    pub async fn verify_token(&self, token: &str) -> Result<Option<(String, String)>, AppError> {
        let mut con = self.redis.clone();
        let key = format!("token:{}", token);
        let result: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut con)
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!(e)))?;

        if let Some(value) = result {
            let parts: Vec<&str> = value.split('.').collect();
            if parts.len() == 2 {
                Ok(Some((parts[0].to_string(), parts[1].to_string())))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub async fn login(&self, req: LoginRequest) -> Result<Json<LoginResponse>, AppError> {
        let uid = self
            .provider
            .login_with_email(req.email, req.password)
            .await?;

        let device_id = req.device_name; // Make this better for real device
        let access_token = Self::generate_token();
        let refresh_token = Self::generate_token();

        self.store_token(
            &access_token,
            &uid,
            &device_id,
            ACCESS_TOKEN_EXPIRATION_SECONDS,
        )
        .await?;

        self.store_token(
            &refresh_token,
            &uid,
            &device_id,
            ACCESS_TOKEN_EXPIRATION_SECONDS * 24 * 7, // 7 days
        )
        .await?;

        let response = LoginResponse {
            user_id: uid,
            access_token,
            refresh_token,
            device_id,
            expires_in: ACCESS_TOKEN_EXPIRATION_SECONDS,
        };

        Ok(Json(response))
    }

    pub async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<Json<RefreshResponse>, AppError> {
        if let Some((user_id, device_id)) = self.verify_token(refresh_token).await? {
            let new_access_token = Self::generate_token();
            self.store_token(
                &new_access_token,
                &user_id,
                &device_id,
                ACCESS_TOKEN_EXPIRATION_SECONDS,
            )
            .await?;
            let response = RefreshResponse {
                access_token: new_access_token,
                expires_in: ACCESS_TOKEN_EXPIRATION_SECONDS,
            };
            Ok(Json(response))
        } else {
            Err(AppError::Unauthorized("Invalid refresh token".into()))
        }
    }

    pub async fn logout(&self, access_token: &str, refresh_token: &str) -> Result<(), AppError> {
        self.delete_token(access_token).await?;
        self.delete_token(refresh_token).await?;
        Ok(())
    }
}

pub async fn login_handler(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    state.auth.login(req).await
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub expires_in: u64,
}

pub async fn refresh_token_handler(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, AppError> {
    state.auth.refresh_token(&req.refresh_token).await
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub access_token: String,
    pub refresh_token: String,
}

pub async fn logout_handler(
    State(state): State<AppState>,
    Json(req): Json<LogoutRequest>,
) -> Result<StatusCode, AppError> {
    state
        .auth
        .logout(&req.access_token, &req.refresh_token)
        .await?;
    Ok(StatusCode::OK)
}
