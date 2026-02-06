use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use gcp_auth::{Token, TokenProvider};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env::var;
use std::sync::Arc;
use tokio::fs;

use crate::{
    AppState,
    activitypub::{Person, create_person},
    auth::IdentityProvider,
    errors::AppError,
};
use axum::{Json, Router, extract::State, routing::post};
use axum_client_ip::ClientIp;
use axum_extra::{TypedHeader, headers::UserAgent};

pub struct FirebaseAuth {
    client: reqwest::Client,
    domain: Arc<String>,
    project_id: String,
    token_provider: Arc<dyn TokenProvider>,
}

#[derive(Debug, Serialize)]
struct SignInRequest {
    email: String,
    password: String,
    #[serde(rename = "returnSecureToken")]
    return_secure_token: bool,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ApiResponse {
    Success(SuccessResponse),
    Error(ErrorResponse),
}
#[derive(Debug, Deserialize)]
struct SuccessResponse {
    #[serde(rename = "localId")]
    local_id: String,
}
#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: ErrorDetail,
}
#[derive(Debug, Deserialize)]
struct ErrorDetail {
    message: String,
}

async fn get_token(provider: &Arc<dyn TokenProvider>) -> Result<Arc<Token>, gcp_auth::Error> {
    provider
        .token(&[
            "https://www.googleapis.com/auth/datastore",
            "https://www.googleapis.com/auth/identitytoolkit",
        ])
        .await
}

impl FirebaseAuth {
    pub async fn new_from_env(domain: Arc<String>, client: reqwest::Client) -> Result<Self> {
        let service_account_path = var("GOOGLE_APPLICATION_CREDENTIALS")
            .expect("GOOGLE_APPLICATION_CREDENTIALS should be set in enviroment");

        let service_account: Value = serde_json::from_str(
            &fs::read_to_string(&service_account_path)
                .await
                .with_context(|| {
                    format!(
                        "Failed to read Google service account credentials from: {}",
                        service_account_path
                    )
                })?,
        )
        .context("Failed to parse service account JSON")?;

        let project_id: String = service_account
            .pointer("/project_id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow!("project_id not found in service account"))?
            .to_string();
        let provider = gcp_auth::provider().await?;
        Ok(Self {
            domain,
            project_id,
            client,
            token_provider: provider,
        })
    }

    pub async fn login_with_email(
        &self,
        email: String,
        password: String,
    ) -> Result<(Person, String), AppError> {
        let token = get_token(&self.token_provider).await?;
        let url = "https://identitytoolkit.googleapis.com/v1/accounts:signInWithPassword";

        let request_body = SignInRequest {
            email: email,
            password: password,
            return_secure_token: true,
        };

        let response = self
            .client
            .post(url)
            .bearer_auth(token.as_str())
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::InternalError(e.into()))?;

        let status = response.status();
        let api_response = response
            .json::<ApiResponse>()
            .await
            .map_err(|e| AppError::InternalError(e.into()))?;

        match (status.is_success(), api_response) {
            (true, ApiResponse::Success(sign_in)) => {
                let uid = sign_in.local_id;
                Ok((self.person_from_uid(&uid).await?, uid))
            }
            (false, ApiResponse::Error(err)) => Err(AppError::Unauthorized(err.error.message)),
            _ => Err(AppError::InternalError(anyhow!(
                "Unexpected response shape"
            ))),
        }
    }

    pub async fn person_from_uid(&self, uid: &str) -> Result<Person, AppError> {
        let token = get_token(&self.token_provider).await?;
        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/users/{}",
            self.project_id, uid
        );
        let response = self
            .client
            .get(url)
            .bearer_auth(token.as_str())
            .send()
            .await?;
        let firestore_response: Value = response.json().await?;
        Ok(create_person(
            &self.domain,
            uid,
            firestore_response
                .pointer("/fields/profileData/mapValue/fields/bio/stringValue")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            firestore_response
                .pointer("/fields/username/stringValue")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or("Unknown".to_string())
                .to_string(),
            firestore_response
                .pointer("/fields/name/stringValue")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            firestore_response
                .pointer("/fields/profileData/mapValue/fields/profilePicture/stringValue")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        ))
    }

    pub async fn uid_from_username(&self, username: &str) -> Result<String, AppError> {
        let token = get_token(&self.token_provider).await?;
        let url = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents:runQuery",
            self.project_id
        );

        let body = serde_json::json!({
            "structuredQuery": {
                "from": [{ "collectionId": "users" }],
                "where": {
                    "fieldFilter": {
                        "field": { "fieldPath": "username" },
                        "op": "EQUAL",
                        "value": { "stringValue": username }
                    }
                },
                "limit": 1
            }
        });
        let response = self
            .client
            .post(url)
            .bearer_auth(token.as_str())
            .json(&body)
            .send()
            .await?;
        let firestore_response: Value = response.json().await?;
        let uid = firestore_response
            .get(0)
            .and_then(|v| v.pointer("/document/name"))
            .and_then(|v| v.as_str())
            .and_then(|v| v.split("/").filter(|seg| !seg.is_empty()).last())
            .ok_or(AppError::NotFound("User Not Found".to_string()))?;
        Ok(uid.to_string())
    }
}

#[async_trait]
impl IdentityProvider for FirebaseAuth {
    async fn person_from_uid(&self, uid: &str) -> Result<Person, AppError> {
        self.person_from_uid(uid).await
    }

    async fn uid_from_username(&self, username: &str) -> Result<String, AppError> {
        self.uid_from_username(username).await
    }
}

pub fn firebase_routes() -> Router<AppState> {
    Router::new().route("/auth/v1/login", post(login_handler))
}

pub async fn login_handler(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    Json(req): Json<crate::auth::LoginRequest>,
) -> Result<Json<crate::auth::LoginResponse>, AppError> {
    let (actor, uid) = state
        .firebase
        .login_with_email(req.email, req.password)
        .await?;

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
