use crate::storage::Storage;
use crate::storage::postgres::PostgresNotificationStore;
use crate::storage::postgres::{
    activities::PostgresActivityStore, actors::PostgresActorStore, devices::PostgresDeviceStore,
    users::PostgresUserStore,
};
use sqlx::PgPool;
use std::sync::Arc;

pub fn postgres_storage(domain: Arc<String>, pool: PgPool) -> Storage {
    Storage {
        notifications: Arc::new(PostgresNotificationStore::new(pool.clone())),
        activities: Arc::new(PostgresActivityStore::new(domain.clone(), pool.clone())),
        actors: Arc::new(PostgresActorStore::new(pool.clone())),
        devices: Arc::new(PostgresDeviceStore::new(domain, pool.clone())),
        users: Arc::new(PostgresUserStore::new(pool)),
    }
}
