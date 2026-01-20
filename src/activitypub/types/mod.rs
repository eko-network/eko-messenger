pub mod actor;
pub mod activity;
pub mod eko_types;

pub use actor::{Person, actor_url, actor_uid, create_person};
pub use activity::{CreateActivity, WithId, NoId, generate_create};
pub use eko_types::{EncryptedMessage, EncryptedMessageEntry, PreKeyBundle};

pub const ACTIVITY_STREAMS_CONTEXT: &str = "https://www.w3.org/ns/activitystreams";
// TODO do we have an eko context link as well?