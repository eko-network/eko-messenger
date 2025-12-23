use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::errors::AppError;
use crate::storage::models::StoredActivity;
use crate::storage::traits::InboxStore;

#[derive(Default)]
pub struct InMemoryInboxStore {
    // inbox_actor_id -> Vec<activity_json>
    inboxes: RwLock<HashMap<String, Vec<StoredActivity>>>,
}

impl InMemoryInboxStore {
    pub fn new() -> Self {
        Self {
            inboxes: RwLock::new(HashMap::new()),
        }
    }

    pub fn add_activity(&self, inbox_actor_id: &str, activity: StoredActivity) {
        let mut inboxes = self.inboxes.write().unwrap();
        inboxes
            .entry(inbox_actor_id.to_string())
            .or_insert_with(Vec::new)
            .push(activity);
    }
}

#[async_trait]
impl InboxStore for InMemoryInboxStore {
    async fn inbox_activities(
        &self,
        inbox_actor_id: &str,
    ) -> Result<Vec<StoredActivity>, AppError> {
        let inboxes = self.inboxes.read().unwrap();
        Ok(inboxes
            .get(inbox_actor_id)
            .cloned()
            .unwrap_or_default())
    }
}
