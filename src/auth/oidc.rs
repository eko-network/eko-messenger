use crate::{
    AppState,
    activitypub::{Person, actor_url, create_person},
    auth::{
        IdentityProvider, LoginResponse, PreKey, SignedPreKey,
        handlers::{JWT_LIFESPAN, REFRESH_EXPIRATION},
        jwt::JwtHelper,
    },
    errors::AppError,
    storage::Storage,
};
use async_trait::async_trait;
use axum::{
    Json,
    extract::{Query, State},
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
    pub name: String,
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
    pub async fn from_env(name: &str, http_client: &reqwest::Client) -> anyhow::Result<Self> {
        let prefix = format!("OIDC_{}", name.to_uppercase());

        let issuer = env::var(format!("{}_ISSUER", prefix))
            .map_err(|_| anyhow::anyhow!("{}_ISSUER not set", prefix))?;
        let client_id = env::var(format!("{}_CLIENT_ID", prefix))
            .map_err(|_| anyhow::anyhow!("{}_CLIENT_ID not set", prefix))?;
        let client_secret = env::var(format!("{}_CLIENT_SECRET", prefix))
            .map_err(|_| anyhow::anyhow!("{}_CLIENT_SECRET not set", prefix))?;
        let redirect_url = env::var(format!("{}_REDIRECT_URL", prefix))
            .map_err(|_| anyhow::anyhow!("{}_REDIRECT_URL not set", prefix))?;

        let issuer_url = IssuerUrl::new(issuer.clone())?;

        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, http_client)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to discover OIDC provider {}: {}", name, e))?;

        Ok(Self {
            name: name.to_string(),
            issuer_url: issuer,
            client_id,
            client_secret,
            redirect_url,
            provider_metadata,
        })
    }
}

pub struct OidcProvider {
    configs: DashMap<String, OidcConfig>,
    http_client: reqwest::Client,
    storage: Arc<Storage>,
    domain: Arc<String>,
    jwt_helper: JwtHelper,
    auth_states: Arc<DashMap<String, AuthState>>,
}

impl OidcProvider {
    pub async fn new_from_env(domain: Arc<String>, storage: Arc<Storage>) -> anyhow::Result<Self> {
        let providers_str =
            env::var("OIDC_PROVIDERS").map_err(|_| anyhow::anyhow!("OIDC_PROVIDERS not set"))?;

        let jwt_helper = JwtHelper::new_from_env()?;
        let configs = DashMap::new();
        let http_client = reqwest::Client::new();

        for name in providers_str.split(',').map(|s| s.trim()) {
            if name.is_empty() {
                continue;
            }

            let config = OidcConfig::from_env(name, &http_client).await?;
            info!("Configured OIDC provider: {}", name);
            configs.insert(name.to_string(), config);
        }

        if configs.is_empty() {
            return Err(anyhow::anyhow!("No OIDC providers configured"));
        }

        Ok(Self {
            configs,
            http_client,
            storage,
            domain,
            jwt_helper,
            auth_states: Arc::new(DashMap::new()),
        })
    }

    pub fn get_config(&self, name: &str) -> Option<OidcConfig> {
        self.configs.get(name).map(|c| c.clone())
    }

    pub fn provider_names(&self) -> Vec<String> {
        self.configs.iter().map(|e| e.key().clone()).collect()
    }

    pub fn start_auth(&self, provider_name: &str) -> Result<(String, CsrfToken, Nonce), AppError> {
        let config = self.configs.get(provider_name).ok_or_else(|| {
            AppError::NotFound(format!("Unknown OIDC provider: {}", provider_name))
        })?;

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
        provider_name: &str,
        code: &str,
        csrf_token: &str,
    ) -> Result<(String, String), AppError> {
        // Verify CSRF token and retrieve stored nonce
        let expected_nonce = self.verify_csrf_token(csrf_token)?;

        let config = self.configs.get(provider_name).ok_or_else(|| {
            AppError::NotFound(format!("Unknown OIDC provider: {}", provider_name))
        })?;

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
            .request_async(&self.http_client)
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
        provider_name: &str,
        email: &str,
        oidc_sub: &str,
    ) -> Result<(String, String), AppError> {
        let config = self.get_config(provider_name).ok_or_else(|| {
            AppError::NotFound(format!("Unknown OIDC provider: {}", provider_name))
        })?;

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

    pub fn create_verification_token(
        &self,
        provider_name: &str,
        email: &str,
        uid: &str,
    ) -> Result<String, AppError> {
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
            provider: provider_name.to_string(),
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

    pub async fn complete_login(
        &self,
        verification_token: &str,
        device_name: &str,
        identity_key: &[u8],
        registration_id: i32,
        pre_keys: &[PreKey],
        signed_pre_key: &SignedPreKey,
        ip_address: &str,
        user_agent: &str,
    ) -> Result<LoginResponse, AppError> {
        let (_provider, _email, uid) = self.verify_verification_token(verification_token)?;

        // Get user info
        let user = self
            .storage
            .users
            .get_user_by_uid(&uid)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let expires_at =
            time::OffsetDateTime::now_utc() + time::Duration::seconds(REFRESH_EXPIRATION);

        // Register device
        let register = self
            .storage
            .devices
            .register_device(
                &uid,
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
        let actor_id = actor_url(&self.domain, &uid);
        let inbox_url = format!("{}/inbox", actor_id);
        let outbox_url = format!("{}/outbox", actor_id);
        self.storage
            .actors
            .upsert_local_actor(&actor_id, &inbox_url, &outbox_url)
            .await?;

        // Create access token
        let access_token = self
            .jwt_helper
            .create_jwt(&uid, register.did)
            .map_err(|e| AppError::InternalError(anyhow::anyhow!(e)))?;

        let expires_at = time::OffsetDateTime::now_utc() + JWT_LIFESPAN;

        // Create actor
        let actor = create_person(
            &self.domain,
            &user.uid,
            None,
            user.username.clone(),
            None,
            None,
        );

        Ok(LoginResponse {
            uid: uid.clone(),
            did: register.did.to_url(&self.domain),
            access_token,
            refresh_token: register.refresh_token,
            expires_at: expires_at.format(&time::format_description::well_known::Rfc3339)?,
            actor,
        })
    }
}

pub struct OidcIdentityProvider {
    provider: Arc<OidcProvider>,
    domain: Arc<String>,
}

impl OidcIdentityProvider {
    pub fn new(domain: Arc<String>, provider: Arc<OidcProvider>) -> Self {
        Self { provider, domain }
    }
}

#[async_trait]
impl IdentityProvider for OidcIdentityProvider {
    async fn login_with_email(
        &self,
        _email: String,
        _password: String,
    ) -> Result<(Person, String), AppError> {
        Err(AppError::BadRequest(
            "OIDC authentication requires using the /auth/v1/oidc/login endpoint".to_string(),
        ))
    }

    async fn person_from_uid(&self, uid: &str) -> Result<Person, AppError> {
        let user = self
            .provider
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

    async fn uid_from_username(&self, username: &str) -> Result<String, AppError> {
        let user = self
            .provider
            .storage
            .users
            .get_user_by_username(username)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        Ok(user.uid)
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcProvidersResponse {
    pub providers: Vec<String>,
}

pub async fn oidc_providers_handler(
    State(state): State<AppState>,
) -> Result<Json<OidcProvidersResponse>, AppError> {
    let oidc = state
        .oidc_provider
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("OIDC is not configured".to_string()))?;

    Ok(Json(OidcProvidersResponse {
        providers: oidc.provider_names(),
    }))
}

pub async fn oidc_login_handler(
    State(state): State<AppState>,
    axum::extract::Path(provider): axum::extract::Path<String>,
) -> Result<Json<OidcLoginResponse>, AppError> {
    let oidc = state
        .oidc_provider
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("OIDC is not configured".to_string()))?;

    let (login_url, csrf_token, _nonce) = oidc.start_auth(&provider)?;

    Ok(Json(OidcLoginResponse {
        login_url,
        state: csrf_token.secret().clone(),
    }))
}

pub async fn oidc_callback_handler(
    State(state): State<AppState>,
    axum::extract::Path(provider): axum::extract::Path<String>,
    Query(query): Query<OidcCallbackQuery>,
) -> Result<Json<OidcCallbackResponse>, AppError> {
    let oidc = state
        .oidc_provider
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("OIDC is not configured".to_string()))?;

    // Verify CSRF token is provided
    if query.state.is_empty() {
        return Err(AppError::Unauthorized(
            "CSRF token (state) is required".to_string(),
        ));
    }

    // Exchange code and verify CSRF token + nonce + ID token signature
    let (email, sub) = oidc
        .exchange_code(&provider, &query.code, &query.state)
        .await?;

    let (uid, _username) = oidc.get_or_create_user(&provider, &email, &sub).await?;

    let verification_token = oidc.create_verification_token(&provider, &email, &uid)?;

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
    let oidc = state
        .oidc_provider
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("OIDC is not configured".to_string()))?;

    let response = oidc
        .complete_login(
            &req.verification_token,
            &req.device_name,
            &req.identity_key,
            req.registration_id,
            &req.pre_keys,
            &req.signed_pre_key,
            &ip.to_string(),
            &user_agent.to_string(),
        )
        .await?;

    Ok(Json(response))
}
