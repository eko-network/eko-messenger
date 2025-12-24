use sqlx::PgPool;
use std::sync::Arc;
use crate::storage::Storage;
use crate::storage::postgres::{
    inbox::PostgresInboxStore,
    outbox::PostgresOutboxStore,
    actors::PostgresActorStore,
    devices::PostgresDeviceStore,
};

pub fn postgres_storage(pool: PgPool) -> Storage {
    Storage {
        inbox: Arc::new(PostgresInboxStore::new(pool.clone())),
        outbox: Arc::new(PostgresOutboxStore::new(pool.clone())),
        actors: Arc::new(PostgresActorStore::new(pool.clone())),
        devices: Arc::new(PostgresDeviceStore::new(pool)),
    }
}

