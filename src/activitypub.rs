use serde_json::json;

pub fn create_actor(username: &str, domain: &str) -> serde_json::Value {
    let user_id = format!("http://{}/users/{}", domain, username);
    let inbox_url = format!("{}/inbox", user_id);
    let outbox_url = format!("{}/outbox", user_id);

    json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "id": user_id,
        "type": "Person",
        "preferredUsername": username,
        "inbox": inbox_url,
        "outbox": outbox_url
    })
}
