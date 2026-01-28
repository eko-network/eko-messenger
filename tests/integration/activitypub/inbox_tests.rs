use crate::common::*;

/// Test getting a user's inbox when authenticated
#[tokio::test]
async fn test_get_inbox_authenticated() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;

    let inbox = alice.get_inbox(&app).await;

    // TODO i wonder if i should create an inbox struct and make
    // sure it can deserialize instead of manually checking fields here

    // Should return an OrderedCollection
    assert_has_field(&inbox, "type");
    assert_eq!(inbox["type"], "OrderedCollection");
    assert_has_field(&inbox, "orderedItems");
    assert_has_field(&inbox, "totalItems");
    assert_eq!(inbox["totalItems"], 0);
}

/// Test getting inbox without authentication fails
#[tokio::test]
async fn test_get_inbox_unauthenticated_fails() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;

    let inbox_url = format!("{}/inbox", &alice.actor_id);

    let response = app
        .client
        .get(&inbox_url)
        .header("Accept", "application/activity+json")
        .send()
        .await
        .expect("Request failed");

    assert_status(response, 401).await;
}

/// Test inbox returns activities in reverse chronological order
#[tokio::test]
async fn test_inbox_multiple_messages() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    // Send 3 messages to bob
    for i in 1..=3 {
        alice
            .send_message_to(&app, &bob, &format!("Message {}", i))
            .await;
    }

    let inbox = bob.get_inbox(&app).await;
    dbg!(&inbox);
    assert_collection_size(&inbox, 3);
    assert_eq!(inbox["totalItems"], 3);

    // TODO Verify ordering?
}
