use async_trait::async_trait;
use sqlx::PgPool;

use crate::errors::AppError;
use crate::storage::models::StoredInboxEntry;
use crate::storage::traits::InboxStore;

pub struct PostgresInboxStore {
    pool: PgPool,
}

impl PostgresInboxStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InboxStore for PostgresInboxStore {
    async fn inbox_activities(
        &self,
        inbox_actor_id: &str,
        did: &str,
    ) -> Result<Vec<StoredInboxEntry>, AppError> {
        let rows = sqlx::query!(
            r#"
            DELETE FROM inbox_entries
            WHERE inbox_actor_id = $1 AND to_did = $2
            RETURNING actor_id, content, from_did
            "#,
            inbox_actor_id,
            did
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| StoredInboxEntry {
                actor_id: r.actor_id,
                from_did: r.from_did,
                content: r.content,
            })
            .collect())
    }

    async fn insert_inbox_entry(
        &self,
        inbox_actor_id: &str,
        to_did: &str,
        entry: StoredInboxEntry,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO inbox_entries (actor_id, inbox_actor_id, from_did, to_did, content)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            entry.actor_id,
            inbox_actor_id,
            entry.from_did,
            to_did,
            entry.content
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
