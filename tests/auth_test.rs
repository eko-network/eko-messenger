mod common;
use common::spawn_app;
use eko_messenger::auth::{Auth, LoginRequest, PreKey, SignedPreKey};
use eko_messenger::firebase_auth::FirebaseAuth;
use sqlx::PgPool;
use std::env;
use uuid::Uuid;

async fn get_auth_service(pool: PgPool) -> Auth<FirebaseAuth> {
    let firebase_auth = FirebaseAuth::new_from_env().unwrap();
    Auth::new(firebase_auth, pool)
}

fn generate_login_request(email: String, password: String) -> LoginRequest {
    LoginRequest {
        email,
        password,
        device_name: "test_device".to_string(),
        device_id: Uuid::new_v4().to_string(),
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

#[tokio::test]
async fn test_login_and_verify_token() {
    if env::var("FIREBASE_API_KEY").is_err() {
        println!("Skipping test: FIREBASE_API_KEY not set");
        return;
    }
    let email = env::var("TEST_USER_EMAIL").expect("TEST_USER_EMAIL not set");
    let password = env::var("TEST_USER_PASSWORD").expect("TEST_USER_PASSWORD not set");

    let app = spawn_app().await;
    let auth = get_auth_service(app.db_pool.clone()).await;

    let login_req = generate_login_request(email, password);

    let login_res = auth
        .login(login_req, "127.0.0.1", "test-agent")
        .await
        .unwrap();

    assert!(!login_res.access_token.is_empty());
    assert!(!login_res.refresh_token.is_empty());

    let claims = auth.verify_access_token(&login_res.access_token).unwrap();

    assert!(!claims.sub.is_empty());
}

#[tokio::test]
async fn test_refresh_token() {
    if env::var("FIREBASE_API_KEY").is_err() {
        println!("Skipping test: FIREBASE_API_KEY not set");
        return;
    }
    let email = env::var("TEST_USER_EMAIL").expect("TEST_USER_EMAIL not set");
    let password = env::var("TEST_USER_PASSWORD").expect("TEST_USER_PASSWORD not set");

    let app = spawn_app().await;
    let auth = get_auth_service(app.db_pool.clone()).await;

    let login_req = generate_login_request(email, password);

    let login_res = auth
        .login(login_req, "127.0.0.1", "test-agent")
        .await
        .unwrap();

    let refresh_res = auth
        .refresh_token(&login_res.refresh_token, "127.0.0.1", "test-agent")
        .await
        .unwrap();

    assert!(!refresh_res.access_token.is_empty());

    let claims = auth.verify_access_token(&refresh_res.access_token).unwrap();

    let first_claims = auth.verify_access_token(&login_res.access_token).unwrap();
    assert_eq!(claims.sub, first_claims.sub);
}

#[tokio::test]
async fn test_logout() {
    if env::var("FIREBASE_API_KEY").is_err() {
        println!("Skipping test: FIREBASE_API_KEY not set");
        return;
    }
    let email = env::var("TEST_USER_EMAIL").expect("TEST_USER_EMAIL not set");
    let password = env::var("TEST_USER_PASSWORD").expect("TEST_USER_PASSWORD not set");

    let app = spawn_app().await;
    let auth = get_auth_service(app.db_pool.clone()).await;

    let login_req = generate_login_request(email, password);

    let login_res = auth
        .login(login_req, "127.0.0.1", "test-agent")
        .await
        .unwrap();

    auth.logout(&login_res.refresh_token).await.unwrap();

    let refresh_result = auth
        .refresh_token(&login_res.refresh_token, "127.0.0.1", "test-agent")
        .await;
    assert!(refresh_result.is_err());
}
