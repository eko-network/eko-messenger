use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::errors::AppError;
use crate::storage::models::{StoredInboxEntry, StoredOutboxActivity};
use crate::storage::traits::{InboxStore, OutboxStore};
use time::OffsetDateTime;

#[derive(Clone, Debug)]
struct Activity {
    id: String,
    activity_json: Value,
    created_at: OffsetDateTime,
}

#[derive(Default)]
pub struct InMemoryOutboxStore {
    activities: RwLock<HashMap<String, Activity>>,
    // inbox_actor_id -> to_did -> Vec<StoredInboxEntry>
    inbox_entries: RwLock<HashMap<String, HashMap<i32, Vec<StoredInboxEntry>>>>,
}

impl InMemoryOutboxStore {
    pub fn new() -> Self {
        Self {
            activities: RwLock::new(HashMap::new()),
            inbox_entries: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl OutboxStore for InMemoryOutboxStore {
    async fn insert_activity(&self, activity: &StoredOutboxActivity) -> Result<(), AppError> {
        let stored_activity = Activity {
            id: activity.activity_id.to_string(),
            activity_json: activity.activity.clone(),
            created_at: activity.created_at,
        };

        let mut activities = self.activities.write().unwrap();
        activities.insert(stored_activity.id.to_string(), stored_activity);

        Ok(())
    }
}

#[async_trait]
impl InboxStore for InMemoryOutboxStore {
    async fn inbox_activities(
        &self,
        inbox_actor_id: &str,
        did: i32,
    ) -> Result<Vec<StoredInboxEntry>, AppError> {
        let inbox_entries = self.inbox_entries.read().unwrap();
        Ok(inbox_entries
            .get(inbox_actor_id)
            .and_then(|by_did| by_did.get(&did))
            .cloned()
            .unwrap_or_default())
    }

    async fn insert_inbox_entry(
        &self,
        inbox_actor_id: &str,
        to_did: i32,
        entry: StoredInboxEntry,
    ) -> Result<(), AppError> {
        let mut inbox_entries = self.inbox_entries.write().unwrap();
        let by_actor = inbox_entries
            .entry(inbox_actor_id.to_string())
            .or_insert_with(HashMap::new);
        let list = by_actor.entry(to_did).or_insert_with(Vec::new);
        list.push(entry);
        Ok(())
    }
}
