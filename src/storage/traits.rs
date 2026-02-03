use crate::{
    activitypub::{Activity, Create, types::eko_types::DeviceAction},
    devices::DeviceId,
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
    /// Returns all of the activities in an actors inbox for a specific device.
    async fn inbox_activities(
        &self,
        inbox_actor_id: &str,
        did: DeviceId,
    ) -> Result<Vec<Activity>, AppError>;

    /// Stores an Activity. If the activity is a deliver it will have a side affect of removing
    /// related creates
    // async fn insert_inbox_entry(&self, entry: Activity) -> Result<(), AppError>;
    /// Stores a create this should mark the message as needing delivery for all devices in the
    /// entries
    async fn insert_create(&self, create: &Create) -> Result<(), AppError>;
    async fn insert_non_create(
        &self,
        activity: &Activity,
        dids: &Vec<DeviceId>,
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
    async fn get_approved_devices(&self, uid: &str) -> Result<Vec<DeviceId>, AppError>;
    async fn device_actions_for_user(&self, uid: &str) -> Result<Vec<DeviceAction>, AppError>;

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

    async fn get_device_status(&self, did: DeviceId) -> Result<bool, AppError>;

    async fn get_prekey_bundle(
        &self,
        did: DeviceId,
    ) -> Result<Option<crate::activitypub::types::eko_types::PreKeyBundle>, AppError>;
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

#[async_trait]
pub trait NotificationStore: Send + Sync {
    async fn upsert_endpoint(
        &self,
        did: DeviceId,
        endpoint: &web_push::SubscriptionInfo,
    ) -> Result<(), AppError>;
    async fn delete_endpoint(&self, did: DeviceId) -> Result<(), AppError>;
    async fn retrive_endpoint(
        &self,
        dids: DeviceId,
    ) -> Result<(web_push::SubscriptionInfo, DeviceId), AppError>;
}

#[async_trait]
pub trait UserStore: Send + Sync {
    async fn get_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<crate::storage::models::StoredUser>, AppError>;

    async fn get_user_by_uid(
        &self,
        uid: &str,
    ) -> Result<Option<crate::storage::models::StoredUser>, AppError>;

    async fn get_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<crate::storage::models::StoredUser>, AppError>;

    async fn get_user_by_oidc(
        &self,
        oidc_issuer: &str,
        oidc_sub: &str,
    ) -> Result<Option<crate::storage::models::StoredUser>, AppError>;

    async fn create_oidc_user(
        &self,
        uid: &str,
        username: &str,
        email: &str,
        oidc_issuer: &str,
        oidc_sub: &str,
    ) -> Result<(), AppError>;
}
