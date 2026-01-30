mod assertions;
mod fixtures;
mod local_auth;

pub use assertions::*;
pub use fixtures::*;
pub use local_auth::LocalIdentityProvider;

#[cfg(feature = "auth-firebase")]
use ::eko_messenger::auth::FirebaseAuth;

use dashmap::DashMap;
use eko_messenger::{
    AppState, app,
    auth::{Auth, LoginRequest, LoginResponse, PreKey, SignedPreKey},
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
    #[cfg(feature = "auth-firebase")]
    Firebase,
}

pub struct SpawnOptions {
    pub storage: StorageBackend,
    pub identity: IdentityBackend,
}

impl Default for SpawnOptions {
    fn default() -> Self {
        Self {
            storage: StorageBackend::Postgres,
            identity: IdentityBackend::Test,
        }
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

#[cfg(feature = "auth-firebase")]
// NOTE this function is never called. If we want to use the entire test bench with firebase auth
// then we would have to do some restructure.
pub async fn spawn_app_firebase() -> TestApp {
    spawn_app_with_options(SpawnOptions {
        identity: IdentityBackend::Firebase,
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

    // Initialize server address
    let storage = match options.storage {
        StorageBackend::Postgres => {
            Arc::new(postgres_storage(domain.clone(), postgres_pool().await))
        }
    };

    let auth_service = match options.identity {
        IdentityBackend::Test => Auth::new(
            domain.clone(),
            LocalIdentityProvider::new(domain.clone(), storage.clone()),
            storage.clone(),
        ),
        #[cfg(feature = "auth-firebase")]
        IdentityBackend::Firebase => {
            let client = reqwest::Client::new();
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
        oidc_provider: None,
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
        storage: storage,
        client: Client::new(),
    }
}

impl TestApp {
    pub fn actor_url(&self, uid: &str) -> String {
        format!("{}/users/{}", self.domain, uid)
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

    pub async fn signup_http(&self, username: &str, email: &str, password: &str) {
        use serde_json::json;

        let signup_url = format!("{}/auth/v1/signup", &self.address);
        let signup_body = json!({
            "username": username,
            "email": email,
            "password": password,
        });

        let signup_res = self
            .client
            .post(&signup_url)
            .header("User-Agent", "test-client")
            .json(&signup_body)
            .send()
            .await
            .expect("HTTP Signup failed");

        let status = signup_res.status();
        assert!(status.is_success(), "Signup failed with status {}", status);
    }

    pub async fn login_http(&self, email: &str, password: &str) -> LoginResponse {
        let login_req = self.generate_login_request(email.to_string(), password.to_string(), None);
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
