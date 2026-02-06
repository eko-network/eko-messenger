mod assertions;
mod fixtures;

pub use assertions::*;
pub use fixtures::*;

use async_trait::async_trait;
use axum::{Router, routing::get};
use eko_messenger::{
    AppState,
    activitypub::{Person, create_person},
    app,
    auth::{IdentityProvider, LoginRequest, LoginResponse, PreKey, SessionManager, SignedPreKey},
    errors::AppError,
    notifications::NotificationService,
    storage::{Storage, postgres::connection::postgres_storage},
    websocket::WebSocketService,
};
use reqwest::Client;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::{env, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use uuid::Uuid;

/// Test-only identity provider that implements IdentityProvider
/// Used by integration tests to avoid dependency on Firebase/OIDC
pub struct TestIdentityProvider {
    pub storage: Arc<Storage>,
    pub domain: Arc<String>,
}

impl TestIdentityProvider {
    pub fn new(storage: Arc<Storage>, domain: Arc<String>) -> Self {
        Self { storage, domain }
    }
}

#[async_trait]
impl IdentityProvider for TestIdentityProvider {
    async fn person_from_uid(&self, uid: &str) -> Result<Person, AppError> {
        // Look up user in storage
        let user = self
            .storage
            .users
            .get_user_by_uid(uid)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", uid)))?;

        // Create Person actor from user
        let person = create_person(
            &self.domain,
            &user.uid,
            None,                  // summary
            user.username.clone(), // preferred_username
            None,                  // name
            None,                  // profile_picture
        );

        Ok(person)
    }

    async fn uid_from_username(&self, username: &str) -> Result<String, AppError> {
        // Look up user by username
        let user = self
            .storage
            .users
            .get_user_by_username(username)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", username)))?;
        Ok(user.uid)
    }
}

pub struct TestApp {
    pub domain: Arc<String>,
    pub address: String,
    pub storage: Arc<Storage>,
    pub sessions: Arc<SessionManager>,
    pub client: Client,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageBackend {
    Postgres,
}

pub struct SpawnOptions {
    pub storage: StorageBackend,
}

impl Default for SpawnOptions {
    fn default() -> Self {
        Self {
            storage: StorageBackend::Postgres,
        }
    }
}

pub async fn spawn_app() -> TestApp {
    spawn_app_with_options(SpawnOptions::default()).await
}

pub async fn spawn_app_with_storage(storage: StorageBackend) -> TestApp {
    spawn_app_with_options(SpawnOptions { storage }).await
}

pub async fn spawn_app_with_options(options: SpawnOptions) -> TestApp {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .unwrap_or_else(|_| {});

    if env::var("JWT_SECRET").is_err() {
        unsafe {
            env::set_var("JWT_SECRET", "test-jwt-secret-do-not-use-in-prod");
        }
    }

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);
    let domain = Arc::new(format!("http://127.0.0.1:{}", port));

    // Initialize storage
    let storage = match options.storage {
        StorageBackend::Postgres => {
            Arc::new(postgres_storage(domain.clone(), postgres_pool().await))
        }
    };

    // Create test identity provider (implements IdentityProvider trait)
    let test_identity = TestIdentityProvider::new(storage.clone(), domain.clone());

    // Create session manager
    let sessions = Arc::new(
        SessionManager::new(domain.clone(), storage.clone())
            .expect("Failed to create SessionManager"),
    );

    let notification_service = NotificationService::new(storage.clone())
        .await
        .expect("Failed to create notification_service");

    let app_state = AppState {
        domain: domain.clone(),
        identity: Arc::new(test_identity) as Arc<dyn IdentityProvider>,
        auth: None, // Tests don't use production auth providers
        sessions: sessions.clone(),
        storage: storage.clone(),
        sockets: Arc::new(WebSocketService::new()),
        notification_service: Arc::new(notification_service),
    };

    // Use production app function to get full router
    let app_router = app(app_state, "ConnectInfo".to_string()).expect("Failed to build app router");

    tokio::spawn(async move {
        axum::serve(
            listener,
            app_router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    TestApp {
        address,
        domain,
        storage,
        sessions,
        client: Client::new(),
    }
}

impl TestApp {
    pub fn actor_url(&self, uid: &str) -> String {
        format!("{}/users/{}", self.domain, uid)
    }

    /// Create a test user directly via storage (no HTTP signup)
    pub async fn create_test_user(&self, username: &str, email: &str) -> String {
        let uid = Uuid::new_v4().to_string();

        self.storage
            .users
            .create_oidc_user(
                &uid,
                username,
                email,
                "test-issuer",
                &Uuid::new_v4().to_string(),
            )
            .await
            .expect("Failed to create test user");

        uid
    }

    pub fn generate_login_request(
        &self,
        email: String,
        password: String,
        device_name: Option<&str>,
    ) -> LoginRequest {
        let device_name = device_name.unwrap_or("test_device");

        // FIXME keys and things need to be fixed
        LoginRequest {
            email,
            password,
            device_name: device_name.to_string(),
            identity_key: vec![1, 2, 3],
            registration_id: 123,
            pre_keys: vec![PreKey {
                id: 1,
                key: vec![4, 5, 6],
            }],
            signed_pre_key: SignedPreKey {
                id: 1,
                key: vec![7, 8, 9],
                signature: vec![10, 11, 12],
            },
        }
    }
}

async fn postgres_pool() -> PgPool {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set when using the Postgres test backend");

    let schema = format!("test_{}", Uuid::new_v4().simple());
    let admin_pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to Postgres");
    sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS \"{schema}\""))
        .execute(&admin_pool)
        .await
        .expect("Failed to create test schema");
    drop(admin_pool);

    let schema_for_connect = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .after_connect(move |conn, _meta| {
            let schema = schema_for_connect.clone();
            Box::pin(async move {
                sqlx::query(&format!("SET search_path TO \"{schema}\""))
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&database_url)
        .await
        .expect("Failed to connect to Postgres with schema");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations on test schema");

    pool
}
