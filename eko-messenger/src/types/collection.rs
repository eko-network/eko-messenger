use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::server::context;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderedCollection<T> {
    #[serde(rename = "@context")]
    pub context: Value,
    #[serde(rename = "type")]
    pub type_field: String,
    pub id: String,
    pub total_items: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ordered_items: Option<Vec<T>>,
}

impl<T> OrderedCollection<T> {
    pub fn new(id: String, items: Option<Vec<T>>) -> Self {
        // TODO probably should be ordered by sent time or smth
        let total = items.as_ref().map_or(0, |v| v.len());
        Self {
            context: context(),
            type_field: "OrderedCollection".to_string(),
            id,
            total_items: total,
            ordered_items: items,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection<T> {
    #[serde(rename = "@context")]
    pub context: Value,
    #[serde(rename = "type")]
    pub type_field: String, // Will be "Collection"
    pub id: String,
    pub total_items: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<T>>,
}

impl<T> Collection<T> {
    pub fn new(id: String, items: Option<Vec<T>>) -> Self {
        let total = items.as_ref().map_or(0, |v| v.len());
        Self {
            context: context(),
            type_field: "Collection".to_string(),
            id,
            total_items: total,
            items: items,
        }
    }
}
