use std::sync::Arc;

use async_trait::async_trait;
use axum::{Json, extract::State, http::StatusCode};
use axum_client_ip::ClientIp;
use axum_extra::{TypedHeader, headers::UserAgent};
use serde::{Deserialize, Serialize};
use serde_with::base64::Base64;
use serde_with::serde_as;
use tracing::info;
use uuid::Uuid;

pub const REFRESH_EXPIRATION: i64 = 60 * 60 * 24 * 31;
pub const JWT_LIFESPAN: time::Duration = time::Duration::minutes(15);

use crate::{
    AppState,
    activitypub::{Person, actor_url},
    auth::jwt::{Claims, JwtHelper},
    errors::AppError,
    storage::Storage,
};
use jsonwebtoken;

#[serde_as]
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreKey {
    pub id: i32,
    #[serde_as(as = "Base64")]
    pub key: Vec<u8>,
}

#[serde_as]
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedPreKey {
    pub id: i32,
    #[serde_as(as = "Base64")]
    pub key: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub signature: Vec<u8>,
}

#[serde_as]
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub device_name: String,
    #[serde_as(as = "Base64")]
    pub identity_key: Vec<u8>,
    pub registration_id: i32,
    pub pre_keys: Vec<PreKey>,
    pub signed_pre_key: SignedPreKey,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub username: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub uid: String,
    pub did: i32,
    pub access_token: String,
    pub refresh_token: Uuid,
    pub expires_at: String,
    pub actor: Person,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: Uuid,
    pub expires_at: String,
}
#[async_trait]
pub trait IdentityProvider: Send + Sync {
    /// Returns actor and uid
    async fn login_with_email(
        &self,
        email: String,
        password: String,
    ) -> Result<(Person, String), AppError>;
    async fn person_from_uid(&self, uid: &str) -> Result<Person, AppError>;
    async fn uid_from_username(&self, username: &str) -> Result<String, AppError>;
    async fn signup(&self, _req: SignupRequest) -> Result<StatusCode, AppError> {
        Err(AppError::BadRequest(
            "Signup is not supported by this provider".to_string(),
        ))
    }
}

pub struct Auth {
    pub provider: Arc<dyn IdentityProvider>,
    storage: Arc<Storage>,
    jwt_helper: JwtHelper,
}

impl Auth {
    pub fn new<P: IdentityProvider + 'static>(provider: P, storage: Arc<Storage>) -> Self {
        let jwt_helper = JwtHelper::new_from_env().expect("Could not instantiate JwtHelper");
        Self {
            provider: Arc::new(provider),
            storage,
            jwt_helper,
        }
    }

    pub async fn login(
        &self,
        req: LoginRequest,
        ip_address: &str,
        user_agent: &str,
        domain: &str,
    ) -> Result<Json<LoginResponse>, AppError> {
        let (actor, uid) = self
            .provider
            .login_with_email(req.email.clone(), req.password.clone())
            .await?;
        let expires_at =
            time::OffsetDateTime::now_utc() + time::Duration::seconds(REFRESH_EXPIRATION);
        let register = self
            .storage
            .devices
            .register_device(
                &uid,
                &req.device_name,
                &req.identity_key,
                req.registration_id,
                &req.pre_keys,
                &req.signed_pre_key,
                ip_address,
                user_agent,
                expires_at,
            )
            .await?;

        let actor_id = actor_url(domain, &uid);
        let inbox_url = format!("{}/inbox", actor_id);
        let outbox_url = format!("{}/outbox", actor_id);
        self.storage
            .actors
            .upsert_local_actor(&actor_id, &inbox_url, &outbox_url)
            .await?;

        let access_token = self
            .jwt_helper
            .create_jwt(&uid, register.did)
            .map_err(|e| anyhow::anyhow!(e))?;

        let expires_at = time::OffsetDateTime::now_utc() + JWT_LIFESPAN;

        let response = LoginResponse {
            uid: uid.clone(),
            did: register.did,
            access_token,
            refresh_token: register.refresh_token,
            expires_at: expires_at.format(&time::format_description::well_known::Rfc3339)?,
            actor,
        };

        Ok(Json(response))
    }

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
                let expires_at = time::OffsetDateTime::now_utc() + JWT_LIFESPAN;
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

    pub async fn logout(&self, refresh_token: &Uuid) -> Result<(), AppError> {
        self.storage.devices.logout_device(refresh_token).await
    }

    pub async fn signup(&self, req: SignupRequest) -> Result<StatusCode, AppError> {
        self.provider.signup(req).await
    }

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
}

pub async fn login_handler(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    state
        .auth
        .login(req, &ip.to_string(), &user_agent.to_string(), &state.domain)
        .await
}

pub async fn signup_handler(
    State(state): State<AppState>,
    Json(req): Json<SignupRequest>,
) -> Result<StatusCode, AppError> {
    state.auth.signup(req).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest {
    pub refresh_token: Uuid,
}

pub async fn refresh_token_handler(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, AppError> {
    info!("refresh token request received");
    state
        .auth
        .refresh_token(&req.refresh_token, &ip.to_string(), &user_agent.to_string())
        .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoutRequest {
    pub refresh_token: Uuid,
}

pub async fn logout_handler(
    State(state): State<AppState>,
    Json(req): Json<LogoutRequest>,
) -> Result<StatusCode, AppError> {
    state.auth.logout(&req.refresh_token).await?;
    Ok(StatusCode::OK)
}
