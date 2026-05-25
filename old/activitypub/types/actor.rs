use serde::{Deserialize, Serialize};
use serde_json::Value;

pub fn default_context_value() -> Value {
    Value::String(super::ACTIVITY_STREAMS_CONTEXT.to_string())
}

/// ActivityPub Actor endpoints
/// Contains additional endpoints which may be useful for this actor
/// Only populated for the owning user when authenticated
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoints {
    /// URL to the user's encrypted group state collection
    pub groups: String,
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
    pub devices: String,
    pub preferred_username: String,
    pub profile_picture: Option<String>,
    pub summary: Option<String>,
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Endpoints>,
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
    let devices_url = format!("{}/deviceActions", id);

    Person {
        context: default_context_value(),
        type_field: "Person".to_string(),
        id,
        inbox: inbox_url,
        outbox: outbox_url,
        devices: devices_url,
        summary,
        preferred_username,
        name,
        profile_picture,
        endpoints: None,
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
        .rfind(|v| !v.is_empty())
        .ok_or(anyhow::anyhow!("unknown url format"))?
        .to_string())
}
