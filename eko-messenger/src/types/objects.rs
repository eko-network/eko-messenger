use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::base64::Base64;
use serde_with::serde_as;

#[serde_as]
#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct PrivateMessage {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    #[serde(rename = "@context")]
    pub context: Value,
    pub actor: String,
    pub to: Vec<String>,
    #[serde_as(as = "Base64")]
    pub content: Vec<u8>,
}

#[serde_as]
#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct WelcomeMessage {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    #[serde(rename = "@context")]
    pub context: Value,
    pub actor: String,
    pub to: Vec<String>,
    #[serde_as(as = "Base64")]
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Object {
    PrivateMessage(PrivateMessage),
    WelcomeMessage(WelcomeMessage),
}

pub trait ObjectBase {
    fn context(&self) -> &Value;
    fn id(&self) -> Option<&str>;
    fn actor(&self) -> &str;
    fn to(&self) -> &Vec<String>;
    fn content(&self) -> &Vec<u8>;
}

pub trait ObjectBaseMut: ObjectBase {
    fn set_id(&mut self, id: String);
}

macro_rules! impl_has_wire_object {
    ($($t:ty),*) => {$(
        impl ObjectBase for $t {
            fn context(&self) -> &Value { &self.context }
            fn id(&self) -> Option<&str> { self.id.as_deref() }
            fn actor(&self) -> &str { &self.actor }
            fn to(&self) -> &Vec<String> { &self.to }
            fn content(&self) -> &Vec<u8> { &self.content }
        }

        impl ObjectBaseMut for $t {
            fn set_id(&mut self, id: String) { self.id = Some(id); }
        }
    )*};
}

impl_has_wire_object!(PrivateMessage, WelcomeMessage);

impl ObjectBase for Object {
    fn context(&self) -> &Value {
        match self {
            Object::PrivateMessage(v) => v.context(),
            Object::WelcomeMessage(v) => v.context(),
        }
    }

    fn id(&self) -> Option<&str> {
        match self {
            Object::PrivateMessage(v) => v.id(),
            Object::WelcomeMessage(v) => v.id(),
        }
    }

    fn actor(&self) -> &str {
        match self {
            Object::PrivateMessage(v) => v.actor(),
            Object::WelcomeMessage(v) => v.actor(),
        }
    }

    fn to(&self) -> &Vec<String> {
        match self {
            Object::PrivateMessage(v) => v.to(),
            Object::WelcomeMessage(v) => v.to(),
        }
    }

    fn content(&self) -> &Vec<u8> {
        match self {
            Object::PrivateMessage(v) => v.content(),
            Object::WelcomeMessage(v) => v.content(),
        }
    }
}
