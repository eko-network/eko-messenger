use eko_messenger::{AppState, app, auth::Auth, firebase_auth::FirebaseAuth};
use sqlx::PgPool;
use std::{env, sync::Arc};
use tokio::net::TcpListener;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub domain: String,
}

pub async fn spawn_app() -> TestApp {
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
        axum::serve(listener, app_router).await.unwrap();
    });

    TestApp {
        address,
        db_pool,
        domain,
    }
}
