use crate::storage::Storage;
use crate::storage::memory::{
    actors::InMemoryActorStore, devices::InMemoryDeviceStore, outbox::InMemoryOutboxStore,
};
use std::sync::Arc;

pub fn memory_storage() -> Storage {
    let outbox = Arc::new(InMemoryOutboxStore::new());
    Storage {
        inbox: outbox.clone(),
        outbox,
        actors: Arc::new(InMemoryActorStore::new()),
        devices: Arc::new(InMemoryDeviceStore::new()),
    }
}
