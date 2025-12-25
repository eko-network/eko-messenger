use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::errors::AppError;
use crate::storage::models::StoredInboxEntry;
use crate::storage::traits::InboxStore;

#[derive(Default)]
pub struct InMemoryInboxStore {
    // inbox_actor_id -> Vec<StoredInboxEntry>
    inboxes: RwLock<HashMap<String, Vec<StoredInboxEntry>>>,
}

impl InMemoryInboxStore {
    pub fn new() -> Self {
        Self {
            inboxes: RwLock::new(HashMap::new()),
        }
    }

    pub fn add_activity(&self, inbox_actor_id: &str, entry: StoredInboxEntry) {
        let mut inboxes = self.inboxes.write().unwrap();
        inboxes
            .entry(inbox_actor_id.to_string())
            .or_insert_with(Vec::new)
            .push(entry);
    }
}

#[async_trait]
impl InboxStore for InMemoryInboxStore {
    async fn inbox_activities(
        &self,
        inbox_actor_id: &str,
        _did: i32,
    ) -> Result<Vec<StoredInboxEntry>, AppError> {
        let mut inboxes = self.inboxes.write().unwrap();
        Ok(inboxes.remove(inbox_actor_id).unwrap_or_default())
    }

    async fn insert_inbox_entry(
        &self,
        _inbox_actor_id: &str,
        _to_did: i32,
        _entry: StoredInboxEntry,
    ) -> Result<(), AppError> {
        Err(AppError::InternalError(anyhow::anyhow!(
            "InMemoryInboxStore does not support insert_inbox_entry; use the shared InMemoryOutboxStore-backed inbox wiring"
        )))
    }
}
