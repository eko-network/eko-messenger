use std::sync::Arc;
use crate::storage::Storage;
use crate::storage::memory::{
    inbox::InMemoryInboxStore,
    outbox::InMemoryOutboxStore,
    actors::InMemoryActorStore,
    devices::InMemoryDeviceStore,
};

pub fn memory_storage() -> Storage {
    Storage {
        inbox: Arc::new(InMemoryInboxStore::new()),
        outbox: Arc::new(InMemoryOutboxStore::new()),
        actors: Arc::new(InMemoryActorStore::new()),
        devices: Arc::new(InMemoryDeviceStore::new()),
    }
}
