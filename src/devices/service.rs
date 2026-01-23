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
        let actions = Self::get_device_actions_for_user(state, uid).await?;
        let mut set = HashSet::new();

        for action in actions {
            match action {
                DeviceAction::AddDevice(add_device) => {
                    set.insert(add_device.did);
                }
                DeviceAction::RevokeDevice(revoke_device) => {
                    set.remove(&revoke_device.did);
                }
            }
        }
        Ok(set)
    }
}
