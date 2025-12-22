use eko_messenger::{
    AppState,
    app,
    auth::{Auth, LoginRequest, PreKey, SignedPreKey},
    firebase_auth::FirebaseAuth,
    storage::{Storage, postgres::connection::postgres_storage},
};
use sqlx::PgPool;
use std::{env, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

pub struct TestApp {
    pub address: String,
    pub domain: String,
    pub storage: Arc<Storage>,
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
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .unwrap_or_else(|_| {});
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);
    let domain = format!("127.0.0.1:{}", port);

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db_pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to Postgres.");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run migrations on test database");

    // Clear tables
    for table in [
        "actors",
        "activities",
        "inbox_entries",
        "devices",
        "refresh_tokens",
        "pre_keys",
        "signed_pre_keys",
    ] {
        sqlx::query(&format!("TRUNCATE TABLE {table} RESTART IDENTITY CASCADE"))
            .execute(&db_pool)
            .await
            .unwrap();
    }

    let storage = Arc::new(postgres_storage(db_pool));

    let firebase_auth =
        FirebaseAuth::new_from_env().expect("Failed to create FirebaseAuth from env");
    let auth_service = Auth::new(firebase_auth, storage.clone());

    let app_state = AppState {
        auth: Arc::new(auth_service),
        domain: domain.clone(),
        storage: storage.clone(),
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
        domain,
        storage,
    }
}
