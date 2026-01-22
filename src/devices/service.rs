use crate::{AppState, activitypub::PreKeyBundle, errors::AppError};

/// Service for managing user devices and key bundles
pub struct DeviceService;

impl DeviceService {
    /// Get all key bundles for a user
    pub async fn get_key_bundles_for_user(
        state: &AppState,
        uid: &str,
    ) -> Result<Vec<PreKeyBundle>, AppError> {
        state.storage.devices.key_bundles_for_user(uid).await
    }

    /// List all device IDs for a user
    pub async fn list_device_ids(state: &AppState, uid: &str) -> Result<Vec<String>, AppError> {
        let bundles = Self::get_key_bundles_for_user(state, uid).await?;
        Ok(bundles.iter().map(|b| b.did.clone()).collect())
    }

    // TODO the rest of the device functions like add, remove, validation
}
