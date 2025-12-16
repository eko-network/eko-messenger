use axum::{Json, extract::State, http::StatusCode};
use axum_client_ip::ClientIp;
use axum_extra::{TypedHeader, headers::UserAgent};
use serde::{Deserialize, Serialize};
use serde_with::base64::Base64;
use serde_with::serde_as;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

pub const REFRESH_EXPIRATION: i64 = 60 * 60 * 24 * 31;

use crate::{
    AppState,
    activitypub::{Person, actor_url, create_person},
    errors::AppError,
    jwt_helper::{Claims, JwtHelper},
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

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub uid: String,
    pub did: i32,
    pub access_token: String,
    pub refresh_token: Uuid,
    pub expires_at: String,
    pub actor: Person,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: Uuid,
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

    /// Returns:
    /// (`did`, `session_id`)
    async fn store_login_data(
        &self,
        uid: &String,
        req: &LoginRequest,
        ip_address: &str,
        user_agent: &str,
        expires_at: &time::OffsetDateTime,
    ) -> Result<(i32, Uuid), AppError> {
        let mut tx = self.db_pool.begin().await?;

        // Device
        let did = sqlx::query!(
            r#"
            INSERT INTO devices (uid, name, identity_key, registration_id)
            VALUES ($1, $2, $3, $4)
            RETURNING did
            "#,
            uid,
            req.device_name,
            req.identity_key,
            req.registration_id
        )
        .fetch_one(&mut *tx)
        .await?
        .did;

        // Refresh token
        let refresh_token = sqlx::query!(
            r#"
            INSERT INTO refresh_tokens (did, ip_address, user_agent, expires_at)
            VALUES ($1, $2, $3, $4)
            RETURNING token
            "#,
            did,
            ip_address,
            user_agent,
            expires_at
        )
        .fetch_one(&mut *tx)
        .await?
        .token;

        // Pre keys
        for pre_key in &req.pre_keys {
            sqlx::query!(
                r#"
                INSERT INTO pre_keys (did, key_id, key)
                VALUES ($1, $2, $3)
                "#,
                did,
                pre_key.id,
                pre_key.key
            )
            .execute(&mut *tx)
            .await?;
        }

        // Signed pre key
        sqlx::query!(
            r#"
            INSERT INTO signed_pre_keys (did, key_id, key, signature)
            VALUES ($1, $2, $3, $4)
            "#,
            did,
            req.signed_pre_key.id,
            req.signed_pre_key.key,
            req.signed_pre_key.signature
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok((did, refresh_token))
    }
    /// Returns:
    /// (`new_token`,`uid`, `did`, `expires_at`)
    pub async fn rotate_refresh_token(
        &self,
        old_token_uid: &Uuid,
        request_ip: &str,
        request_user_agent: &str,
    ) -> Result<Option<(Uuid, String, i32, time::OffsetDateTime)>, sqlx::Error> {
        let mut tx = self.db_pool.begin().await?;

        let old_token = sqlx::query!(
            "SELECT d.did, r.user_agent, r.expires_at, d.uid FROM refresh_tokens AS r JOIN devices AS d ON d.did = r.did WHERE token = $1",
            old_token_uid
        )
        .fetch_optional(&mut *tx)
        .await?;

        let (uid, did) = match old_token {
            Some(token) => {
                sqlx::query!("DELETE FROM refresh_tokens WHERE did = $1", token.did)
                    .execute(&mut *tx)
                    .await?;
                if token.expires_at <= time::OffsetDateTime::now_utc() {
                    tx.commit().await?;
                    return Ok(None);
                }
                if token.user_agent != request_user_agent {
                    tx.commit().await?;
                    return Ok(None);
                }
                (token.uid, token.did)
            }
            None => {
                tx.commit().await?;
                return Ok(None);
            }
        };

        let expires_at =
            time::OffsetDateTime::now_utc() + time::Duration::seconds(REFRESH_EXPIRATION);

        let new_token = sqlx::query!(
            r#"
            INSERT INTO refresh_tokens (did, ip_address, user_agent, expires_at)
            VALUES ($1, $2, $3, $4)
            RETURNING token
            "#,
            did,
            request_ip,
            request_user_agent,
            expires_at
        )
        .fetch_one(&mut *tx)
        .await?
        .token;

        tx.commit().await?;

        Ok(Some((new_token, uid, did, expires_at)))
    }
    pub async fn login(
        &self,
        req: LoginRequest,
        ip_address: &str,
        user_agent: &str,
        domain: &str,
    ) -> Result<Json<LoginResponse>, AppError> {
        let uid = self
            .provider
            //FIXME: pointles clone
            .login_with_email(req.email.clone(), req.password.clone())
            .await?;
        let expires_at =
            time::OffsetDateTime::now_utc() + time::Duration::seconds(REFRESH_EXPIRATION);
        let (did, refresh_token) = self
            .store_login_data(&uid, &req, ip_address, user_agent, &expires_at)
            .await?;
        self.create_actor(&uid, domain).await?;

        let access_token = self
            .jwt_helper
            .create_jwt(&uid, did)
            .map_err(|e| anyhow::anyhow!(e))?;

        let expires_at =
            time::OffsetDateTime::now_utc() + time::Duration::seconds(REFRESH_EXPIRATION);

        let response = LoginResponse {
            uid: uid.clone(),
            did: did,
            access_token,
            refresh_token,
            expires_at: expires_at.format(&time::format_description::well_known::Rfc3339)?,
            actor: create_person(&uid, domain),
        };

        Ok(Json(response))
    }

    async fn create_actor(&self, uid: &str, domain: &str) -> Result<(), AppError> {
        let actor_url = actor_url(domain, uid);
        let inbox_url = format!("{}/inbox", actor_url);
        let outbox_url = format!("{}/outbox", actor_url);

        info!(
            "Creating actor: id={}, inbox_url={}, outbox_url={}",
            actor_url, inbox_url, outbox_url
        );

        sqlx::query!(
            r#"
            INSERT INTO actors (id, is_local, inbox_url, outbox_url)
            VALUES ($1, true, $2, $3)
            ON CONFLICT (id) DO NOTHING
            "#,
            actor_url,
            inbox_url,
            outbox_url,
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }
    pub async fn refresh_token(
        &self,
        old_refresh_token: &Uuid,
        ip_address: &str,
        user_agent: &str,
    ) -> Result<Json<RefreshResponse>, AppError> {
        if let Some((refresh_token, uid, did, expires_at)) = self
            .rotate_refresh_token(old_refresh_token, ip_address, user_agent)
            .await
            .map_err(|e| anyhow::anyhow!(e))?
        {
            let new_access_token = self
                .jwt_helper
                .create_jwt(&uid, did)
                .map_err(|e| anyhow::anyhow!(e))?;

            let response = RefreshResponse {
                access_token: new_access_token,
                expires_at: expires_at.format(&time::format_description::well_known::Rfc3339)?,
                refresh_token: refresh_token.clone(),
            };
            Ok(Json(response))
        } else {
            Err(AppError::Unauthorized("Invalid refresh token".into()))
        }
    }

    pub async fn logout(&self, refresh_token: &Uuid) -> Result<(), AppError> {
        let mut tx = self.db_pool.begin().await?;

        sqlx::query!(
            r#"
            DELETE FROM devices WHERE did = (SELECT did FROM refresh_tokens WHERE token = $1)
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
    state
        .auth
        .login(req, &ip.to_string(), &user_agent.to_string(), &state.domain)
        .await
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
