use eko_messenger::{
    AppState, app, auth::Auth, firebase_auth::FirebaseAuth, jwt_helper::JwtHelper,
};
use redis::Client;
use std::{env, sync::Arc};
use tokio::net::TcpListener;
use uuid::Uuid;

pub struct TestApp {
    pub address: String,
    pub redis_client: Client,
    pub domain: String,
}

pub fn generate_test_token(username: &str) -> String {
    let jwt_helper = JwtHelper::new_from_env().unwrap();
    jwt_helper.create_jwt(username).unwrap()
}
pub fn generate_user_id() -> String {
    Uuid::new_v4().to_string()
}

pub async fn spawn_app() -> TestApp {
    // Bind to port 0 to let the OS assign a random available port.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");

    let domain = format!("127.0.0.1:{}", port);

    let redis_client = Client::open(redis_url).expect("Failed to create Redis client");
    let redis_conn = redis_client
        .get_multiplexed_tokio_connection()
        .await
        .expect("Failed to connect to Redis");

    let firebase_auth =
        FirebaseAuth::new_from_env().expect("Failed to create FirebaseAuth from env");
    let auth_service = Auth::new(firebase_auth, redis_conn.clone());

    let app_state = AppState {
        redis: redis_conn,
        auth: Arc::new(auth_service),
        domain: domain.clone(),
    };

    let app_router = app(app_state, "ConnectInfo".to_string())
        .expect("Failed to build Axum router in test setup");

    tokio::spawn(async move {
        axum::serve(listener, app_router).await.unwrap();
    });

    TestApp {
        address,
        redis_client,
        domain,
    }
}

