use std::sync::Arc;
use crate::storage::Storage;
use crate::storage::memory::{
    outbox::InMemoryOutboxStore,
    actors::InMemoryActorStore,
    devices::InMemoryDeviceStore,
};

pub fn memory_storage() -> Storage {
    let outbox = Arc::new(InMemoryOutboxStore::new());
    Storage {
        inbox: outbox.clone(),
        outbox,
        actors: Arc::new(InMemoryActorStore::new()),
        devices: Arc::new(InMemoryDeviceStore::new()),
    }
}
