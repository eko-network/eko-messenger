use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::storage::models::{StoredActivity, StoredOutboxActivity};
use crate::storage::traits::{InboxStore, OutboxStore};
use crate::errors::AppError;
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
    inbox_entries: RwLock<HashMap<String, Vec<String>>>, // inbox_actor_id -> Vec<activity_id>
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
    async fn insert_activity(
        &self,
        activity: &StoredOutboxActivity,
    ) -> Result<(), AppError> {
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
    ) -> Result<Vec<StoredActivity>, AppError> {
        let inbox_entries = self.inbox_entries.read().unwrap();
        let activity_ids = inbox_entries
            .get(inbox_actor_id)
            .cloned()
            .unwrap_or_default();
        drop(inbox_entries);

        let activities = self.activities.read().unwrap();
        let mut items: Vec<StoredActivity> = activity_ids
            .into_iter()
            .filter_map(|id| {
                activities.get(&id).map(|a| StoredActivity {
                    activity: a.activity_json.clone(),
                    inbox_actor_id: inbox_actor_id.to_string(),
                    created_at: a.created_at,
                })
            })
            .collect();

        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(items)
    }

    async fn insert_inbox_entry(
        &self,
        inbox_actor_id: &str,
        activity_id: &str,
    ) -> Result<(), AppError> {
        let mut inbox_entries = self.inbox_entries.write().unwrap();
        let entry = inbox_entries
            .entry(inbox_actor_id.to_string())
            .or_insert_with(Vec::new);
        if !entry.iter().any(|id| id == activity_id) {
            entry.push(activity_id.to_string());
        }
        Ok(())
    }
}
