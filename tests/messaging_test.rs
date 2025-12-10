mod common;

use common::{generate_login_request, spawn_app};
use eko_messenger::outbox::OutboxPayload;
use reqwest::Client;
use serde_json::Value;
use std::env;

#[tokio::test]
async fn test_send_and_receive_message_to_self() {
    let app = spawn_app().await;
    let client = Client::new();

    let email = env::var("TEST_USER_EMAIL").expect("TEST_USER_EMAIL not set");
    let password = env::var("TEST_USER_PASSWORD").expect("TEST_USER_PASSWORD not set");
    let username = email.split('@').next().unwrap();

    let login_req = generate_login_request(email.clone(), password.clone());
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
    let auth_token = login_json["accessToken"].as_str().unwrap();

    let outbox_url = format!("{}/api/v1/outbox", &app.address);
    let message_content = "test message to self".to_string();
    let outbox_payload = OutboxPayload {
        recipient_username: username.to_string(), // Sending to self
        content: message_content.clone(),
    };

    let outbox_res = client
        .post(&outbox_url)
        .bearer_auth(auth_token)
        .header("User-Agent", "test-client")
        .json(&outbox_payload)
        .send()
        .await
        .expect("Failed to send message");

    let outbox_status = outbox_res.status().as_u16();
    let outbox_body = outbox_res.text().await.unwrap();
    assert_eq!(
        outbox_status, 201,
        "Expected CREATED status, got {}: {}",
        outbox_status, outbox_body
    );

    let inbox_url = format!("{}/api/v1/inbox", &app.address);
    let inbox_res = client
        .get(&inbox_url)
        .bearer_auth(auth_token)
        .header("User-Agent", "test-client")
        .send()
        .await
        .expect("Failed to get inbox");

    let inbox_status = inbox_res.status().as_u16();
    let inbox_body = inbox_res.text().await.unwrap();
    assert_eq!(
        inbox_status, 200,
        "Expected OK status, got {}: {}",
        inbox_status, inbox_body
    );

    let inbox: Vec<Value> = serde_json::from_str(&inbox_body).unwrap();

    assert_eq!(inbox.len(), 1, "Expected 1 message in inbox");
    let received_activity = &inbox[0];
    assert_eq!(
        received_activity["type"], "Create",
        "Activity type mismatch"
    );

    let note = &received_activity["object"];
    assert_eq!(note["content"], message_content, "Message content mismatch");
    assert_eq!(
        note["attributedTo"],
        format!("https://{}/users/{}", app.domain, username),
        "AttributedTo mismatch"
    );
}
