use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::base64::Base64;
use serde_with::serde_as;

use crate::errors::AppError;

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

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreKeyBundle {
    pub did: i32,
    #[serde_as(as = "Base64")]
    pub identity_key: Vec<u8>,
    pub registration_id: i32,

    pub pre_key_id: i32,
    #[serde_as(as = "Base64")]
    pub pre_key: Vec<u8>,

    pub signed_pre_key_id: i32,
    #[serde_as(as = "Base64")]
    pub signed_pre_key: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub signed_pre_key_signature: Vec<u8>,
}

pub fn actor_url(domain: &str, uid: &str) -> String {
    return format!("{}/users/{}", domain, uid);
}

pub fn actor_uid(url: &str) -> anyhow::Result<String> {
    Ok(url
        .split("/")
        .filter(|v| !v.is_empty())
        .last()
        .ok_or(anyhow::anyhow!("unknown url format"))?
        .to_string())
}

pub fn generate_create(
    to_actor: String,
    from_actor: String,
    to_did: i32,
    from_did: i32,
    content: Vec<u8>,
) -> CreateActivity<NoId> {
    CreateActivity {
        context: default_context_value(),
        type_field: "Create".to_string(),
        id: NoId,
        actor: from_actor.clone(),
        object: EncryptedMessage {
            context: default_context_value(),
            attributed_to: from_actor,
            content: vec![EncryptedMessageEntry {
                to: to_did,
                from: from_did,
                content: content,
            }],
            id: NoId,
            to: vec![to_actor],
            type_field: "Note".to_string(),
        },
    }
}
