use crate::{
    Activity,
    devices::DeviceId,
    errors::AppError,
    storage::models::{StoredDevice, StoredGroupState},
    types::{Create, Delivered},
};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait ActivityStore: Send + Sync {
    /// Returns all of the activities in an actors inbox for a specific device. This has side
    /// affects for `Delivered` causing their corresponding deliver requests to be
    /// removed
    async fn inbox_activities(&self, did: DeviceId) -> Result<Vec<Activity>, AppError>;
    //
    /// Stores a create this should mark the message as needing delivery for all devices in the
    async fn insert_create(&self, create: &Create, devices: &[DeviceId]) -> Result<(), AppError>;

    async fn insert_delivered(
        &self,
        delivered: &Delivered,
        devices: &[DeviceId],
        device: DeviceId,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait OutboxStore: Send + Sync {}

#[async_trait]
pub trait DeviceStore: Send + Sync {
    async fn list_devices_for_user(&self, uid: &str) -> Result<Vec<StoredDevice>, AppError>;
    async fn take_key_package(&self, did: DeviceId) -> Result<Vec<u8>, AppError>;
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

// #[async_trait]
// pub trait NotificationStore: Send + Sync {
//     async fn upsert_endpoint(
//         &self,
//         did: DeviceId,
//         endpoint: &web_push::SubscriptionInfo,
//     ) -> Result<(), AppError>;
//     async fn delete_endpoint(&self, did: DeviceId) -> Result<(), AppError>;
//     async fn retrive_endpoint(
//         &self,
//         dids: DeviceId,
//     ) -> Option<(web_push::SubscriptionInfo, DeviceId)>;
// }

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

#[async_trait]
pub trait GroupStore: Send + Sync {
    /// Upsert encrypted group state. Replaces existing state only if epoch is higher.
    /// Returns true if the state was inserted/updated, false if the epoch was stale.
    async fn upsert_group_state(&self, state: &StoredGroupState) -> Result<bool, AppError>;

    /// Get a single encrypted group state by group_id for a user.
    async fn get_group_state(
        &self,
        user_id: &str,
        group_id: &Uuid,
    ) -> Result<Option<StoredGroupState>, AppError>;

    /// List all encrypted group states for a user.
    async fn get_all_group_states(&self, user_id: &str) -> Result<Vec<StoredGroupState>, AppError>;

    /// Delete an encrypted group state. Returns true if a row was deleted.
    async fn delete_group_state(&self, user_id: &str, group_id: &Uuid) -> Result<bool, AppError>;
}

/// Full messenger storage — implement all sub-traits on your backend type.
pub trait Storage: Send + Sync + DeviceStore + ActivityStore {}

impl<T> Storage for T where T: Send + Sync + DeviceStore + ActivityStore {}
