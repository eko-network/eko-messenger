use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::{
    errors::AppError,
    storage::traits::ActorStore,
};

#[derive(Clone, Debug)]
struct Actor {
    id: String,
    is_local: bool,
    inbox_url: String,
    outbox_url: String,
}

#[derive(Default)]
pub struct InMemoryActorStore {
    actors: RwLock<HashMap<String, Actor>>,
}

impl InMemoryActorStore {
    pub fn new() -> Self {
        Self {
            actors: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ActorStore for InMemoryActorStore {
    async fn ensure_local_actor(
        &self,
        actor_id: &str,
        inbox_url: &str,
        outbox_url: &str,
    ) -> Result<(), AppError> {
        let mut actors = self.actors.write().unwrap();
        actors.entry(actor_id.to_string()).or_insert_with(|| Actor {
            id: actor_id.to_string(),
            is_local: true,
            inbox_url: inbox_url.to_string(),
            outbox_url: outbox_url.to_string(),
        });

        Ok(())
    }
}
