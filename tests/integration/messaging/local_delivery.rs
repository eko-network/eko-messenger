use crate::common::*;

/// Test sending a message from one user to another on the same server
#[tokio::test]
async fn test_send_message_between_local_users() {
    let app = spawn_app().await;

    // Create two users
    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    // Alice sends a message to Bob
    let response = alice.send_message_to(&app, &bob, "Hello Bob!").await;

    // Verify the message was accepted
    assert_success(response).await;

    // Bob should have the message in his inbox
    let bob_inbox = bob.get_inbox(&app).await;
    assert_collection_size(&bob_inbox, 1);

    // The activity should be a Create activity
    let activity = &bob_inbox["orderedItems"][0];
    assert_activity_type(activity, "Create");

    // The actor should be Alice
    assert_field_equals(activity, "actor", &alice.actor_id.into());
}

/// Test sending a message to yourself
#[tokio::test]
async fn test_send_message_to_self() {
    // TODO (see other FIXMEs)
}

/// Test empty inbox for new user
#[tokio::test]
async fn test_new_user_has_empty_inbox() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;

    let inbox = alice.get_inbox(&app).await;
    assert_collection_size(&inbox, 0);
}

/// Test concurrent message delivery to same recipient
#[tokio::test]
async fn test_concurrent_messages_to_same_user() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    // Alice sends 3 messages rapidly
    let messages: Vec<String> = (1..=3).map(|i| format!("Message {}", i)).collect();
    let mut handles = vec![];
    for message in &messages {
        let fut = alice.send_message_to(&app, &bob, message);
        handles.push(fut);
    }

    // Wait for all to complete
    for handle in handles {
        let response = handle.await;
        assert_success(response).await;
    }

    // Bob should have all 3 messages
    let bob_inbox = bob.get_inbox(&app).await;
    assert_collection_size(&bob_inbox, 3);
}
