pub mod activity;
pub mod actor;
pub mod collection;
pub mod eko_types;
pub mod serde_helpers;

pub use activity::{Activity, Create, Delivered, Take};
pub use actor::{Endpoints, Person, actor_uid, actor_url, create_person};
pub use collection::OrderedCollection;
pub use eko_types::{EncryptedMessage, EncryptedMessageEntry, PreKeyBundle};
pub use serde_helpers::{proof_condensor, single_item_vec, single_item_vec_borrowed};

pub const ACTIVITY_STREAMS_CONTEXT: &str = "https://www.w3.org/ns/activitystreams";
// TODO do we have an eko context link as well?
