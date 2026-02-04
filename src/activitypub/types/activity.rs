use crate::activitypub::types::single_item_vec;
use serde::{Deserialize, Serialize};

use serde_json::Value;

use crate::activitypub::PreKeyBundle;

use super::eko_types::EncryptedMessage;

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

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "activity_type", rename_all = "PascalCase")]
pub enum ActivityType {
    Create,
    Take,
    Delivered,
    Update,
    Reject,
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

    pub fn activity_type(&self) -> ActivityType {
        match self {
            Activity::Create(_) => ActivityType::Create,
            Activity::Take(_) => ActivityType::Take,
            Activity::Delivered(_) => ActivityType::Delivered,
        }
    }
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Take {
    #[serde(rename = "@context")]
    pub context: Value,
    #[serde(default)]
    pub actor: String,
    #[serde(with = "single_item_vec")]
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
    #[serde(with = "single_item_vec")]
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
    #[serde(with = "single_item_vec")]
    pub to: String,
}
impl_activity_base!(Create, Take, Delivered);
