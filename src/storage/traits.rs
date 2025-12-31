use crate::{
    activitypub::PreKeyBundle,
    errors::AppError,
    storage::models::{
        RegisterDeviceResult, RotatedRefreshToken, StoredInboxEntry, StoredOutboxActivity,
    },
};
/// Defines the interface to store and get information
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait InboxStore: Send + Sync {
    /// Returns all of the activities in an actors inbox
    async fn inbox_activities(
        &self,
        inbox_actor_id: &str,
        did: i32,
    ) -> Result<Vec<StoredInboxEntry>, AppError>;

    /// Links an inbox to an existing stored activity.
    async fn insert_inbox_entry(
        &self,
        inbox_actor_id: &str,
        to_did: i32,
        entry: StoredInboxEntry,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait OutboxStore: Send + Sync {
    async fn insert_activity(
        &self,
        activity: &StoredOutboxActivity,
    ) -> Result<(), crate::errors::AppError>;
}

#[async_trait]
pub trait DeviceStore: Send + Sync {
    async fn key_bundles_for_user(&self, uid: &str) -> Result<Vec<PreKeyBundle>, AppError>;

    async fn register_device(
        &self,
        uid: &str,
        device_name: &str,
        identity_key: &[u8],
        registration_id: i32,
        pre_keys: &[crate::auth::PreKey],
        signed_pre_key: &crate::auth::SignedPreKey,
        ip_address: &str,
        user_agent: &str,
        expires_at: time::OffsetDateTime,
    ) -> Result<RegisterDeviceResult, AppError>;

    async fn rotate_refresh_token(
        &self,
        old_token: &Uuid,
        ip_address: &str,
        user_agent: &str,
    ) -> Result<Option<RotatedRefreshToken>, AppError>;

    async fn logout_device(&self, refresh_token: &Uuid) -> Result<(), AppError>;
}

#[async_trait]
pub trait ActorStore: Send + Sync {
    /// Upsert a local actor
    async fn upsert_local_actor(
        &self,
        actor_id: &str,
        inbox_url: &str,
        outbox_url: &str,
    ) -> Result<(), AppError>;

    /// Returns true if the actor exists and is local
    async fn is_local_actor(&self, actor_id: &str) -> Result<bool, AppError>;
}
