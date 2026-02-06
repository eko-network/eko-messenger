use crate::{
    AppState,
    activitypub::{Person, actor_url},
    auth::{
        LoginResponse, PreKey, RefreshRequest, RefreshResponse, SignedPreKey,
        jwt::{Claims, JwtHelper},
    },
    errors::AppError,
    storage::Storage,
};
use axum::{Json, extract::State};
use axum_client_ip::ClientIp;
use axum_extra::{TypedHeader, headers::UserAgent};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

pub const REFRESH_EXPIRATION: i64 = 60 * 60 * 24 * 31;
pub const JWT_LIFESPAN: time::Duration = time::Duration::minutes(15);

pub struct SessionManager {
    storage: Arc<Storage>,
    jwt_helper: JwtHelper,
    domain: Arc<String>,
}

impl SessionManager {
    pub fn new(domain: Arc<String>, storage: Arc<Storage>) -> anyhow::Result<Self> {
        let jwt_helper = JwtHelper::new_from_env()?;
        Ok(Self {
            storage,
            jwt_helper,
            domain,
        })
    }

    /// Verify JWT access token and return claims
    pub fn verify_access_token(&self, token: &str) -> Result<Claims, AppError> {
        let data = self.jwt_helper.decrypt_jwt(token);
        match data {
            Ok(token_data) => Ok(token_data.claims),
            Err(e) => match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    Err(AppError::Unauthorized("Token has expired".to_string()))
                }
                _ => Err(anyhow::anyhow!(e).into()),
            },
        }
    }

    /// Register device, create actor, and issue tokens
    pub async fn complete_login(
        &self,
        uid: &str,
        actor: Person,
        device_name: &str,
        identity_key: &[u8],
        registration_id: i32,
        pre_keys: &[PreKey],
        signed_pre_key: &SignedPreKey,
        ip_address: &str,
        user_agent: &str,
    ) -> Result<Json<LoginResponse>, AppError> {
        let expires_at = OffsetDateTime::now_utc() + time::Duration::seconds(REFRESH_EXPIRATION);

        // Register device
        let register = self
            .storage
            .devices
            .register_device(
                uid,
                device_name,
                identity_key,
                registration_id,
                pre_keys,
                signed_pre_key,
                ip_address,
                user_agent,
                expires_at,
            )
            .await?;

        // Upsert local actor
        let actor_id = actor_url(&self.domain, uid);
        let inbox_url = format!("{}/inbox", actor_id);
        let outbox_url = format!("{}/outbox", actor_id);
        self.storage
            .actors
            .upsert_local_actor(&actor_id, &inbox_url, &outbox_url)
            .await?;

        // Create access token
        let access_token = self
            .jwt_helper
            .create_jwt(uid, register.did)
            .map_err(|e| anyhow::anyhow!(e))?;

        let expires_at = OffsetDateTime::now_utc() + JWT_LIFESPAN;

        Ok(Json(LoginResponse {
            uid: uid.to_string(),
            did: register.did.to_url(&self.domain),
            access_token,
            refresh_token: register.refresh_token,
            expires_at: expires_at.format(&time::format_description::well_known::Rfc3339)?,
            actor,
        }))
    }

    /// Refresh access token using refresh token
    pub async fn refresh_token(
        &self,
        old_refresh_token: &Uuid,
        ip_address: &str,
        user_agent: &str,
    ) -> Result<Json<RefreshResponse>, AppError> {
        let result = self
            .storage
            .devices
            .rotate_refresh_token(old_refresh_token, ip_address, user_agent)
            .await?;

        match result {
            Some(rotated) => {
                let access_token = self.jwt_helper.create_jwt(&rotated.uid, rotated.did)?;
                let expires_at = OffsetDateTime::now_utc() + JWT_LIFESPAN;
                Ok(Json(RefreshResponse {
                    access_token,
                    refresh_token: rotated.refresh_token,
                    expires_at: expires_at
                        .format(&time::format_description::well_known::Rfc3339)?,
                }))
            }
            None => Err(AppError::Unauthorized("Invalid refresh token".into())),
        }
    }

    /// Logout and invalidate refresh token
    pub async fn logout(&self, refresh_token: &Uuid) -> Result<(), AppError> {
        self.storage.devices.logout_device(refresh_token).await
    }
}

pub async fn refresh_handler(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, AppError> {
    state
        .sessions
        .refresh_token(&req.refresh_token, &ip.to_string(), &user_agent.to_string())
        .await
}
