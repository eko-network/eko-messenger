use crate::activitypub::types::eko_types::EncryptedMessageView;
use crate::activitypub::types::single_item_vec;
use crate::activitypub::types::single_item_vec_borrowed;
use serde::{Deserialize, Serialize};

use serde_json::Value;

use crate::activitypub::PreKeyBundle;

use super::eko_types::EncryptedMessage;

macro_rules! impl_activity_base {
    ($($variant:ty),*) => {
        $(
            impl ActivityBase for $variant {
                fn id(&self) -> Option<&str> { self.id.as_deref() }
                fn actor(&self) -> &str { &self.actor }
                fn to(&self) -> &str { &self.to }
            }

            impl ActivityBaseMut for $variant {
                fn set_id(&mut self, id: String) { self.id = Some(id); }
            }
        )*
    };
}

macro_rules! define_activities {
    ($($variant:ident),*) => {
        #[derive(Debug, sqlx::Type)]
        #[sqlx(type_name = "activity_type", rename_all = "PascalCase")]
        pub enum ActivityType {
            $($variant),*
        }

        #[derive(Debug, Deserialize, Serialize)]
        #[serde(tag = "type")]
        pub enum Activity {
            $( $variant($variant) ),*
        }

        impl Activity {
            pub fn activity_type(&self) -> ActivityType {
                match self {
                    $( Activity::$variant(_) => ActivityType::$variant, )*
                }
            }
        }
    };
}

macro_rules! delegate_activity {
    ($self:ident, $variant:ident, $inner:ident => $result:expr) => {
        match $self {
            Activity::Create($inner) => $result,
            Activity::Take($inner) => $result,
            Activity::Delivered($inner) => $result,
        }
    };
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
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateView<'a> {
    #[serde(rename = "@context")]
    pub context: &'a serde_json::Value,
    pub id: Option<&'a str>,
    pub actor: &'a str,
    pub object: EncryptedMessageView<'a>,
    #[serde(with = "single_item_vec_borrowed")]
    pub to: &'a str,
    #[serde(rename = "type")]
    pub type_field: &'static str,
}

// Create enum
define_activities!(Create, Delivered, Take);

pub trait ActivityBase {
    fn id(&self) -> Option<&str>;
    fn actor(&self) -> &str;
    fn to(&self) -> &str;
}

pub trait ActivityBaseMut: ActivityBase {
    fn set_id(&mut self, id: String);
}

// add traits to variants
impl_activity_base!(Create, Take, Delivered);

impl Activity {
    pub fn as_base(&self) -> &dyn ActivityBase {
        delegate_activity!(self, variant, inner => inner)
    }

    pub fn as_base_mut(&mut self) -> &mut dyn ActivityBaseMut {
        delegate_activity!(self, variant, inner => inner)
    }
}

impl ActivityBase for Activity {
    fn id(&self) -> Option<&str> {
        self.as_base().id()
    }
    fn actor(&self) -> &str {
        self.as_base().actor()
    }
    fn to(&self) -> &str {
        self.as_base().to()
    }
}

impl ActivityBaseMut for Activity {
    fn set_id(&mut self, id: String) {
        self.as_base_mut().set_id(id);
    }
}

impl<'a> ActivityBase for CreateView<'a> {
    fn id(&self) -> Option<&str> {
        self.id
    }
    fn actor(&self) -> &str {
        self.actor
    }
    fn to(&self) -> &str {
        self.to
    }
}
