use async_trait::async_trait;
use sqlx::PgPool;

use crate::errors::AppError;
use crate::storage::models::StoredActivity;
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
    ) -> Result<Vec<StoredActivity>, AppError> {
        let rows = sqlx::query!(
            r#"
            SELECT a.activity_json, a.created_at
            FROM activities a
            INNER JOIN inbox_entries ie ON a.id = ie.activity_id
            WHERE ie.inbox_actor_id = $1
            ORDER BY a.created_at DESC
            "#,
            inbox_actor_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| StoredActivity {
                activity: r.activity_json,
                inbox_actor_id: inbox_actor_id.to_string(),
                created_at: r.created_at,
            })
            .collect())
    }
}

