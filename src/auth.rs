use axum::{Json, extract::State, http::StatusCode};
use axum_client_ip::ClientIp;
use axum_extra::{TypedHeader, headers::UserAgent};
use serde::{Deserialize, Serialize};
use serde_with::base64::Base64;
use serde_with::serde_as;
use sqlx::PgPool;
use uuid::Uuid;

pub const REFRESH_EXPIRATION: i64 = 60 * 60 * 24 * 31;

use crate::{
    AppState,
    errors::AppError,
    jwt_helper::{Claims, JwtHelper},
};
use jsonwebtoken;

#[serde_as]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PreKey {
    pub id: i32,
    #[serde_as(as = "Base64")]
    pub key: Vec<u8>,
}

#[serde_as]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SignedPreKey {
    pub id: i32,
    #[serde_as(as = "Base64")]
    pub key: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub signature: Vec<u8>,
}

#[serde_as]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub device_name: String,
    pub device_id: String,
    #[serde_as(as = "Base64")]
    pub identity_key: Vec<u8>,
    pub registration_id: i32,
    pub pre_keys: Vec<PreKey>,
    pub signed_pre_key: SignedPreKey,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: String,
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

    async fn store_login_data(
        &self,
        user_id: &str,
        req: &LoginRequest,
        ip_address: &str,
        user_agent: &str,
        refresh_token: &str,
        expires_at: &time::OffsetDateTime,
    ) -> Result<(), AppError> {
        let mut tx = self.db_pool.begin().await?;

        // Device
        sqlx::query!(
            r#"
            INSERT INTO devices (id, user_id, name, identity_key, registration_id)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            req.device_id,
            user_id,
            req.device_name,
            req.identity_key,
            req.registration_id
        )
        .execute(&mut *tx)
        .await?;

        // Refresh token
        sqlx::query!(
            r#"
            INSERT INTO refresh_tokens (token, device_id, ip_address, user_agent, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            refresh_token,
            req.device_id,
            ip_address,
            user_agent,
            expires_at
        )
        .execute(&mut *tx)
        .await?;

        // Pre keys
        for pre_key in &req.pre_keys {
            sqlx::query!(
                r#"
                INSERT INTO pre_keys (device_id, key_id, key)
                VALUES ($1, $2, $3)
                "#,
                req.device_id,
                pre_key.id,
                pre_key.key
            )
            .execute(&mut *tx)
            .await?;
        }

        // Signed pre key
        sqlx::query!(
            r#"
            INSERT INTO signed_pre_keys (device_id, key_id, key, signature)
            VALUES ($1, $2, $3, $4)
            "#,
            req.device_id,
            req.signed_pre_key.id,
            req.signed_pre_key.key,
            req.signed_pre_key.signature
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn rotate_refresh_token(
        &self,
        old_token_str: &str,
        request_ip: &str,
        request_user_agent: &str,
    ) -> Result<Option<(String, String, time::OffsetDateTime)>, sqlx::Error> {
        let mut tx = self.db_pool.begin().await?;

        let old_token = sqlx::query!(
            "SELECT device_id, user_agent, expires_at FROM refresh_tokens WHERE token = $1",
            old_token_str
        )
        .fetch_optional(&mut *tx)
        .await?;

        let device_id = match old_token {
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
                token.device_id
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
            INSERT INTO refresh_tokens (token, device_id, ip_address, user_agent, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            new_token_str,
            device_id,
            request_ip,
            request_user_agent,
            expires_at
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Some((new_token_str, device_id, expires_at)))
    }

    pub async fn login(
        &self,
        req: LoginRequest,
        ip_address: &str,
        user_agent: &str,
    ) -> Result<Json<LoginResponse>, AppError> {
        let uid = self
            .provider
            //FIXME pointles clone
            .login_with_email(req.email.clone(), req.password.clone())
            .await?;

        let access_token = self
            .jwt_helper
            .create_jwt(&req.device_id)
            .map_err(|e| anyhow::anyhow!(e))?;
        let refresh_token = Self::generate_token();

        let expires_at =
            time::OffsetDateTime::now_utc() + time::Duration::seconds(REFRESH_EXPIRATION);
        self.store_login_data(
            &uid,
            &req,
            ip_address,
            user_agent,
            &refresh_token,
            &expires_at,
        )
        .await?;

        let response = LoginResponse {
            access_token,
            refresh_token,
            expires_at: expires_at.format(&time::format_description::well_known::Rfc3339)?,
        };

        Ok(Json(response))
    }

    pub async fn refresh_token(
        &self,
        old_refresh_token: &str,
        ip_address: &str,
        user_agent: &str,
    ) -> Result<Json<LoginResponse>, AppError> {
        if let Some((refresh_token, device_id, expires_at)) = self
            .rotate_refresh_token(old_refresh_token, ip_address, user_agent)
            .await
            .map_err(|e| anyhow::anyhow!(e))?
        {
            let new_access_token = self
                .jwt_helper
                .create_jwt(&device_id)
                .map_err(|e| anyhow::anyhow!(e))?;
            let response = LoginResponse {
                access_token: new_access_token,
                expires_at: expires_at.format(&time::format_description::well_known::Rfc3339)?,
                refresh_token: refresh_token.clone(),
            };
            println!("{:?} {:?}", old_refresh_token, refresh_token);
            Ok(Json(response))
        } else {
            Err(AppError::Unauthorized("Invalid refresh token".into()))
        }
    }

    pub async fn logout(&self, refresh_token: &str) -> Result<(), AppError> {
        let mut tx = self.db_pool.begin().await?;

        sqlx::query!(
            r#"
            DELETE FROM devices WHERE id = (SELECT device_id FROM refresh_tokens WHERE token = $1)
            "#,
            refresh_token
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

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
    Json(req): Json<LogoutRequest>,
) -> Result<StatusCode, AppError> {
    state.auth.logout(&req.refresh_token).await?;
    Ok(StatusCode::OK)
}
