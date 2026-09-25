//! Application state shared by every command.
//!
//! `AppState` is the composition root's output: a fully wired
//! `HealthService<SqliteHealthStore, SystemClock>` plus a place to record why
//! the app fell back to an in-memory world.
//!
//! Tauri stores this behind `State<'_, AppState>`, so commands get it without
//! any globals or thread-locals. The service is `Send + Sync` (the connection is
//! behind a `Mutex`), which is required because commands may run on a worker
//! pool.

use std::sync::RwLock;

use lr_application::HealthService;
use lr_persistence::SqliteHealthStore;

use crate::SystemClock;

/// Concrete service type the shell wires up.
pub type WiredHealthService = HealthService<SqliteHealthStore, SystemClock>;

/// Everything a command needs.
pub struct AppState {
    /// The wired health/status use case.
    pub health: WiredHealthService,
    startup_warning: RwLock<Option<String>>,
}

impl AppState {
    /// Wrap a wired service.
    pub fn new(health: WiredHealthService) -> Self {
        Self {
            health,
            startup_warning: RwLock::new(None),
        }
    }

    /// Record a non-fatal startup problem (e.g. "fell back to in-memory").
    pub fn set_startup_warning(&mut self, warning: Option<String>) {
        *self.startup_warning.write().expect("warning lock") = warning;
    }

    /// Read the recorded startup problem, if any.
    pub fn startup_warning(&self) -> Option<String> {
        self.startup_warning
            .read()
            .expect("warning lock")
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lr_persistence::SqliteHealthStore;

    const T0: &str = "2026-09-25T00:00:00+00:00";

    fn state() -> AppState {
        AppState::new(HealthService::new(
            SqliteHealthStore::open_in_memory(T0),
            SystemClock,
        ))
    }

    #[test]
    fn starts_without_a_warning() {
        assert!(state().startup_warning().is_none());
    }

    #[test]
    fn warning_can_be_set_and_read() {
        let mut s = state();
        s.set_startup_warning(Some("disk went away".into()));
        assert_eq!(s.startup_warning().as_deref(), Some("disk went away"));

        s.set_startup_warning(None);
        assert!(s.startup_warning().is_none());
    }
}
