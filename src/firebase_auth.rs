use anyhow::{Result, anyhow};
use reqwest;
use serde::{Deserialize, Serialize};
use std::env::var_os;

use crate::auth::IdentityProvider;
use crate::errors::AppError;

pub struct FirebaseAuth {
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
    pub fn new_from_env() -> Result<Self> {
        let api_key = var_os("FIREBASE_API_KEY")
            .expect("FIREBASE_API_KEY not found in enviroment")
            .into_string()
            .map_err(|_| anyhow!("Failed to convert from OsString to String"))?;
        Ok(Self {
            api_key,
            client: reqwest::Client::new(),
        })
    }
}

impl IdentityProvider for FirebaseAuth {
    async fn login_with_email(&self, email: String, password: String) -> Result<String, AppError> {
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
            Ok(sign_in_response.local_id)
        } else {
            let error_response = response
                .json::<ErrorResponse>()
                .await
                .map_err(|e| AppError::InternalError(e.into()))?;
            Err(AppError::Unauthorized(error_response.error.message))
        }
    }
}
