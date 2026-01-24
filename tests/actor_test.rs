mod common;

use common::spawn_app;
use serde_json::Value;

#[tokio::test]
async fn test_get_user_actor() {
    let app = spawn_app().await;
    let client = &app.client;
    let login = app.login_http("actor@example.com", "password").await;
    let uid = login.uid;

    let actor_url = format!("{}/users/{}", &app.address, &uid);

    let res = client
        .get(&actor_url)
        .header("User-Agent", "test-client")
        .send()
        .await
        .expect("Failed to get user actor");

    let status = res.status().as_u16();
    let body = res.text().await.unwrap();
    assert_eq!(status, 200, "Expected OK status, got {}: {}", status, body);

    let actor: Value = serde_json::from_str(&body).expect("Failed to parse actor response");

    assert_eq!(actor["type"], "Person", "Actor type should be Person");
    assert_eq!(
        actor["id"],
        format!("{}/users/{}", app.domain, uid),
        "Actor ID mismatch"
    );
    assert_eq!(
        actor["inbox"],
        format!("{}/users/{}/inbox", app.domain, uid),
        "Inbox URL mismatch"
    );
    assert_eq!(
        actor["outbox"],
        format!("{}/users/{}/outbox", app.domain, uid),
        "Outbox URL mismatch"
    );
    assert_eq!(
        actor["devices"],
        format!("{}/users/{}/deviceActions", app.domain, uid),
        "Key bundle URL mismatch"
    );
    assert_eq!(
        actor["preferredUsername"], uid,
        "Preferred username should match UID"
    );
    assert!(
        actor["@context"].is_string() || actor["@context"].is_array(),
        "Context should be present"
    );
}
