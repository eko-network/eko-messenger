mod common;

use redis::Commands;

#[tokio::test]
async fn test_get_from_inbox() {
    let app = common::spawn_app().await;
    let client = reqwest::Client::new();

    let username = "testuser";
    let token = common::generate_test_token(username);

    // Manually populate the inbox
    let mut con = app
        .redis_client
        .get_connection()
        .expect("Failed to get Redis connection for setup.");

    let inbox_key = format!("inbox:{}", username);
    let message = r#"{"@context":"https://www.w3.org/ns/activitystreams","id":"http://example.com/activities/1","type":"Create","actor":"http://example.com/users/1","object":{"id":"http://example.com/notes/1","type":"Note","content":"Hello world"}}"#;

    let _: () = con
        .lpush(&inbox_key, message)
        .expect("Failed to push message to inbox.");

    let response = client
        .get(&format!("{}/api/v1/inbox", &app.address))
        .bearer_auth(token)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert - HTTP Response
    assert_eq!(response.status().as_u16(), 200);

    let messages: Vec<String> = response
        .json()
        .await
        .expect("Failed to parse response as JSON.");

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0], message);

    // Clean up
    let _: () = con.del(&inbox_key).expect("Failed to clean up inbox key.");
}

#[tokio::test]
async fn test_get_from_inbox_unauthenticated() {
    let app = common::spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/api/v1/inbox", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 401);
}
