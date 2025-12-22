use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;
use crate::{
    types::PreKeyBundle,
    errors::AppError,
};

#[async_trait]
pub trait InboxStore: Send + Sync {
    async fn inbox_activities(
        &self,
        inbox_actor_id: &str,
    ) -> Result<Vec<Value>, AppError>;
}

#[async_trait]
pub trait OutboxStore: Send + Sync {
    async fn insert_local_activity(
        &self,
        activity_id: &str,
        actor_id: &str,
        activity_type: &str,
        activity_json: Value,
        inbox_actor_id: &str,
    ) -> Result<(), crate::errors::AppError>;
}

#[async_trait]
pub trait DeviceStore: Send + Sync {
    async fn key_bundles_for_user(
        &self,
        uid: &str,
    ) -> Result<Vec<PreKeyBundle>, AppError>;

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
    ) -> Result<(i32, Uuid), AppError>;

    async fn rotate_refresh_token(
        &self,
        old_token: &Uuid,
        ip_address: &str,
        user_agent: &str,
    ) -> Result<Option<(Uuid, String, i32, time::OffsetDateTime)>, AppError>;

    async fn logout_device(
        &self,
        refresh_token: &Uuid,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait ActorStore: Send + Sync {
    async fn ensure_local_actor(
        &self,
        actor_id: &str,
        inbox_url: &str,
        outbox_url: &str,
    ) -> Result<(), AppError>;
}
