use crate::types::{KeyPackage, Object};
use serde::{Deserialize, Serialize};

use serde_json::Value;

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct Take {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    #[serde(rename = "@context")]
    pub context: Value,
    pub actor: String,
    pub to: Vec<String>,
    #[serde(default)]
    pub result: Option<KeyPackage>,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct Delivered {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    #[serde(rename = "@context")]
    pub context: Value,
    pub actor: String,
    pub to: Vec<String>,
    pub object: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Create {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    #[serde(rename = "@context")]
    pub context: Value,
    pub actor: String,
    pub object: Object,
    pub to: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]

pub enum Activity {
    Take(Take),
    Create(Create),
    Delivered(Delivered),
}

pub trait ActivityBase {
    fn id(&self) -> Option<&str>;
    fn actor(&self) -> &str;
    fn to(&self) -> &Vec<String>;
}

macro_rules! impl_activity_base {
    ($($t:ty),*) => {$(
        impl ActivityBase for $t {
            fn id(&self) -> Option<&str> { self.id.as_deref() }
            fn actor(&self) -> &str { &self.actor }
            fn to(&self) -> &Vec<String> { &self.to }
        }
        impl ActivityBaseMut for $t {
            fn set_id(&mut self, id: String) { self.id = Some(id); }
        }
    )*};
}

impl_activity_base!(Take, Create, Delivered);

impl ActivityBase for Activity {
    fn id(&self) -> Option<&str> {
        match self {
            Activity::Take(v) => v.id(),
            Activity::Create(v) => v.id(),
            Activity::Delivered(v) => v.id(),
        }
    }
    fn actor(&self) -> &str {
        match self {
            Activity::Take(v) => v.actor(),
            Activity::Create(v) => v.actor(),
            Activity::Delivered(v) => v.actor(),
        }
    }
    fn to(&self) -> &Vec<String> {
        match self {
            Activity::Take(v) => v.to(),
            Activity::Create(v) => v.to(),
            Activity::Delivered(v) => v.to(),
        }
    }
}

impl Activity {
    pub fn as_mut(&mut self) -> &mut dyn ActivityBaseMut {
        match self {
            Activity::Take(v) => v,
            Activity::Create(v) => v,
            Activity::Delivered(v) => v,
        }
    }
}

pub trait ActivityBaseMut: ActivityBase {
    fn set_id(&mut self, id: String);
}
