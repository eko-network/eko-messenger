use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::eko_types::EncryptedMessage;

fn default_context_value() -> Value {
    Value::String(super::ACTIVITY_STREAMS_CONTEXT.to_string())
}

/// Marker type for activities that have an ID
#[derive(Debug, Serialize, Deserialize)]
pub struct WithId(pub String);

/// Marker type for activities without an ID
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NoId;

/// ActivityPub Create activity
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

/// Helper function to generate a Create activity for messages
pub fn generate_create(
    to_actor: String,
    from_actor: String,
    to_did: i32,
    from_did: i32,
    content: Vec<u8>,
) -> CreateActivity<NoId> {
    use super::eko_types::EncryptedMessageEntry;
    
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
                content,
            }],
            id: NoId,
            to: vec![to_actor],
            type_field: "Note".to_string(),
        },
    }
}
