mod common;

use redis::Commands;
use serde_json::json;

#[tokio::test]
async fn test_post_to_outbox() {
    let app = common::spawn_app().await;
    let client = reqwest::Client::new();

    let sender_username = common::generate_user_id();
    let recipient_username = common::generate_user_id();
    let message_content = "Hello from an integration test!";

    let token = common::generate_test_token(&sender_username);

    let payload = json!({
        "sender_username": sender_username,
        "recipient_username": recipient_username,
        "content": message_content
    });

    let response = client
        .post(&format!("{}/api/v1/outbox", &app.address))
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert - HTTP Response
    assert_eq!(response.status().as_u16(), 201);

    // Assert - Database State
    let mut con = app
        .redis_client
        .get_connection()
        .expect("Failed to get Redis connection for assertions.");

    let outbox_key = format!("outbox:{}", sender_username);
    let messages: Vec<String> = con
        .lrange(&outbox_key, 0, 0)
        .expect("Failed to retrieve messages from Redis.");

    assert_eq!(messages.len(), 1, "Expected one message in the outbox.");

    let stored_activity: serde_json::Value =
        serde_json::from_str(&messages[0]).expect("Failed to parse stored activity as JSON.");

    assert_eq!(stored_activity["type"], "Create");
    assert_eq!(
        stored_activity["actor"],
        format!("http://{}/users/{}", app.domain, sender_username)
    );
    assert_eq!(stored_activity["object"]["type"], "Note");
    assert_eq!(stored_activity["object"]["content"], message_content);
    assert_eq!(
        stored_activity["object"]["attributedTo"],
        format!("http://{}/users/{}", app.domain, sender_username)
    );
    assert_eq!(
        stored_activity["object"]["to"][0],
        format!("http://{}/users/{}", app.domain, recipient_username)
    );

    // Assert - Recipient's Inbox State
    let inbox_key = format!("inbox:{}", recipient_username);
    let recipient_messages: Vec<String> = con
        .lrange(&inbox_key, 0, 0)
        .expect("Failed to retrieve messages from recipient's inbox.");

    assert_eq!(
        recipient_messages.len(),
        1,
        "Expected one message in the recipient's inbox."
    );
    let stored_recipient_activity: serde_json::Value = serde_json::from_str(&recipient_messages[0])
        .expect("Failed to parse stored recipient activity as JSON.");

    assert_eq!(stored_recipient_activity["type"], "Create");
    assert_eq!(
        stored_recipient_activity["actor"],
        format!("http://{}/users/{}", app.domain, sender_username)
    );
    assert_eq!(stored_recipient_activity["object"]["type"], "Note");
    assert_eq!(
        stored_recipient_activity["object"]["content"],
        message_content
    );
    assert_eq!(
        stored_recipient_activity["object"]["attributedTo"],
        format!("http://{}/users/{}", app.domain, sender_username)
    );
    assert_eq!(
        stored_recipient_activity["object"]["to"][0],
        format!("http://{}/users/{}", app.domain, recipient_username)
    );

    // Clean up
    let _: () = con
        .del(&[&outbox_key, &inbox_key])
        .expect("Failed to clean up recipient's inbox key.");
}

#[tokio::test]
async fn test_post_to_outbox_unauthenticated() {
    let app = common::spawn_app().await;
    let client = reqwest::Client::new();

    let sender_username = "testuser";
    let recipient_username = "recipient";
    let message_content = "Hello from an integration test!";

    let payload = json!({
        "sender_username": sender_username,
        "recipient_username": recipient_username,
        "content": message_content
    });

    let response = client
        .post(&format!("{}/api/v1/outbox", &app.address))
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 401);
}
