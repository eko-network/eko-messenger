use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::base64::Base64;
use serde_with::serde_as;

/// Represents an encrypted message in the Eko protocol
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

/// A single encrypted message entry for a specific device
#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptedMessageEntry {
    pub to: String,
    pub from: String,
    #[serde_as(as = "Base64")]
    pub content: Vec<u8>,
}

/// Prekey bundle for establishing encrypted sessions (Signal protocol)
#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreKeyBundle {
    pub did: String,
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
