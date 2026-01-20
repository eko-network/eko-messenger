use crate::storage::postgres::PostgresNotificationStore;
use crate::storage::postgres::{
    actors::PostgresActorStore, devices::PostgresDeviceStore, inbox::PostgresInboxStore,
    outbox::PostgresOutboxStore, users::PostgresUserStore,
};
use crate::storage::Storage;
use sqlx::PgPool;
use std::sync::Arc;

pub fn postgres_storage(pool: PgPool) -> Storage {
    Storage {
        notifications: Arc::new(PostgresNotificationStore::new(pool.clone())),
        inbox: Arc::new(PostgresInboxStore::new(pool.clone())),
        outbox: Arc::new(PostgresOutboxStore::new(pool.clone())),
        actors: Arc::new(PostgresActorStore::new(pool.clone())),
        devices: Arc::new(PostgresDeviceStore::new(pool.clone())),
        users: Arc::new(PostgresUserStore::new(pool)),
    }
}
