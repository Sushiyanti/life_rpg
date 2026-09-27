//! Application state constructed once by the shell composition root.

use crate::SystemClock;
use lr_application::{HealthService, WorldService};
use lr_persistence::SqliteHealthStore;
use std::sync::{Arc, RwLock};

pub type WiredStore = Arc<SqliteHealthStore>;
pub type WiredHealthService = HealthService<WiredStore, SystemClock>;
pub type WiredWorldService = WorldService<WiredStore, SystemClock>;

pub struct AppState {
    pub health: WiredHealthService,
    pub world: WiredWorldService,
    startup_warning: RwLock<Option<String>>,
}
impl AppState {
    pub fn new(health: WiredHealthService, world: WiredWorldService) -> Self {
        Self {
            health,
            world,
            startup_warning: RwLock::new(None),
        }
    }
    pub fn set_startup_warning(&mut self, warning: Option<String>) {
        *self.startup_warning.write().expect("warning lock") = warning;
    }
    pub fn startup_warning(&self) -> Option<String> {
        self.startup_warning.read().expect("warning lock").clone()
    }
}
