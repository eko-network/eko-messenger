use async_trait::async_trait;
use dashmap::DashMap;
use eko_messenger::{
    AppState, app,
    auth::{
        Auth, FirebaseAuth, IdentityProvider, LoginRequest, LoginResponse, PreKey, SignedPreKey,
    },
    notifications::NotificationService,
    storage::{Storage, postgres::connection::postgres_storage},
};
use reqwest::Client;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::{env, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use uuid::Uuid;

pub struct TestApp {
    pub domain: Arc<String>,
    pub address: String,
    pub storage: Arc<Storage>,
    pub client: Client,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageBackend {
    Postgres,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityBackend {
    Test,
    Firebase,
}

pub struct SpawnOptions {
    pub storage: StorageBackend,
    pub identity: IdentityBackend,
}

impl Default for SpawnOptions {
    fn default() -> Self {
        Self {
            storage: selected_storage_backend(),
            identity: IdentityBackend::Test,
        }
    }
}

fn selected_storage_backend() -> StorageBackend {
    let backend = env::var("TEST_STORAGE_BACKEND")
        .or_else(|_| env::var("STORAGE_BACKEND"))
        .unwrap_or_else(|_| "postgres".to_string());

    match backend.to_lowercase().as_str() {
        "postgres" => StorageBackend::Postgres,
        other => panic!("Invalid storage backend '{other}'."),
    }
}

#[derive(Clone)]
pub struct TestIdentityProvider {
    domain: Arc<String>,
}

impl TestIdentityProvider {
    pub fn new(domain: Arc<String>) -> Self {
        Self { domain }
    }
}

fn uid_from_email(email: &str) -> String {
    let uid: String = email
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if uid.is_empty() {
        "testuser".to_string()
    } else {
        uid
    }
}

#[async_trait]
impl IdentityProvider for TestIdentityProvider {
    async fn login_with_email(
        &self,
        email: String,
        _password: String,
    ) -> Result<(eko_messenger::activitypub::Person, String), eko_messenger::errors::AppError> {
        let uid = uid_from_email(&email);
        let person = self.person_from_uid(&uid).await?;
        Ok((person, uid))
    }

    async fn person_from_uid(
        &self,
        uid: &str,
    ) -> Result<eko_messenger::activitypub::Person, eko_messenger::errors::AppError> {
        use eko_messenger::activitypub::create_person;
        Ok(create_person(
            &self.domain,
            uid,
            Some("Test user".to_string()),
            uid.to_string(),
            Some("Test User".to_string()),
            None,
        ))
    }

    async fn uid_from_username(
        &self,
        username: &str,
    ) -> Result<String, eko_messenger::errors::AppError> {
        // In test mode, username is the uid
        Ok(username.to_string())
    }
}

pub fn generate_login_request(email: String, password: String) -> LoginRequest {
    LoginRequest {
        email,
        password,
        device_name: "test_device".to_string(),
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

pub async fn spawn_app() -> TestApp {
    spawn_app_with_options(SpawnOptions::default()).await
}

pub async fn spawn_app_with_storage(storage: StorageBackend) -> TestApp {
    spawn_app_with_options(SpawnOptions {
        storage,
        ..Default::default()
    })
    .await
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

    // Initialize server address for tests
    // Note: This will fail if called multiple times, but that's ok for tests
    let storage = match options.storage {
        StorageBackend::Postgres => {
            Arc::new(postgres_storage(domain.clone(), postgres_pool().await))
        }
    };

    let client = reqwest::Client::new();

    let auth_service = match options.identity {
        IdentityBackend::Test => Auth::new(
            domain.clone(),
            TestIdentityProvider::new(domain.clone()),
            storage.clone(),
        ),
        IdentityBackend::Firebase => {
            let firebase_auth = FirebaseAuth::new_from_env(domain.clone(), client.clone())
                .await
                .expect("Failed to create FirebaseAuth from env");
            Auth::new(domain.clone(), firebase_auth, storage.clone())
        }
    };

    let notification_service = NotificationService::new(storage.clone())
        .await
        .expect("Failed to create notification_service");

    let app_state = AppState {
        domain: domain.clone(),
        auth: Arc::new(auth_service),
        storage: storage.clone(),
        sockets: Arc::new(DashMap::new()),
        notification_service: Arc::new(notification_service),
    };

    let app_router = app(app_state, "ConnectInfo".to_string())
        .expect("Failed to build Axum router in test setup");

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
        domain: domain,
        storage,
        client: Client::new(),
    }
}

impl TestApp {
    pub fn actor_url(&self, uid: &str) -> String {
        format!("{}/users/{}", self.domain, uid)
    }

    pub async fn login_http(&self, email: &str, password: &str) -> LoginResponse {
        let login_req = generate_login_request(email.to_string(), password.to_string());
        let login_url = format!("{}/auth/v1/login", &self.address);

        let login_res = self
            .client
            .post(&login_url)
            .header("User-Agent", "test-client")
            .json(&login_req)
            .send()
            .await
            .expect("HTTP Login failed");

        let status = login_res.status();
        let body = login_res.text().await.expect("Failed reading login body");
        assert!(
            status.is_success(),
            "Login failed with status {}: {}",
            status,
            body
        );

        serde_json::from_str::<LoginResponse>(&body).expect("Failed to parse login response")
    }
}

#[cfg(feature = "integration-firebase")]
pub async fn spawn_app_firebase() -> TestApp {
    spawn_app_with_options(SpawnOptions {
        identity: IdentityBackend::Firebase,
        ..Default::default()
    })
    .await
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
