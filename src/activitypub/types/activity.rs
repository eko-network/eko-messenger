use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::activitypub::PreKeyBundle;

use super::eko_types::EncryptedMessage;

fn default_context_value() -> Value {
    Value::String(super::ACTIVITY_STREAMS_CONTEXT.to_string())
}

mod single_item_vec {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut v: Vec<String> = Vec::deserialize(deserializer)?;
        if v.len() != 1 {
            return Err(D::Error::custom(format!(
                "expected exactly 1 item in 'to' list, found {}",
                v.len()
            )));
        }
        Ok(v.remove(0))
    }

    pub fn serialize<S>(value: &String, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        vec![value].serialize(serializer)
    }
}

pub trait ActivityBase {
    fn id(&self) -> Option<&String>;
    fn set_id(&mut self, id: String);
    fn actor(&self) -> &String;
    fn to(&self) -> &String;
}

macro_rules! impl_activity_base {
    ($($t:ty),*) => {
        $(
            impl ActivityBase for $t {
                fn id(&self) -> Option<&String> { self.id.as_ref() }
                fn set_id(&mut self, id: String) { self.id = Some(id); }
                fn actor(&self) -> &String { &self.actor }
                fn to(&self) -> &String { &self.to }
            }
        )*
    };
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Activity {
    Create(Create),
    Take(Take),
    Delivered(Delivered),
}

impl Activity {
    pub fn as_base(&self) -> &dyn ActivityBase {
        match self {
            Activity::Create(c) => c,
            Activity::Take(t) => t,
            Activity::Delivered(d) => d,
        }
    }

    pub fn as_base_mut(&mut self) -> &mut dyn ActivityBase {
        match self {
            Activity::Create(c) => c,
            Activity::Take(t) => t,
            Activity::Delivered(d) => d,
        }
    }
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Take {
    #[serde(rename = "@context")]
    pub context: Value,
    #[serde(default)]
    pub actor: String,
    pub to: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub result: Option<PreKeyBundle>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Delivered {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "@context")]
    pub context: Value,
    #[serde(default)]
    pub actor: String,
    pub to: String,
    pub object: String,
}

/// ActivityPub Create activity
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Create {
    #[serde(rename = "@context")]
    pub context: Value,
    #[serde(default)]
    pub id: Option<String>,
    pub actor: String,
    pub object: EncryptedMessage,
    pub to: String,
}
impl_activity_base!(Create, Take, Delivered);

// Helper function to generate a Create activity for messages
// pub fn generate_create(
//     to_actor: String,
//     from_actor: String,
//     to_did: String,
//     from_did: String,
//     content: Vec<u8>,
// ) -> Activity {
//     use super::eko_types::EncryptedMessageEntry;
//
//     Activity::Create(Create {
//         context: default_context_value(),
//         id: None,
//         actor: from_actor.clone(),
//         object: EncryptedMessage {
//             context: default_context_value(),
//             attributed_to: from_actor,
//             content: vec![EncryptedMessageEntry {
//                 to: to_did,
//                 from: from_did,
//                 content,
//             }],
//             id: None,
//             to: vec![to_actor],
//             type_field: "Note".to_string(),
//         },
//     })
// }
