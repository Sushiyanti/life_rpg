//! Application state constructed once by the shell composition root.

use crate::SystemClock;
use lr_application::{
    ConceptService, HealthService, SemanticsService, TagService, TimelineService, WorldService,
};
use lr_persistence::SqliteHealthStore;
use std::sync::{Arc, RwLock};

pub type WiredStore = Arc<SqliteHealthStore>;
pub type WiredHealthService = HealthService<WiredStore, SystemClock>;
pub type WiredWorldService = WorldService<WiredStore, SystemClock>;
pub type WiredSemanticsService = SemanticsService<WiredStore, SystemClock>;
pub type WiredConceptService = ConceptService<WiredStore, SystemClock>;
pub type WiredTimelineService = TimelineService<WiredStore>;
pub type WiredTagService = TagService<WiredStore, SystemClock>;

pub struct AppState {
    pub health: WiredHealthService,
    pub world: WiredWorldService,
    pub semantics: WiredSemanticsService,
    pub concepts: WiredConceptService,
    pub timeline: WiredTimelineService,
    pub tags: WiredTagService,
    startup_warning: RwLock<Option<String>>,
    recovery_store: Option<WiredStore>,
}
impl AppState {
    pub fn new(
        health: WiredHealthService,
        world: WiredWorldService,
        semantics: WiredSemanticsService,
        concepts: WiredConceptService,
        timeline: WiredTimelineService,
        tags: WiredTagService,
    ) -> Self {
        Self {
            health,
            world,
            semantics,
            concepts,
            timeline,
            tags,
            startup_warning: RwLock::new(None),
            recovery_store: None,
        }
    }
    pub fn set_startup_warning(&mut self, warning: Option<String>) {
        *self.startup_warning.write().expect("warning lock") = warning;
    }
    pub fn startup_warning(&self) -> Option<String> {
        self.startup_warning.read().expect("warning lock").clone()
    }
    pub fn set_recovery_store(&mut self, store: WiredStore) {
        self.recovery_store = Some(store);
    }
    pub fn recovery_store(&self) -> Option<WiredStore> {
        self.recovery_store.clone()
    }
}
