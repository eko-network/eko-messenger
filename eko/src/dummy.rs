use async_trait::async_trait;
use eko_messenger::{
    activitypub::{Activity, Create, types::eko_types::Device},
    devices::DeviceId,
    errors::AppError,
    storage::{ActivityStore, DeviceStore},
};

#[derive(Default)]
pub struct DummyStorage;

#[async_trait]
impl ActivityStore for DummyStorage {
    async fn inbox_activities(&self, _did: DeviceId) -> Result<Vec<Activity>, AppError> {
        Ok(vec![])
    }

    async fn insert_create(&self, _create: &Create) -> Result<(), AppError> {
        Ok(())
    }

    async fn insert_non_create(
        &self,
        _activity: &Activity,
        _dids: &[DeviceId],
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn delete_delivery(&self, _activity_id: &str, _did: &DeviceId) -> Result<bool, AppError> {
        Ok(false)
    }

    async fn delete_deliveries(
        &self,
        _activity_ids: &[String],
        _did: &DeviceId,
    ) -> Result<u64, AppError> {
        Ok(0)
    }

    async fn claim_first_delivery(&self, _create_id: &str) -> Result<bool, AppError> {
        Ok(false)
    }
}

#[async_trait]
impl DeviceStore for DummyStorage {
    async fn list_devices_for_user(&self, _uid: &str) -> Result<Vec<Device>, AppError> {
        Ok(vec![])
    }
}
