use crate::activitypub::types::ACTIVITY_STREAMS_CONTEXT;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderedCollection<T> {
    #[serde(rename = "@context")]
    pub context: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub id: String,
    pub total_items: usize,
    pub ordered_items: Vec<T>,
}

impl<T> OrderedCollection<T> {
    pub fn new(id: String, items: Vec<T>) -> Self {
        // TODO probably should be ordered by sent time or smth
        let total = items.len();
        Self {
            context: ACTIVITY_STREAMS_CONTEXT.to_string(),
            type_field: "OrderedCollection".to_string(),
            id,
            total_items: total,
            ordered_items: items,
        }
    }
}
