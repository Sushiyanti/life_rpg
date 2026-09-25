//! # `lr-persistence` — the PERSISTENCE layer (adapter)
//!
//! This crate answers **"how is everything saved?"** and it is the *only* place
//! in the codebase allowed to know that SQLite exists.
//!
//! Responsibilities:
//!
//! * Open a local SQLite file and configure it ([`pragma`]).
//! * Run versioned, embedded migrations and keep a ledger ([`migrations`]).
//! * Implement the application's ports ([`sqlite_store::SqliteHealthStore`]
//!   implements `lr_application::HealthStore`).
//!
//! Boundaries that matter:
//!
//! * `rusqlite::Error` never escapes this crate — it is translated into
//!   [`lr_application::StorageError`] at the edge.
//! * Because `bundled` is enabled, the SQLite C library is compiled into the
//!   binary. No system `libsqlite3`, no network, nothing to install on the
//!   user's machine — which is exactly what "offline-first local desktop app"
//!   requires.
//!
//! Data location: the caller supplies the path (the desktop shell passes
//! Tauri's `app_data_dir`), so this crate stays testable — tests point it at a
//! temp directory or at `:memory:`.

pub mod error;
pub mod migrations;
pub mod pragma;
pub mod sqlite_store;

pub use error::PersistenceError;
pub use migrations::{applied_version, run_migrations, Migration, MIGRATIONS};
pub use sqlite_store::SqliteHealthStore;

/// Human-readable name of this layer, used by status reporting.
pub const LAYER_NAME: &str = "persistence";
