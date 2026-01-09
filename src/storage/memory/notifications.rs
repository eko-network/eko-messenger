use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::{errors::AppError, storage::traits::NotificationStore};

#[derive(Default)]
pub struct InMemoryNotificationStore {
    endpoints: RwLock<HashMap<i32, web_push::SubscriptionInfo>>,
}

impl InMemoryNotificationStore {
    pub fn new() -> Self {
        Self {
            endpoints: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl NotificationStore for InMemoryNotificationStore {
    async fn upsert_endpoint(
        &self,
        did: i32,
        endpoint: &web_push::SubscriptionInfo,
    ) -> Result<(), AppError> {
        let mut endpoints = self.endpoints.write().unwrap();
        endpoints.insert(did, endpoint.clone());
        Ok(())
    }

    async fn retrive_endpoints(
        &self,
        dids: &[i32],
    ) -> Result<Vec<web_push::SubscriptionInfo>, AppError> {
        let endpoints = self.endpoints.read().unwrap();
        let results = dids
            .iter()
            .filter_map(|did| endpoints.get(did).cloned())
            .collect();
        Ok(results)
    }
}
