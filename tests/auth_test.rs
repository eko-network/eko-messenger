mod common;
use common::{generate_login_request, spawn_app};
use eko_messenger::auth::{Auth, FirebaseAuth};
use eko_messenger::storage::Storage;
use reqwest::Client;
use serde_json::Value;
use std::env;
use std::sync::Arc;

async fn get_auth_service(storage: Arc<Storage>, domain: &str) -> Auth {
    let firebase_auth = FirebaseAuth::new_from_env_with_domain(domain.to_string())
        .await
        .unwrap();
    Auth::new(firebase_auth, storage)
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
    let auth = get_auth_service(app.storage.clone(), &app.domain).await;

    let login_req = generate_login_request(email, password);

    let login_res = auth
        .login(login_req, "127.0.0.1", "test-agent", &app.domain)
        .await
        .unwrap();

    assert!(!login_res.access_token.is_empty());

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
    let auth = get_auth_service(app.storage.clone(), &app.domain).await;

    let login_req = generate_login_request(email, password);

    let login_res = auth
        .login(login_req, "127.0.0.1", "test-agent", &app.domain)
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
    let auth = get_auth_service(app.storage.clone(), &app.domain).await;

    let login_req = generate_login_request(email, password);

    let login_res = auth
        .login(login_req, "127.0.0.1", "test-agent", &app.domain)
        .await
        .unwrap();

    auth.logout(&login_res.refresh_token).await.unwrap();

    let refresh_result = auth
        .refresh_token(&login_res.refresh_token, "127.0.0.1", "test-agent")
        .await;
    assert!(refresh_result.is_err());
}

#[tokio::test]
async fn test_http_login() {
    if env::var("FIREBASE_API_KEY").is_err() {
        println!("Skipping test: FIREBASE_API_KEY not set");
        return;
    }
    let email = env::var("TEST_USER_EMAIL").expect("TEST_USER_EMAIL not set");
    let password = env::var("TEST_USER_PASSWORD").expect("TEST_USER_PASSWORD not set");

    let app = spawn_app().await;
    let client = Client::new();

    let login_req = generate_login_request(email, password);
    let login_url = format!("{}/auth/v1/login", &app.address);

    let login_res = client
        .post(&login_url)
        .header("User-Agent", "test-client")
        .json(&login_req)
        .send()
        .await
        .expect("HTTP Login failed");

    let login_status = login_res.status().as_u16();
    let login_body = login_res.text().await.unwrap();
    assert_eq!(
        login_status, 200,
        "Login failed with status {}: {}",
        login_status, login_body
    );

    let login_json: Value = serde_json::from_str(&login_body).unwrap();
    let access_token = login_json["accessToken"].as_str().unwrap();

    let auth_service = get_auth_service(app.storage.clone(), &app.domain).await;
    let claims = auth_service.verify_access_token(access_token).unwrap();
    assert!(!claims.sub.is_empty());
}
