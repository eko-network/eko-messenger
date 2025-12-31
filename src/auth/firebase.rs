use anyhow::{Result, anyhow};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env::var_os;

use crate::activitypub::{Person, create_person};
use crate::auth::IdentityProvider;
use crate::errors::AppError;
use crate::auth::gcp::get_token;
use async_trait::async_trait;

#[derive(Debug)]
pub struct UserInfo {
    username: String,
    profile_picture: Option<String>,
    summary: Option<String>,
    name: Option<String>,
}

pub struct FirebaseAuth {
    domain: String,
    api_key: String,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct SignInRequest {
    email: String,
    password: String,
    #[serde(rename = "returnSecureToken")]
    return_secure_token: bool,
}

#[derive(Debug, Deserialize)]
struct SignInResponse {
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

impl FirebaseAuth {
    pub fn new_from_env_with_domain(domain: String) -> Result<Self> {
        let api_key = var_os("FIREBASE_API_KEY")
            .expect("FIREBASE_API_KEY not found in enviroment")
            .into_string()
            .map_err(|_| anyhow!("Failed to convert from OsString to String"))?;
        Ok(Self {
            domain,
            api_key,
            client: reqwest::Client::new(),
        })
    }
}

#[async_trait]
impl IdentityProvider for FirebaseAuth {
    async fn login_with_email(
        &self,
        email: String,
        password: String,
    ) -> Result<(Person, String), AppError> {
        let url = format!(
            "https://identitytoolkit.googleapis.com/v1/accounts:signInWithPassword?key={}",
            self.api_key
        );

        let request_body = SignInRequest {
            email: email,
            password: password,
            return_secure_token: true,
        };

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::InternalError(e.into()))?;

        if response.status().is_success() {
            let sign_in_response = response
                .json::<SignInResponse>()
                .await
                .map_err(|e| AppError::InternalError(e.into()))?;
            let uid = sign_in_response.local_id;
            Ok((Self::person_from_uid(self, &uid).await?, uid))
        } else {
            let error_response = response
                .json::<ErrorResponse>()
                .await
                .map_err(|e| AppError::InternalError(e.into()))?;
            Err(AppError::Unauthorized(error_response.error.message))
        }
    }

    async fn person_from_uid(&self, uid: &str) -> Result<Person, AppError> {
        let token = get_token().await?;
        let url = format!(
            "https://firestore.googleapis.com/v1/projects/untitled-2832f/databases/(default)/documents/users/{}",
            uid
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
                .and_then(|v| v.as_str().map(|s| s.to_string())),
            firestore_response
                .pointer("/fields/username/stringValue")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or("Unknown".to_string())
                .to_string(),
            firestore_response
                .pointer("/fields/name/stringValue")
                .and_then(|v| v.as_str().map(|s| s.to_string())),
            firestore_response
                .pointer("/fields/profileData/mapValue/fields/profilePicture/stringValue")
                .and_then(|v| v.as_str().map(|s| s.to_string())),
        ))
    }

    async fn uid_from_username(&self, username: &str) -> Result<String, AppError> {
        let token = get_token().await?;
        let url = "https://firestore.googleapis.com/v1/projects/untitled-2832f/databases/(default)/documents:runQuery";

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
