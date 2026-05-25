/// Defines the internal system state
use crate::devices::DeviceId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::base64::Base64;
use serde_with::serde_as;
use time::OffsetDateTime;
use uuid::Uuid;

#[serde_as]
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreKey {
    pub id: i32,
    #[serde_as(as = "Base64")]
    pub key: Vec<u8>,
}

#[serde_as]
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedPreKey {
    pub id: i32,
    #[serde_as(as = "Base64")]
    pub key: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub signature: Vec<u8>,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRegistration {
    pub device_name: String,
    #[serde_as(as = "Base64")]
    pub identity_key: Vec<u8>,
    pub registration_id: i32,
    pub pre_keys: Vec<PreKey>,
    pub signed_pre_key: SignedPreKey,
    pub user_agent: String,
}

#[derive(Debug, Clone)]
pub struct StoredActivity {
    pub activity: Value,
    pub inbox_actor_id: String,
    // TODO I think we need more information (like device_id)
    pub created_at: OffsetDateTime,
}
#[derive(Debug, Clone)]
pub struct StoredInboxEntry {
    pub id: String,
    pub target_id: String,
    pub actor_id: String,
    pub from_did: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct StoredOutboxActivity {
    pub activity_id: String,
    pub actor_id: String,
    pub activity_type: String,
    pub activity: Value,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct RegisterDeviceResult {
    pub approved: bool,
    pub did: DeviceId,
    pub refresh_token: Uuid,
}

#[derive(Debug, Clone)]
pub struct RotatedRefreshToken {
    pub refresh_token: Uuid,
    pub uid: String,
    pub did: DeviceId,
    pub expires_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct StoredUser {
    pub uid: String,
    pub username: String,
    pub email: String,
    pub oidc_issuer: Option<String>,
    pub oidc_sub: Option<String>,
    pub created_at: OffsetDateTime,
}

/// Opaque encrypted group state blob stored for device synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredGroupState {
    pub id: String,
    pub group_id: Uuid,
    pub user_id: String,
    pub epoch: i64,
    pub encrypted_content: Vec<u8>,
    pub encoding: String,
}
