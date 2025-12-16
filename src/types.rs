use ::serde::{Deserialize, Serialize};
use serde_with::base64::Base64;
use serde_with::serde_as;

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
