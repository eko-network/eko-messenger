/// Defines the internal system state
use crate::devices::DeviceId;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

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
