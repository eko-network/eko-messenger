use serde::{Deserialize, Serialize};
use serde_with::base64::Base64;
use serde_with::serde_as;
use uuid::Uuid;

use crate::activitypub::Person;

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

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub uid: String,
    pub did: String,
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest {
    pub refresh_token: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoutRequest {
    pub refresh_token: Uuid,
}
