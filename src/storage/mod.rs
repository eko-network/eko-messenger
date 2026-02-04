pub mod models;
pub mod postgres;
pub mod traits;

use crate::storage::traits::*;
use std::sync::Arc;

#[derive(Clone)]
pub struct Storage {
    pub notifications: Arc<dyn NotificationStore>,
    pub activities: Arc<dyn ActivityStore>,
    pub devices: Arc<dyn DeviceStore>,
    pub actors: Arc<dyn ActorStore>,
    pub users: Arc<dyn UserStore>,
}
