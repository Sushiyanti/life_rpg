//! # `lr-application` — the APPLICATION layer (use cases + ports)
//!
//! This crate answers **"what commands/queries operate on the world?"**.
//!
//! It is the seam between the UI and the world. It owns:
//!
//! * **Ports** ([`ports`]) — traits describing what the application needs from
//!   the outside world (storage, clock). Written in the application's own
//!   vocabulary, so SQLite specifics can never leak upward.
//! * **Services** ([`services`]) — use cases implemented against those ports.
//!   Pure orchestration: no SQL, no Tauri, no React.
//! * **Errors** ([`error`]) — one error vocabulary for command handlers.
//!
//! Because the ports are traits, every use case in this crate is testable with
//! fakes and a frozen clock — see the tests in `src/services/health_service.rs`.
//! `cargo test -p lr-application` needs no database and no display server.

pub mod error;
pub mod ports;
pub mod services;

pub use error::{AppError, StorageError};
pub use ports::{
    Clock, HealthStore, MigrationRecord, RoundTripProof, SchemaReport, StoreDiagnostics,
};
pub use services::health::{
    ApplicationInfo, DatabaseInfo, HealthReport, HealthService, HealthStatus, RoundTripInfo,
};

/// Human-readable name of this layer, used by status reporting.
pub const LAYER_NAME: &str = "application";
