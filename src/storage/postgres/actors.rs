use async_trait::async_trait;
use sqlx::PgPool;

use crate::{
    errors::AppError,
    storage::traits::ActorStore,
};

pub struct PostgresActorStore {
    pool: PgPool,
}

impl PostgresActorStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ActorStore for PostgresActorStore {
    async fn upsert_local_actor(
        &self,
        actor_id: &str,
        inbox_url: &str,
        outbox_url: &str,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO actors (id, is_local, inbox_url, outbox_url)
            VALUES ($1, true, $2, $3)
            ON CONFLICT (id) DO NOTHING
            "#,
            actor_id,
            inbox_url,
            outbox_url,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn is_local_actor(
        &self,
        actor_id: &str,
    ) -> Result<bool, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT is_local
            FROM actors
            WHERE id = $1
            "#,
            actor_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.is_local).unwrap_or(false))
    }
}

