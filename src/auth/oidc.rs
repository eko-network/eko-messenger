use crate::{
    AppState,
    activitypub::{Person, create_person},
    auth::{IdentityProvider, LoginResponse, PreKey, SignedPreKey},
    errors::AppError,
    storage::Storage,
};
use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{get, post},
};
use axum_client_ip::ClientIp;
use axum_extra::{TypedHeader, headers::UserAgent};
use dashmap::DashMap;
use openidconnect::{
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    RedirectUrl, Scope, TokenResponse,
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
};
use serde::{Deserialize, Serialize};
use serde_with::base64::Base64;
use serde_with::serde_as;
use std::{env, sync::Arc, time::Duration};
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct OidcConfig {
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
    pub provider_metadata: CoreProviderMetadata,
}

#[derive(Debug, Clone)]
struct AuthState {
    csrf_token: String,
    nonce: String,
    created_at: time::OffsetDateTime,
}

impl AuthState {
    fn new(csrf_token: CsrfToken, nonce: Nonce) -> Self {
        Self {
            csrf_token: csrf_token.secret().clone(),
            nonce: nonce.secret().clone(),
            created_at: time::OffsetDateTime::now_utc(),
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        let elapsed = time::OffsetDateTime::now_utc() - self.created_at;
        elapsed > time::Duration::seconds(ttl.as_secs() as i64)
    }
}

impl OidcConfig {
    pub async fn from_env(http_client: &reqwest::Client) -> anyhow::Result<Self> {
        let issuer = env::var("OIDC_ISSUER").map_err(|_| anyhow::anyhow!("OIDC_ISSUER not set"))?;
        if issuer.is_empty() {
            return Err(anyhow::anyhow!("OIDC_ISSUER cannot be empty"));
        }

        let client_id =
            env::var("OIDC_CLIENT_ID").map_err(|_| anyhow::anyhow!("OIDC_CLIENT_ID not set"))?;
        if client_id.is_empty() {
            return Err(anyhow::anyhow!("OIDC_CLIENT_ID cannot be empty"));
        }

        let client_secret = env::var("OIDC_CLIENT_SECRET")
            .map_err(|_| anyhow::anyhow!("OIDC_CLIENT_SECRET not set"))?;
        if client_secret.is_empty() {
            return Err(anyhow::anyhow!("OIDC_CLIENT_SECRET cannot be empty"));
        }

        let redirect_url = env::var("OIDC_REDIRECT_URL")
            .map_err(|_| anyhow::anyhow!("OIDC_REDIRECT_URL not set"))?;
        if redirect_url.is_empty() {
            return Err(anyhow::anyhow!("OIDC_REDIRECT_URL cannot be empty"));
        }

        let issuer_url = IssuerUrl::new(issuer.clone())?;

        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, http_client)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to discover OIDC provider: {}", e))?;

        Ok(Self {
            issuer_url: issuer,
            client_id,
            client_secret,
            redirect_url,
            provider_metadata,
        })
    }
}

pub struct Oidc {
    config: OidcConfig,
    client: reqwest::Client,
    pub storage: Arc<Storage>,
    domain: Arc<String>,
    auth_states: Arc<DashMap<String, AuthState>>,
}

impl Oidc {
    pub async fn new_from_env(
        domain: Arc<String>,
        storage: Arc<Storage>,
        client: reqwest::Client,
    ) -> anyhow::Result<Self> {
        let config = OidcConfig::from_env(&client).await?;
        info!("Configured OIDC provider: {}", config.issuer_url);

        Ok(Self {
            config,
            client,
            storage,
            domain,
            auth_states: Arc::new(DashMap::new()),
        })
    }

    pub fn start_auth(&self) -> Result<(String, CsrfToken, Nonce), AppError> {
        let config = &self.config;

        let client =
            CoreClient::from_provider_metadata(
                config.provider_metadata.clone(),
                ClientId::new(config.client_id.clone()),
                Some(ClientSecret::new(config.client_secret.clone())),
            )
            .set_redirect_uri(RedirectUrl::new(config.redirect_url.clone()).map_err(
                |e| AppError::InternalError(anyhow::anyhow!("Invalid redirect URL: {}", e)),
            )?);

        let (auth_url, csrf_token, nonce) = client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .url();

        // Store CSRF token and nonce for verification
        let state_key = csrf_token.secret().clone();
        let auth_state = AuthState::new(csrf_token.clone(), nonce.clone());
        self.auth_states.insert(state_key, auth_state);

        // Clean up expired states periodically
        self.cleanup_expired_states();

        Ok((auth_url.to_string(), csrf_token, nonce))
    }

    fn cleanup_expired_states(&self) {
        let ttl = Duration::from_secs(15 * 60);
        self.auth_states.retain(|_, state| !state.is_expired(ttl));
    }

    fn verify_csrf_token(&self, csrf_token: &str) -> Result<String, AppError> {
        let state = self
            .auth_states
            .get(csrf_token)
            .ok_or_else(|| AppError::Unauthorized("Invalid or expired CSRF token".to_string()))?;

        let ttl = Duration::from_secs(15 * 60);
        if state.is_expired(ttl) {
            self.auth_states.remove(csrf_token);
            return Err(AppError::Unauthorized("CSRF token has expired".to_string()));
        }

        Ok(state.nonce.clone())
    }

    fn consume_auth_state(&self, csrf_token: &str) -> Result<(), AppError> {
        self.auth_states.remove(csrf_token);
        Ok(())
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        csrf_token: &str,
    ) -> Result<(String, String), AppError> {
        // Verify CSRF token and retrieve stored nonce
        let expected_nonce = self.verify_csrf_token(csrf_token)?;

        let config = &self.config;

        let client_id = ClientId::new(config.client_id.clone());
        let client_secret = ClientSecret::new(config.client_secret.clone());
        let redirect_url = RedirectUrl::new(config.redirect_url.clone())
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("Invalid redirect URL: {}", e)))?;

        let client = CoreClient::from_provider_metadata(
            config.provider_metadata.clone(),
            client_id.clone(),
            Some(client_secret.clone()),
        )
        .set_redirect_uri(redirect_url.clone());

        let token_response = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .map_err(|e| {
                error!("Failed to prepare token exchange: {:?}", e);
                AppError::InternalError(anyhow::anyhow!("Failed to prepare token exchange: {}", e))
            })?
            .request_async(&self.client)
            .await
            .map_err(|e| {
                error!("Token exchange failed: {:?}", e);
                AppError::InternalError(anyhow::anyhow!("Token exchange failed: {}", e))
            })?;

        let id_token = token_response
            .id_token()
            .ok_or_else(|| AppError::InternalError(anyhow::anyhow!("No ID token in response")))?;

        let expected_nonce_obj = Nonce::new(expected_nonce.clone());

        let id_token_claims = id_token
            .claims(&client.id_token_verifier(), &expected_nonce_obj)
            .map_err(|e| {
                error!("ID token verification failed: {:?}", e);
                AppError::Unauthorized(format!("Invalid ID token: {}", e))
            })?;

        if let Some(token_nonce) = id_token_claims.nonce() {
            if token_nonce.secret() != &expected_nonce {
                return Err(AppError::Unauthorized(
                    "Nonce mismatch - possible replay attack".to_string(),
                ));
            }
        } else {
            return Err(AppError::Unauthorized(
                "Nonce missing from ID token".to_string(),
            ));
        }

        // Extract email and subject from verified claims
        let email = id_token_claims
            .email()
            .map(|e| e.to_string())
            .ok_or_else(|| AppError::BadRequest("Email not provided by IdP".to_string()))?;

        let sub = id_token_claims.subject().to_string();

        // Consume the auth state to prevent reuse
        self.consume_auth_state(csrf_token)?;

        Ok((email, sub))
    }

    pub async fn get_or_create_user(
        &self,
        email: &str,
        oidc_sub: &str,
    ) -> Result<(String, String), AppError> {
        let config = &self.config;
        let issuer = &config.issuer_url;

        if let Some(user) = self
            .storage
            .users
            .get_user_by_oidc(issuer, oidc_sub)
            .await?
        {
            return Ok((user.uid, user.username));
        }

        // TODO: allow users to pick username, should that happen here in the flow?
        let username = email.split('@').next().unwrap_or("user").to_string();
        let mut final_username = username.clone();
        let mut counter = 1;
        while self
            .storage
            .users
            .get_user_by_username(&final_username)
            .await?
            .is_some()
        {
            final_username = format!("{}_{}", username, counter);
            counter += 1;
        }

        let uid = Uuid::new_v4().to_string();
        self.storage
            .users
            .create_oidc_user(&uid, &final_username, email, issuer, oidc_sub)
            .await?;

        info!("Created new OIDC user: {} ({})", final_username, uid);
        Ok((uid, final_username))
    }

    pub fn create_verification_token(&self, email: &str, uid: &str) -> Result<String, AppError> {
        use jsonwebtoken::{EncodingKey, Header, encode};

        #[derive(Serialize)]
        struct VerificationClaims {
            provider: String,
            email: String,
            uid: String,
            exp: usize,
            iat: usize,
        }

        let now = time::OffsetDateTime::now_utc();
        let claims = VerificationClaims {
            provider: "oidc".to_string(),
            email: email.to_string(),
            uid: uid.to_string(),
            exp: (now + time::Duration::minutes(10)).unix_timestamp() as usize,
            iat: now.unix_timestamp() as usize,
        };

        let secret = env::var("JWT_SECRET")
            .map_err(|_| AppError::InternalError(anyhow::anyhow!("JWT_SECRET not set")))?;
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Failed to create token: {}", e)))?;

        Ok(token)
    }

    pub fn verify_verification_token(
        &self,
        token: &str,
    ) -> Result<(String, String, String), AppError> {
        use jsonwebtoken::{DecodingKey, Validation, decode};

        #[derive(Deserialize)]
        struct VerificationClaims {
            provider: String,
            email: String,
            uid: String,
        }

        let secret = env::var("JWT_SECRET")
            .map_err(|_| AppError::InternalError(anyhow::anyhow!("JWT_SECRET not set")))?;
        let token_data = decode::<VerificationClaims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| AppError::Unauthorized(format!("Invalid verification token: {}", e)))?;

        Ok((
            token_data.claims.provider,
            token_data.claims.email,
            token_data.claims.uid,
        ))
    }

    pub async fn person_from_uid(&self, uid: &str) -> Result<Person, AppError> {
        let user = self
            .storage
            .users
            .get_user_by_uid(uid)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        Ok(create_person(
            &self.domain,
            &user.uid,
            None,
            user.username,
            None,
            None,
        ))
    }

    pub async fn uid_from_username(&self, username: &str) -> Result<String, AppError> {
        let user = self
            .storage
            .users
            .get_user_by_username(username)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        Ok(user.uid)
    }
}

#[async_trait]
impl IdentityProvider for Oidc {
    async fn person_from_uid(&self, uid: &str) -> Result<Person, AppError> {
        self.person_from_uid(uid).await
    }

    async fn uid_from_username(&self, username: &str) -> Result<String, AppError> {
        self.uid_from_username(username).await
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcLoginResponse {
    pub login_url: String,
    pub state: String,
}

#[derive(Debug, Deserialize)]
pub struct OidcCallbackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcCallbackResponse {
    pub verification_token: String,
    pub email: String,
    pub uid: String,
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcCompleteRequest {
    pub verification_token: String,
    pub device_name: String,
    #[serde_as(as = "Base64")]
    pub identity_key: Vec<u8>,
    pub registration_id: i32,
    pub pre_keys: Vec<PreKey>,
    pub signed_pre_key: SignedPreKey,
}

pub fn oidc_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/v1/oidc/login", get(oidc_login_handler))
        .route("/auth/v1/oidc/callback", get(oidc_callback_handler))
        .route("/auth/v1/oidc/complete", post(oidc_complete_handler))
}

pub async fn oidc_login_handler(
    State(state): State<AppState>,
) -> Result<Json<OidcLoginResponse>, AppError> {
    let oidc = state.auth.as_oidc();
    let (login_url, csrf_token, _nonce) = oidc.start_auth()?;

    Ok(Json(OidcLoginResponse {
        login_url,
        state: csrf_token.secret().clone(),
    }))
}

pub async fn oidc_callback_handler(
    State(state): State<AppState>,
    Query(query): Query<OidcCallbackQuery>,
) -> Result<Json<OidcCallbackResponse>, AppError> {
    // Verify CSRF token is provided
    if query.state.is_empty() {
        return Err(AppError::Unauthorized(
            "CSRF token (state) is required".to_string(),
        ));
    }

    let oidc = state.auth.as_oidc();

    // Exchange code and verify CSRF token + nonce + ID token signature
    let (email, sub) = oidc.exchange_code(&query.code, &query.state).await?;

    let (uid, _username) = oidc.get_or_create_user(&email, &sub).await?;

    let verification_token = oidc.create_verification_token(&email, &uid)?;

    Ok(Json(OidcCallbackResponse {
        verification_token,
        email,
        uid,
    }))
}

pub async fn oidc_complete_handler(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    Json(req): Json<OidcCompleteRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let oidc = state.auth.as_oidc();

    let (_provider, _email, uid) = oidc.verify_verification_token(&req.verification_token)?;

    let user = oidc
        .storage
        .users
        .get_user_by_uid(&uid)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let actor = create_person(&state.domain, &user.uid, None, user.username, None, None);

    state
        .sessions
        .complete_login(
            &uid,
            actor,
            &req.device_name,
            &req.identity_key,
            req.registration_id,
            &req.pre_keys,
            &req.signed_pre_key,
            &ip.to_string(),
            &user_agent.to_string(),
        )
        .await
}
