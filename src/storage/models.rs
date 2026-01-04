/// Defines the internal system state
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
    pub actor_id: String,
    pub from_did: i32,
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
    pub did: i32,
    pub refresh_token: Uuid,
}

#[derive(Debug, Clone)]
pub struct RotatedRefreshToken {
    pub refresh_token: Uuid,
    pub uid: String,
    pub did: i32,
    pub expires_at: OffsetDateTime,
}
