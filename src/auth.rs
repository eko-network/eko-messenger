use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
};
use axum_client_ip::ClientIp;
use axum_extra::{TypedHeader, headers::UserAgent};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

pub const REFRESH_EXPIRATION: i64 = 60 * 60 * 24 * 31;

use crate::{
    AppState,
    errors::AppError,
    jwt_helper::{Claims, JwtHelper},
};
use jsonwebtoken;
#[derive(Deserialize, Debug)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub device_name: String,
}

#[derive(Serialize, Debug)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
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
    db_pool: PgPool,
    jwt_helper: JwtHelper,
}

impl<T: IdentityProvider> Auth<T> {
    pub fn new(provider: T, db_pool: PgPool) -> Self {
        let jwt_helper = JwtHelper::new_from_env().expect("Could not instantiate JwtHelper");
        Self {
            provider: provider,
            db_pool,
            jwt_helper,
        }
    }

    fn generate_token() -> String {
        Uuid::new_v4().to_string()
    }
    async fn store_new_token(
        &self,
        user_id: &str,
        token_str: &str,
        ip_address: &str,
        device_name: &str,
        user_agent: &str,
    ) -> Result<(), sqlx::Error> {
        let expires_at =
            time::OffsetDateTime::now_utc() + time::Duration::seconds(REFRESH_EXPIRATION);

        sqlx::query!(
            r#"
            INSERT INTO refresh_tokens (token, user_id, ip_address, device_name, user_agent, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            token_str,
            user_id,
            ip_address,
            device_name,
            user_agent,
            expires_at
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    pub async fn rotate_refresh_token(
        &self,
        old_token_str: &str,
        request_ip: &str,
        request_user_agent: &str,
    ) -> Result<Option<(String, String)>, sqlx::Error> {
        let mut tx = self.db_pool.begin().await?;

        let old_token = sqlx::query!(
            "SELECT user_id, user_agent, device_name, expires_at FROM refresh_tokens WHERE token = $1",
            old_token_str
        )
        .fetch_optional(&mut *tx)
        .await?;

        let (user_id, device_name) = match old_token {
            Some(token) => {
                if token.expires_at <= time::OffsetDateTime::now_utc() {
                    sqlx::query!("DELETE FROM refresh_tokens WHERE token = $1", old_token_str)
                        .execute(&mut *tx)
                        .await?;
                    tx.commit().await?;
                    return Ok(None);
                }
                if token.user_agent != request_user_agent {
                    sqlx::query!("DELETE FROM refresh_tokens WHERE token = $1", old_token_str)
                        .execute(&mut *tx)
                        .await?;
                    tx.commit().await?;
                    return Ok(None);
                }
                (token.user_id, token.device_name)
            }
            None => {
                tx.commit().await?;
                return Ok(None);
            }
        };

        sqlx::query!("DELETE FROM refresh_tokens WHERE token = $1", old_token_str)
            .execute(&mut *tx)
            .await?;

        let new_token_str = Self::generate_token();
        let expires_at =
            time::OffsetDateTime::now_utc() + time::Duration::seconds(REFRESH_EXPIRATION);
        sqlx::query!(
            r#"
            INSERT INTO refresh_tokens (token, user_id, ip_address, device_name, user_agent, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            new_token_str,
            user_id,
            request_ip,
            device_name,
            request_user_agent,
            expires_at
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Some((new_token_str, user_id)))
    }

    async fn revoke_token(&self, user_id: &str, token_str: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM refresh_tokens WHERE token = $1 AND user_id = $2",
            token_str,
            user_id
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }
    pub async fn login(
        &self,
        req: LoginRequest,
        ip_address: &str,
        user_agent: &str,
    ) -> Result<Json<LoginResponse>, AppError> {
        let uid = self
            .provider
            .login_with_email(req.email.clone(), req.password)
            .await?;

        let access_token = self
            .jwt_helper
            .create_jwt(&uid)
            .map_err(|e| anyhow::anyhow!(e))?;
        let refresh_token = Self::generate_token();

        self.store_new_token(
            &uid,
            &refresh_token,
            ip_address,
            &req.device_name,
            user_agent,
        )
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

        let response = LoginResponse {
            access_token,
            refresh_token,
            expires_in: REFRESH_EXPIRATION,
        };

        Ok(Json(response))
    }

    pub async fn refresh_token(
        &self,
        old_refresh_token: &str,
        ip_address: &str,
        user_agent: &str,
    ) -> Result<Json<LoginResponse>, AppError> {
        if let Some((refresh_token, user_id)) = self
            .rotate_refresh_token(old_refresh_token, ip_address, user_agent)
            .await
            .map_err(|e| anyhow::anyhow!(e))?
        {
            let new_access_token = self
                .jwt_helper
                .create_jwt(&user_id)
                .map_err(|e| anyhow::anyhow!(e))?;
            let response = LoginResponse {
                access_token: new_access_token,
                expires_in: REFRESH_EXPIRATION,
                refresh_token: refresh_token.clone(),
            };
            println!("{:?} {:?}", old_refresh_token, refresh_token);
            Ok(Json(response))
        } else {
            Err(AppError::Unauthorized("Invalid refresh token".into()))
        }
    }

    pub async fn logout(&self, refresh_token: &str, user_id: &str) -> Result<(), AppError> {
        self.revoke_token(user_id, refresh_token)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(())
    }

    pub fn verify_access_token(&self, token: &str) -> Result<Claims, AppError> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp() as usize;

        let data: jsonwebtoken::TokenData<Claims> = self
            .jwt_helper
            .decrypt_jwt(token)
            .map_err(|e| anyhow::anyhow!(e))?;
        if data.claims.exp < now {
            return Err(AppError::Unauthorized("Token has Expired".to_string()));
        }
        Ok(data.claims)
    }
}
pub async fn login_handler(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    println!("{:?}", req);
    state
        .auth
        .login(req, &ip.to_string(), &user_agent.to_string())
        .await
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

pub async fn refresh_token_handler(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    state
        .auth
        .refresh_token(&req.refresh_token, &ip.to_string(), &user_agent.to_string())
        .await
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

pub async fn logout_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Arc<Claims>>,
    Json(req): Json<LogoutRequest>,
) -> Result<StatusCode, AppError> {
    state.auth.logout(&req.refresh_token, &claims.sub).await?;
    Ok(StatusCode::OK)
}
