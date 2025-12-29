use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::base64::Base64;
use serde_with::serde_as;

const ACTIVITY_STREAMS_CONTEXT: &str = "https://www.w3.org/ns/activitystreams";

fn default_context_value() -> Value {
    Value::String(ACTIVITY_STREAMS_CONTEXT.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithId(pub String);

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NoId;

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedMessage<Id> {
    #[serde(rename = "@context")]
    pub context: Value,
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(default)]
    pub id: Id,
    pub content: Vec<EncryptedMessageEntry>,
    pub attributed_to: String,
    pub to: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct CreateActivity<Id> {
    #[serde(rename = "@context")]
    pub context: Value,
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(default)]
    pub id: Id,
    pub actor: String,
    pub object: EncryptedMessage<Id>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptedMessageEntry {
    pub to: i32,
    pub from: i32,
    #[serde_as(as = "Base64")]
    pub content: Vec<u8>,
}

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
        id: id,
        inbox: inbox_url,
        outbox: outbox_url,
        //FIXME
        key_bundle: key_bundles_url,
        summary,
        preferred_username,
        name,
        profile_picture,
    }
}

pub fn actor_url(domain: &str, uid: &str) -> String {
    return format!("http://{}/users/{}", domain, uid);
}
