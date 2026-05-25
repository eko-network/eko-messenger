use uuid::Uuid;

use crate::{AppState, errors::AppError, storage::models::StoredGroupState};

/// Service for managing encrypted group state.
/// Groups are stored as opaque encrypted blobs, so the server cannot decrypt or validate them.
pub struct GroupService;

impl GroupService {
    /// Upsert encrypted group state for a user. The server generates the resource id.
    /// Returns true if the state was written (new or higher epoch), false if epoch was stale.
    pub async fn upsert_group_state(
        state: &AppState,
        user_id: &str,
        group_id: Uuid,
        epoch: i64,
        encrypted_content: Vec<u8>,
    ) -> Result<bool, AppError> {
        let id = format!("{}/users/{}/groupState/{}", state.domain, user_id, group_id);

        let stored = StoredGroupState {
            id,
            group_id,
            user_id: user_id.to_string(),
            epoch,
            encrypted_content,
            encoding: "base64".to_string(),
        };

        state.storage.groups.upsert_group_state(&stored).await
    }

    /// Get a single encrypted group state by group_id for a user.
    pub async fn get_group_state(
        state: &AppState,
        user_id: &str,
        group_id: &Uuid,
    ) -> Result<Option<StoredGroupState>, AppError> {
        state
            .storage
            .groups
            .get_group_state(user_id, group_id)
            .await
    }

    /// List all encrypted group states for a user.
    pub async fn get_all_group_states(
        state: &AppState,
        user_id: &str,
    ) -> Result<Vec<StoredGroupState>, AppError> {
        state.storage.groups.get_all_group_states(user_id).await
    }

    /// Delete an encrypted group state. Returns true if a row was deleted.
    pub async fn delete_group_state(
        state: &AppState,
        user_id: &str,
        group_id: &Uuid,
    ) -> Result<bool, AppError> {
        state
            .storage
            .groups
            .delete_group_state(user_id, group_id)
            .await
    }
}
