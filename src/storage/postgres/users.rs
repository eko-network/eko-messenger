use crate::{
    errors::AppError,
    storage::{models::StoredUser, traits::UserStore},
};
use async_trait::async_trait;
use sqlx::PgPool;

pub struct PostgresUserStore {
    pool: PgPool,
}

impl PostgresUserStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserStore for PostgresUserStore {
    async fn get_user_by_email(&self, email: &str) -> Result<Option<StoredUser>, AppError> {
        let user = sqlx::query_as!(
            StoredUser,
            r#"
            SELECT uid, username, email, oidc_issuer, oidc_sub, created_at
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn get_user_by_uid(&self, uid: &str) -> Result<Option<StoredUser>, AppError> {
        let user = sqlx::query_as!(
            StoredUser,
            r#"
            SELECT uid, username, email, oidc_issuer, oidc_sub, created_at
            FROM users
            WHERE uid = $1
            "#,
            uid
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<StoredUser>, AppError> {
        let user = sqlx::query_as!(
            StoredUser,
            r#"
            SELECT uid, username, email, oidc_issuer, oidc_sub, created_at
            FROM users
            WHERE username = $1
            "#,
            username
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn get_user_by_oidc(
        &self,
        oidc_issuer: &str,
        oidc_sub: &str,
    ) -> Result<Option<StoredUser>, AppError> {
        let user = sqlx::query_as!(
            StoredUser,
            r#"
            SELECT uid, username, email, oidc_issuer, oidc_sub, created_at
            FROM users
            WHERE oidc_issuer = $1 AND oidc_sub = $2
            "#,
            oidc_issuer,
            oidc_sub
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn create_oidc_user(
        &self,
        uid: &str,
        username: &str,
        email: &str,
        oidc_issuer: &str,
        oidc_sub: &str,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO users (uid, username, email, oidc_issuer, oidc_sub)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            uid,
            username,
            email,
            oidc_issuer,
            oidc_sub
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
