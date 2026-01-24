use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::eko_types::EncryptedMessage;

fn default_context_value() -> Value {
    Value::String(super::ACTIVITY_STREAMS_CONTEXT.to_string())
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Activity {
    Create(Create),
    Take(Take),
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Take {
    #[serde(rename = "@context")]
    pub context: Value,
    #[serde(default)]
    pub actor: String,
    pub target: String,
}

/// ActivityPub Create activity
#[derive(Serialize, Deserialize, Debug)]
pub struct Create {
    #[serde(rename = "@context")]
    pub context: Value,
    #[serde(default)]
    pub id: Option<String>,
    pub actor: String,
    pub object: EncryptedMessage,
}

/// Helper function to generate a Create activity for messages
pub fn generate_create(
    to_actor: String,
    from_actor: String,
    to_did: String,
    from_did: String,
    content: Vec<u8>,
) -> Activity {
    use super::eko_types::EncryptedMessageEntry;

    Activity::Create(Create {
        context: default_context_value(),
        id: None,
        actor: from_actor.clone(),
        object: EncryptedMessage {
            context: default_context_value(),
            attributed_to: from_actor,
            content: vec![EncryptedMessageEntry {
                to: to_did,
                from: from_did,
                content,
            }],
            id: None,
            to: vec![to_actor],
            type_field: "Note".to_string(),
        },
    })
}
