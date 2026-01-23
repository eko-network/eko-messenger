pub mod activity;
pub mod actor;
pub mod eko_types;

pub use activity::{Activities, Create, generate_create};
pub use actor::{Person, actor_uid, actor_url, create_person};
pub use eko_types::{EncryptedMessage, EncryptedMessageEntry, PreKeyBundle};

pub const ACTIVITY_STREAMS_CONTEXT: &str = "https://www.w3.org/ns/activitystreams";
// TODO do we have an eko context link as well?
