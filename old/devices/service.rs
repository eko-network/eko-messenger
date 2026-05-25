use std::collections::HashSet;

use crate::{AppState, activitypub::types::eko_types::DeviceAction, errors::AppError};

/// Service for managing user devices and key bundles
pub struct DeviceService;

impl DeviceService {
    /// Get all device actions for a user
    pub async fn get_device_actions_for_user(
        state: &AppState,
        uid: &str,
    ) -> Result<Vec<DeviceAction>, AppError> {
        state.storage.devices.device_actions_for_user(uid).await
    }

    /// List all device IDs for a user
    pub async fn list_device_ids(state: &AppState, uid: &str) -> Result<HashSet<String>, AppError> {
        let dids = state.storage.devices.get_approved_devices(uid).await?;
        Ok(dids.into_iter().map(|v| v.to_url(&state.domain)).collect())
    }
}
