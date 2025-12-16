use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::base64::Base64;
use serde_with::serde_as;

#[derive(Debug, Serialize, Deserialize)]
pub struct WithId(pub String);

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NoId;

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
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
    context: Value,
    #[serde(rename = "type")]
    type_field: String,
    id: String,
    inbox: String,
    outbox: String,
    key_bundle: String,
    preferred_username: String,
}

pub fn create_person(uid: &str, domain: &str) -> Person {
    let id = actor_url(domain, uid);
    let inbox_url = format!("{}/inbox", id);
    let outbox_url = format!("{}/outbox", id);
    let key_bundles_url = format!("{}/keys/bundle.json", id);

    Person {
        context: Value::String("https://www.w3.org/ns/activitystreams".to_string()),
        type_field: "Person".to_string(),
        id: id,
        //FIXME
        preferred_username: uid.to_string(),
        inbox: inbox_url,
        outbox: outbox_url,
        key_bundle: key_bundles_url,
    }
}

pub fn actor_url(domain: &str, uid: &str) -> String {
    return format!("http://{}/users/{}", domain, uid);
}
