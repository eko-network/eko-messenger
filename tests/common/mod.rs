use eko_messenger::{
    AppState, app,
    auth::{Auth, LoginRequest, PreKey, SignedPreKey},
    firebase_auth::FirebaseAuth,
};
use sqlx::PgPool;
use std::{env, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub domain: String,
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
    sqlx::query!("TRUNCATE TABLE actors RESTART IDENTITY CASCADE")
        .execute(&db_pool)
        .await
        .expect("Failed to truncate actors table");
    sqlx::query!("TRUNCATE TABLE activities RESTART IDENTITY CASCADE")
        .execute(&db_pool).await
        .expect("Failed to truncate activities table");
    sqlx::query!("TRUNCATE TABLE inbox_entries RESTART IDENTITY CASCADE")
        .execute(&db_pool)
        .await
        .expect("Failed to truncate inbox_entries table");
    sqlx::query!("TRUNCATE TABLE devices RESTART IDENTITY CASCADE")
        .execute(&db_pool)
        .await
        .expect("Failed to truncate devices table");
    sqlx::query!("TRUNCATE TABLE refresh_tokens RESTART IDENTITY CASCADE")
        .execute(&db_pool)
        .await
        .expect("Failed to truncate refresh_tokens table");
    sqlx::query!("TRUNCATE TABLE pre_keys RESTART IDENTITY CASCADE")
        .execute(&db_pool)
        .await
        .expect("Failed to truncate pre_keys table");
    sqlx::query!("TRUNCATE TABLE signed_pre_keys RESTART IDENTITY CASCADE")
        .execute(&db_pool)
        .await
        .expect("Failed to truncate signed_pre_keys table");

    let firebase_auth =
        FirebaseAuth::new_from_env().expect("Failed to create FirebaseAuth from env");
    let auth_service = Auth::new(firebase_auth, db_pool.clone());

    let app_state = AppState {
        auth: Arc::new(auth_service),
        domain: domain.clone(),
        pool: db_pool.clone(),
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
        db_pool,
        domain,
    }
}
