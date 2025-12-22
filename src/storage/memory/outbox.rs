use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::storage::traits::OutboxStore;
use crate::errors::AppError;

#[derive(Clone, Debug)]
struct Activity {
    id: String,
    actor_id: String,
    activity_type: String,
    activity_json: Value,
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
    async fn insert_local_activity(
        &self,
        activity_id: &str,
        actor_id: &str,
        activity_type: &str,
        activity_json: Value,
        inbox_actor_id: &str,
    ) -> Result<(), AppError> {
        let activity = Activity {
            id: activity_id.to_string(),
            actor_id: actor_id.to_string(),
            activity_type: activity_type.to_string(),
            activity_json,
        };

        let mut activities = self.activities.write().unwrap();
        activities.insert(activity_id.to_string(), activity);
        drop(activities);

        let mut inbox_entries = self.inbox_entries.write().unwrap();
        inbox_entries
            .entry(inbox_actor_id.to_string())
            .or_insert_with(Vec::new)
            .push(activity_id.to_string());

        Ok(())
    }
}
