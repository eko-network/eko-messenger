use serde::{Deserialize, Serialize};
use serde_json::Value;

fn default_context_value() -> Value {
    Value::String(super::ACTIVITY_STREAMS_CONTEXT.to_string())
}

/// ActivityPub Person (Actor) type
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    #[serde(rename = "@context")]
    pub context: Value,
    #[serde(rename = "type")]
    pub type_field: String,
    pub id: String,
    pub inbox: String,
    pub outbox: String,
    pub key_bundle: String,
    pub preferred_username: String,
    pub profile_picture: Option<String>,
    pub summary: Option<String>,
    pub name: Option<String>,
}

/// Create a new Person actor
pub fn create_person(
    domain: &str,
    uid: &str,
    summary: Option<String>,
    preferred_username: String,
    name: Option<String>,
    profile_picture: Option<String>,
) -> Person {
    let id = actor_url(domain, uid);
    let inbox_url = format!("{}/inbox", id);
    let outbox_url = format!("{}/outbox", id);
    let key_bundles_url = format!("{}/keys/bundle.json", id);

    Person {
        context: default_context_value(),
        type_field: "Person".to_string(),
        id,
        inbox: inbox_url,
        outbox: outbox_url,
        key_bundle: key_bundles_url,
        summary,
        preferred_username,
        name,
        profile_picture,
    }
}

/// Generate an actor URL from domain and user ID
pub fn actor_url(domain: &str, uid: &str) -> String {
    format!("{}/users/{}", domain, uid)
}

/// Extract user ID from an actor URL
pub fn actor_uid(url: &str) -> anyhow::Result<String> {
    Ok(url
        .split('/')
        .filter(|v| !v.is_empty())
        .last()
        .ok_or(anyhow::anyhow!("unknown url format"))?
        .to_string())
}
