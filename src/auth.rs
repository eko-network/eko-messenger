use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
};
use axum_client_ip::ClientIp;
use axum_extra::{TypedHeader, headers::UserAgent};
use redis::{AsyncCommands, RedisResult, aio::MultiplexedConnection};
use serde::{Deserialize, Serialize};
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
    redis: MultiplexedConnection,
    jwt_helper: JwtHelper,
}

impl<T: IdentityProvider> Auth<T> {
    pub fn new(provider: T, redis: MultiplexedConnection) -> Self {
        let jwt_helper = JwtHelper::new_from_env().expect("Could not instantiate JwtHelper");
        Self {
            provider: provider,
            redis,
            jwt_helper,
        }
    }

    fn generate_token() -> String {
        Uuid::new_v4().to_string()
    }

    fn token_key(token: &str) -> String {
        format!("rt:{}", token)
    }

    fn user_key(user_id: &str) -> String {
        format!("user_rt:{}", user_id)
    }
    async fn store_new_token(
        &self,
        user_id: &str,
        token_str: &str,
        ip_address: &str,
        device_name: &str,
        user_agent: &str,
    ) -> RedisResult<()> {
        let token_key = Self::token_key(token_str);
        let user_key = Self::user_key(user_id);

        let metadata = &[
            ("userId", user_id),
            ("ip", ip_address),
            ("device_name", device_name),
            ("user_agent", user_agent),
            ("issuedAt", &chrono::Utc::now().to_rfc3339()),
        ];

        let mut con = self.redis.clone();
        let _: () = redis::pipe()
            .hset_multiple(&token_key, metadata)
            .expire(&token_key, REFRESH_EXPIRATION)
            .sadd(&user_key, &token_key)
            .query_async(&mut con)
            .await?;

        Ok(())
    }

    async fn validate_token(&self, token_str: &str) -> RedisResult<Option<String>> {
        let token_key = Self::token_key(token_str);
        let mut con = self.redis.clone();
        let user_id: Option<String> = con.hget(&token_key, "userId").await?;
        match user_id {
            Some(uid) => {
                let user_key = Self::user_key(&uid);
                let is_in_set: bool = con.sismember(&user_key, &token_key).await?;
                if is_in_set {
                    Ok(Some(uid))
                } else {
                    // Token found, but invalid
                    let _: () = con.del(&token_key).await?;
                    Ok(None)
                }
            }
            None => {
                // No tekon found
                Ok(None)
            }
        }
    }

    pub async fn rotate_refresh_token(
        &self,
        old_token_str: &str,
        request_ip: &str,
        request_user_agent: &str,
    ) -> RedisResult<Option<(String, String)>> {
        let mut con = self.redis.clone();
        let user_id = match self.validate_token(old_token_str).await? {
            Some(uid) => uid,
            None => return Ok(None), // Token is invalid, expired, or already revoked
        };

        let old_token_key = Self::token_key(old_token_str);
        let (original_user_agent, device_name): (Option<String>, Option<String>) = con
            .hmget(&old_token_key, &["user_agent", "device_name"])
            .await?;

        if original_user_agent.as_deref() != Some(request_user_agent) {
            self.revoke_token(&user_id, old_token_str).await?;
            return Ok(None);
        }

        let new_token_str = Uuid::new_v4().to_string();

        let new_token_key = Self::token_key(&new_token_str);
        let user_key = Self::user_key(&user_id);

        let metadata = &[
            ("userId", user_id.as_str()),
            ("ip", request_ip),
            ("issuedAt", &chrono::Utc::now().to_rfc3339()),
            ("user_agent", request_user_agent),
            ("device_name", device_name.as_deref().unwrap_or_default()),
        ];

        let _: () = redis::pipe()
            .del(&old_token_key)
            .srem(&user_key, &old_token_key)
            .hset_multiple(&new_token_key, metadata)
            .expire(&new_token_key, REFRESH_EXPIRATION)
            .sadd(&user_key, &new_token_key)
            .query_async(&mut con)
            .await?;

        Ok(Some((new_token_str, user_id)))
    }

    async fn revoke_token(&self, user_id: &str, token_str: &str) -> RedisResult<()> {
        let token_key = Self::token_key(token_str);
        let user_key = Self::user_key(user_id);

        let mut con = self.redis.clone();
        let _: () = redis::pipe()
            .del(&token_key)
            .srem(&user_key, &token_key)
            .query_async(&mut con)
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
        let now = chrono::Utc::now().timestamp() as usize;

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
