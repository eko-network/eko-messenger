use crate::common::*;
use futures_util::{StreamExt};
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::HeaderValue},
};

/// Test that WebSocket connection requires authentication
#[tokio::test]
async fn test_websocket_requires_authentication() {
    let app = spawn_app().await;

    // Try connecting without authentication token
    let ws_url = format!("{}/ws", app.address.replace("http://", "ws://"));
    let result = timeout(Duration::from_secs(2), connect_async(&ws_url)).await;

    // Connection should fail without authentication
    assert!(
        result.is_err() || result.unwrap().is_err(),
        "WebSocket connection should fail without authentication"
    );
}

/// Test that WebSocket connection succeeds with valid authentication
#[tokio::test]
async fn test_websocket_with_authentication() {
    let app = spawn_app().await;

    // Create user and get token
    let alice = TestUser::create(&app, "alice").await;

    // Connect to WebSocket with authentication
    let ws_url = format!("{}/ws", app.address.replace("http://", "ws://"));
    let mut request = ws_url.into_client_request().unwrap();
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", alice.token)).unwrap(),
    );

    // Connection should succeed
    let result = timeout(Duration::from_secs(2), connect_async(request)).await;
    assert!(
        result.is_ok(),
        "WebSocket connection with valid token should succeed"
    );

    let ws_result = result.unwrap();
    assert!(
        ws_result.is_ok(),
        "WebSocket handshake should complete successfully"
    );

    let (mut ws_stream, _) = ws_result.unwrap();

    // Close the connection cleanly
    ws_stream.close(None).await.unwrap();
}

/// Test that WebSocket connection stays open (doesn't disconnect immediately)
#[tokio::test]
async fn test_websocket_stays_connected() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;

    // Connect with authentication
    let ws_url = format!("{}/ws", app.address.replace("http://", "ws://"));
    let mut request = ws_url.into_client_request().unwrap();
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", alice.token)).unwrap(),
    );

    let (mut ws_stream, _) = connect_async(request).await.unwrap();

    // Wait briefly and verify no unexpected disconnect
    let result = timeout(Duration::from_millis(500), ws_stream.next()).await;

    // Should timeout (no messages sent), meaning connection is stable
    assert!(
        result.is_err(),
        "WebSocket should stay connected without closing"
    );

    // Close cleanly
    ws_stream.close(None).await.unwrap();
}

/// Test receiving a message via WebSocket when another user sends you a message
#[tokio::test]
async fn test_receive_message_via_websocket() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    // Bob connects to WebSocket
    let ws_url = format!("{}/ws", app.address.replace("http://", "ws://"));
    let mut request = ws_url.into_client_request().unwrap();
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", bob.token)).unwrap(),
    );

    let (mut ws_stream, _) = connect_async(request).await.unwrap();

    // Alice sends a message to Bob
    alice.send_message_to(&app, &bob, "Hello Bob!").await;

    // Bob should receive the message via WebSocket
    let result = timeout(Duration::from_secs(2), ws_stream.next()).await;

    assert!(result.is_ok(), "Should receive message via WebSocket");

    let message = result.unwrap();
    assert!(message.is_some(), "WebSocket message should exist");

    // Clean up
    ws_stream.close(None).await.unwrap();
}

/// Test WebSocket connection with invalid token fails
#[tokio::test]
async fn test_websocket_with_invalid_token() {
    let app = spawn_app().await;

    let ws_url = format!("{}/ws", app.address.replace("http://", "ws://"));
    let mut request = ws_url.into_client_request().unwrap();
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str("Bearer invalid-token").unwrap(),
    );

    let result = timeout(Duration::from_secs(2), connect_async(request)).await;

    // Should fail with invalid token
    assert!(
        result.is_err() || result.unwrap().is_err(),
        "WebSocket connection with invalid token should fail"
    );
}
