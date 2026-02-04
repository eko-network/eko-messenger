use crate::activitypub::types::{proof_condensor, single_item_vec, single_item_vec_borrowed};
use crate::devices::DeviceId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::base64::Base64;
use serde_with::{hex::Hex, serde_as};

/// Represents an encrypted message in the Eko protocol
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedMessage {
    #[serde(rename = "@context")]
    pub context: Value,
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(default)]
    pub id: Option<String>,
    pub content: Vec<EncryptedMessageEntry>,
    pub attributed_to: String,
    #[serde(with = "single_item_vec")]
    pub to: String,
}

/// A single encrypted message entry for a specific device
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
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
    pub did: DeviceId,
    pub pre_key_id: i32,
    #[serde_as(as = "Base64")]
    pub pre_key: Vec<u8>,

    pub signed_pre_key_id: i32,
    #[serde_as(as = "Base64")]
    pub signed_pre_key: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub signed_pre_key_signature: Vec<u8>,
}

/// Device action types enum
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum DeviceAction {
    AddDevice(AddDevice),
    RevokeDevice(RevokeDevice),
}

/// Represents an AddDevice action in the Eko protocol
#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddDevice {
    #[serde(rename = "@context")]
    pub context: Value,
    pub id: String,
    #[serde_as(as = "Option<Hex>")]
    pub prev: Option<[u8; 32]>,
    pub did: String,
    pub key_collection: String,
    #[serde_as(as = "Base64")]
    pub identity_key: Vec<u8>,
    pub registration_id: i32,
    #[serde(with = "proof_condensor")]
    pub proof: Vec<DataIntegrityProof>,
}

/// Data Integrity Proof
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DataIntegrityProof {
    #[serde(rename = "type")]
    pub type_field: String,
    pub cryptosuite: String,
    pub verification_method: String,
    pub proof_purpose: String,
    pub proof_value: String,
}

/// Proof entry for RevokeDevice
#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceProof {
    pub did: String,
    pub signature: String,
}

/// Represents a RevokeDevice action in the Eko protocol
#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeDevice {
    #[serde(rename = "@context")]
    pub context: Value,
    pub id: String,
    pub did: String,
    #[serde_as(as = "Option<Hex>")]
    pub prev: Option<[u8; 32]>,
    pub proof: Vec<DeviceProof>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedMessageView<'a> {
    #[serde(rename = "@context")]
    pub context: &'a serde_json::Value,
    #[serde(rename = "type")]
    pub type_field: &'a str,
    pub id: Option<&'a str>,
    pub content: &'a [EncryptedMessageEntry],
    pub attributed_to: &'a str,
    #[serde(with = "single_item_vec_borrowed")]
    pub to: &'a str,
}
