mod common;

use common::spawn_app;
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::HeaderValue, protocol::Message},
};

#[tokio::test]
async fn test_websocket_connection_requires_authentication() {
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

#[tokio::test]
async fn test_websocket_connection_with_authentication() {
    let app = spawn_app().await;

    // First, login to get a token
    let email = "testuser@example.com";
    let password = "testpass123";
    let login_response = app.login_http(email, password).await;

    // Connect to WebSocket with authentication via Authorization header
    let ws_url = format!("{}/ws", app.address.replace("http://", "ws://"));

    // Create request with Authorization header
    let mut request = ws_url.into_client_request().unwrap();
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", login_response.access_token)).unwrap(),
    );

    // Connect with authentication
    let result = timeout(Duration::from_secs(2), connect_async(request)).await;

    // Connection should succeed with valid token
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

#[tokio::test]
async fn test_websocket_stays_connected() {
    let app = spawn_app().await;

    let email = "testuser2@example.com";
    let password = "testpass456";
    let login_response = app.login_http(email, password).await;

    // Connect with JWT in Authorization header
    let ws_url = format!("{}/ws", app.address.replace("http://", "ws://"));
    let mut request = ws_url.into_client_request().unwrap();
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", login_response.access_token)).unwrap(),
    );

    let (mut ws_stream, _) = connect_async(request).await.unwrap();

    // Keep connection open for a short duration
    let result = timeout(Duration::from_millis(500), ws_stream.next()).await;

    // Should timeout (no messages sent), meaning connection is stable
    assert!(
        result.is_err(),
        "WebSocket should stay connected without closing"
    );

    // Close cleanly
    ws_stream.close(None).await.unwrap();
}

#[tokio::test]
async fn test_websocket_disconnection() {
    let app = spawn_app().await;

    let email = "testuser3@example.com";
    let password = "testpass789";
    let login_response = app.login_http(email, password).await;

    // Connect with JWT
    let ws_url = format!("{}/ws", app.address.replace("http://", "ws://"));
    let mut request = ws_url.into_client_request().unwrap();
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", login_response.access_token)).unwrap(),
    );

    let (mut ws_stream, _) = connect_async(request).await.unwrap();

    // Send close frame
    ws_stream.close(None).await.unwrap();

    // Verify connection is closed - either None, a Close message, or connection reset
    let next_msg = ws_stream.next().await;
    match next_msg {
        None => {}                        // Connection closed cleanly
        Some(Ok(Message::Close(_))) => {} // Received close frame
        Some(Err(_)) => {}                // Connection reset/error (also indicates closure)
        other => panic!("Expected WebSocket to be closed, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_multiple_websocket_connections() {
    let app = spawn_app().await;

    // Login as two different users
    let user1_email = "user1@example.com";
    let user1_password = "pass1";
    let login1 = app.login_http(user1_email, user1_password).await;

    let user2_email = "user2@example.com";
    let user2_password = "pass2";
    let login2 = app.login_http(user2_email, user2_password).await;

    // Connect user 1
    let ws_url1 = format!("{}/ws", app.address.replace("http://", "ws://"));
    let mut request1 = ws_url1.into_client_request().unwrap();
    request1.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", login1.access_token)).unwrap(),
    );
    let (mut ws1, _) = connect_async(request1).await.unwrap();

    // Connect user 2
    let ws_url2 = format!("{}/ws", app.address.replace("http://", "ws://"));
    let mut request2 = ws_url2.into_client_request().unwrap();
    request2.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", login2.access_token)).unwrap(),
    );
    let (mut ws2, _) = connect_async(request2).await.unwrap();

    // Both connections should be active
    // Close first connection
    ws1.close(None).await.unwrap();

    // Second connection should still be active
    let result = timeout(Duration::from_millis(100), ws2.next()).await;
    assert!(result.is_err(), "Second WebSocket should still be open");

    // Cleanup
    ws2.close(None).await.unwrap();
}

#[tokio::test]
async fn test_websocket_ping_pong() {
    let app = spawn_app().await;

    let email = "pinguser@example.com";
    let password = "pingpass";
    let login_response = app.login_http(email, password).await;

    // Connect with JWT
    let ws_url = format!("{}/ws", app.address.replace("http://", "ws://"));
    let mut request = ws_url.into_client_request().unwrap();
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", login_response.access_token)).unwrap(),
    );

    let (mut ws_stream, _) = connect_async(request).await.unwrap();

    // Send a ping
    ws_stream.send(Message::Ping(vec![1, 2, 3])).await.unwrap();

    // Pong is automatically handled by the client, just verify connection stays alive
    let _result = timeout(Duration::from_millis(100), ws_stream.next()).await;
    // Either timeout or receive pong (depending on client implementation)

    // Cleanup
    ws_stream.close(None).await.unwrap();
}
