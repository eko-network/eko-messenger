pub mod traits;
pub mod postgres;

use std::sync::Arc;
use crate::storage::traits::*;

#[derive(Clone)]
pub struct Storage {
    pub inbox: Arc<dyn InboxStore>,
    pub outbox: Arc<dyn OutboxStore>,
    pub devices: Arc<dyn DeviceStore>,
    pub actors: Arc<dyn ActorStore>,
}
