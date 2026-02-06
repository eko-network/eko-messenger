#![cfg(feature = "auth-firebase")]

use crate::auth_providers::helpers::*;
use crate::common::*;
use eko_messenger::auth::Firebase;
use std::sync::Arc;

//
// DIRECT PROVIDER TESTS (Unit Tests)
// These test Firebase provider methods directly without HTTP
//

/// Test that Firebase provider can be initialized from environment variables
#[tokio::test]
async fn test_firebase_provider_initialization() {
    let domain = Arc::new("http://test.example.com".to_string());
    let client = reqwest::Client::new();

    let _firebase = Firebase::new_from_env(domain.clone(), client)
        .await
        .expect("Firebase provider should initialize from env vars");

    // If we got here, provider initialized successfully
}

/// Test Firebase login with email and password (direct provider call)
#[tokio::test]
async fn test_firebase_login_with_email() {
    let creds = TestCredentials::from_env();
    let domain = Arc::new("http://test.example.com".to_string());
    let client = reqwest::Client::new();

    let firebase = Firebase::new_from_env(domain.clone(), client)
        .await
        .expect("Failed to create Firebase provider");

    let (person, uid) = firebase
        .login_with_email(creds.email.clone(), creds.password.clone())
        .await
        .expect("Firebase login should succeed with valid credentials");

    // Verify we got a Person and UID back
    assert!(!uid.is_empty(), "UID should not be empty");
    assert_eq!(person.type_field, "Person");
    assert!(!person.id.is_empty(), "Person ID should not be empty");
    assert!(
        !person.preferred_username.is_empty(),
        "Person should have a username"
    );
}

/// Test fetching Person from Firebase by UID
#[tokio::test]
async fn test_firebase_person_from_uid() {
    let creds = TestCredentials::from_env();
    let domain = Arc::new("http://test.example.com".to_string());
    let client = reqwest::Client::new();

    let firebase = Firebase::new_from_env(domain.clone(), client)
        .await
        .expect("Failed to create Firebase provider");

    // First login to get a valid UID
    let (_person, uid) = firebase
        .login_with_email(creds.email.clone(), creds.password.clone())
        .await
        .expect("Firebase login should succeed");

    // Now fetch Person by UID
    let person = firebase
        .person_from_uid(&uid)
        .await
        .expect("Should be able to fetch Person by UID");

    assert_eq!(person.type_field, "Person");
    assert!(!person.id.is_empty());
    assert!(!person.preferred_username.is_empty());
}

/// Test fetching UID from Firebase by username
#[tokio::test]
async fn test_firebase_uid_from_username() {
    let creds = TestCredentials::from_env();
    let domain = Arc::new("http://test.example.com".to_string());
    let client = reqwest::Client::new();

    let firebase = Firebase::new_from_env(domain.clone(), client)
        .await
        .expect("Failed to create Firebase provider");

    // First login to get username and UID
    let (person, expected_uid) = firebase
        .login_with_email(creds.email.clone(), creds.password.clone())
        .await
        .expect("Firebase login should succeed");

    let username = person.preferred_username.clone();

    // Now lookup UID by username
    let uid = firebase
        .uid_from_username(&username)
        .await
        .expect("Should be able to fetch UID by username");

    assert_eq!(uid, expected_uid, "UID should match");
}

/// Test Firebase login with invalid credentials
#[tokio::test]
async fn test_firebase_invalid_credentials() {
    let creds = TestCredentials::from_env();
    let domain = Arc::new("http://test.example.com".to_string());
    let client = reqwest::Client::new();

    let firebase = Firebase::new_from_env(domain.clone(), client)
        .await
        .expect("Failed to create Firebase provider");

    let result = firebase
        .login_with_email(creds.email.clone(), "wrong-password".to_string())
        .await;

    assert!(
        result.is_err(),
        "Login should fail with invalid credentials"
    );
}

//
// HTTP INTEGRATION TESTS
// These test the full HTTP flow through production endpoints
//

/// Test Firebase login via HTTP endpoint
#[tokio::test]
async fn test_firebase_http_login_flow() {
    let app = spawn_app_with_firebase().await;
    let creds = TestCredentials::from_env();

    let login_req = app.generate_login_request(creds.email, creds.password, None);

    let response = app
        .client
        .post(&format!("{}/auth/v1/login", app.address))
        .header("User-Agent", "test-agent")
        .json(&login_req)
        .send()
        .await
        .expect("Login request should succeed");

    assert!(
        response.status().is_success(),
        "Login should return 200 OK, got: {}",
        response.status()
    );

    let login_response: eko_messenger::auth::LoginResponse = response
        .json()
        .await
        .expect("Response should be valid LoginResponse");

    assert_valid_login_response(&login_response);

    // Verify the access token is valid
    let claims = assert_valid_access_token(&app.sessions, &login_response.access_token);
    assert!(!claims.sub.is_empty(), "Token should have subject");
}

/// Test Firebase login followed by token refresh
#[tokio::test]
async fn test_firebase_http_login_and_refresh() {
    let app = spawn_app_with_firebase().await;
    let creds = TestCredentials::from_env();

    // Step 1: Login
    let login_req = app.generate_login_request(creds.email, creds.password, None);

    let login_response: eko_messenger::auth::LoginResponse = app
        .client
        .post(&format!("{}/auth/v1/login", app.address))
        .header("User-Agent", "test-agent")
        .json(&login_req)
        .send()
        .await
        .expect("Login should succeed")
        .json()
        .await
        .expect("Should parse LoginResponse");

    let first_claims = assert_valid_access_token(&app.sessions, &login_response.access_token);

    // Step 2: Refresh token
    let refresh_req = eko_messenger::auth::RefreshRequest {
        refresh_token: login_response.refresh_token.clone(),
    };

    let refresh_response: eko_messenger::auth::RefreshResponse = app
        .client
        .post(&format!("{}/auth/v1/refresh", app.address))
        .header("User-Agent", "test-agent")
        .json(&refresh_req)
        .send()
        .await
        .expect("Refresh request should succeed")
        .json()
        .await
        .expect("Should parse RefreshResponse");

    assert!(
        !refresh_response.access_token.is_empty(),
        "Refresh should return new access token"
    );

    // Verify new token is valid and has same user
    let new_claims = assert_valid_access_token(&app.sessions, &refresh_response.access_token);
    assert_eq!(
        new_claims.sub, first_claims.sub,
        "Refreshed token should have same user"
    );
}

/// Test Firebase logout invalidates refresh token
#[tokio::test]
async fn test_firebase_http_logout() {
    let app = spawn_app_with_firebase().await;
    let creds = TestCredentials::from_env();

    // Step 1: Login
    let login_req = app.generate_login_request(creds.email, creds.password, None);

    let login_response: eko_messenger::auth::LoginResponse = app
        .client
        .post(&format!("{}/auth/v1/login", app.address))
        .header("User-Agent", "test-agent")
        .json(&login_req)
        .send()
        .await
        .expect("Login should succeed")
        .json()
        .await
        .expect("Should parse LoginResponse");

    assert_valid_login_response(&login_response);

    // Step 2: Logout
    let logout_req = eko_messenger::auth::LogoutRequest {
        refresh_token: login_response.refresh_token.clone(),
    };

    let logout_response = app
        .client
        .post(&format!("{}/auth/v1/logout", app.address))
        .header("User-Agent", "test-agent")
        .header(
            "Authorization",
            format!("Bearer {}", login_response.access_token),
        )
        .json(&logout_req)
        .send()
        .await
        .expect("Logout request should succeed");

    assert!(
        logout_response.status().is_success(),
        "Logout should succeed"
    );

    // Step 3: Try to refresh - should fail
    let refresh_req = eko_messenger::auth::RefreshRequest {
        refresh_token: login_response.refresh_token,
    };

    let refresh_response = app
        .client
        .post(&format!("{}/auth/v1/refresh", app.address))
        .header("User-Agent", "test-agent")
        .json(&refresh_req)
        .send()
        .await
        .expect("Refresh request should complete");

    assert_eq!(
        refresh_response.status(),
        401,
        "Refresh should fail after logout with 401 Unauthorized"
    );
}

/// Test Firebase login with invalid credentials via HTTP
#[tokio::test]
async fn test_firebase_http_invalid_credentials() {
    let app = spawn_app_with_firebase().await;
    let creds = TestCredentials::from_env();

    let login_req = app.generate_login_request(creds.email, "wrong-password".to_string(), None);

    let response = app
        .client
        .post(&format!("{}/auth/v1/login", app.address))
        .header("User-Agent", "test-agent")
        .json(&login_req)
        .send()
        .await
        .expect("Request should complete");

    assert_eq!(
        response.status(),
        401,
        "Login with wrong password should return 401 Unauthorized"
    );
}

/// Test Firebase login with multiple devices
#[tokio::test]
async fn test_firebase_http_multiple_devices() {
    let app = spawn_app_with_firebase().await;
    let creds = TestCredentials::from_env();

    // Device 1 login
    let login_req1 =
        app.generate_login_request(creds.email.clone(), creds.password.clone(), Some("device1"));

    let login_response1: eko_messenger::auth::LoginResponse = app
        .client
        .post(&format!("{}/auth/v1/login", app.address))
        .header("User-Agent", "test-agent")
        .json(&login_req1)
        .send()
        .await
        .expect("Device 1 login should succeed")
        .json()
        .await
        .expect("Should parse LoginResponse");

    assert_valid_login_response(&login_response1);

    // Device 2 login
    let login_req2 =
        app.generate_login_request(creds.email.clone(), creds.password.clone(), Some("device2"));

    let login_response2: eko_messenger::auth::LoginResponse = app
        .client
        .post(&format!("{}/auth/v1/login", app.address))
        .header("User-Agent", "test-agent")
        .json(&login_req2)
        .send()
        .await
        .expect("Device 2 login should succeed")
        .json()
        .await
        .expect("Should parse LoginResponse");

    assert_valid_login_response(&login_response2);

    // Both tokens should be valid
    let claims1 = assert_valid_access_token(&app.sessions, &login_response1.access_token);
    let claims2 = assert_valid_access_token(&app.sessions, &login_response2.access_token);

    // Same user, different refresh tokens
    assert_eq!(
        claims1.sub, claims2.sub,
        "Both devices should have same user"
    );
    assert_ne!(
        login_response1.refresh_token, login_response2.refresh_token,
        "Different devices should have different refresh tokens"
    );

    // Logout device 1
    let logout_req = eko_messenger::auth::LogoutRequest {
        refresh_token: login_response1.refresh_token.clone(),
    };

    app.client
        .post(&format!("{}/auth/v1/logout", app.address))
        .header("User-Agent", "test-agent")
        .header(
            "Authorization",
            format!("Bearer {}", login_response1.access_token),
        )
        .json(&logout_req)
        .send()
        .await
        .expect("Logout should succeed");

    // Device 1 refresh should fail
    let refresh_req1 = eko_messenger::auth::RefreshRequest {
        refresh_token: login_response1.refresh_token,
    };

    let refresh_response1 = app
        .client
        .post(&format!("{}/auth/v1/refresh", app.address))
        .header("User-Agent", "test-agent")
        .json(&refresh_req1)
        .send()
        .await
        .expect("Request should complete");

    assert_eq!(
        refresh_response1.status(),
        401,
        "Device 1 refresh should fail after logout"
    );

    // Device 2 refresh should still work
    let refresh_req2 = eko_messenger::auth::RefreshRequest {
        refresh_token: login_response2.refresh_token,
    };

    let refresh_response2 = app
        .client
        .post(&format!("{}/auth/v1/refresh", app.address))
        .header("User-Agent", "test-agent")
        .json(&refresh_req2)
        .send()
        .await
        .expect("Request should complete");

    assert!(
        refresh_response2.status().is_success(),
        "Device 2 refresh should still work"
    );
}
