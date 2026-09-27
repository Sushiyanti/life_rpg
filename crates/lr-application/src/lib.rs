//! # `lr-application` — use cases and storage ports.
//!
//! This layer depends only on `lr-domain`; it contains no SQL, SQLite, Tauri, or
//! React concepts. Persistence adapters implement the traits in [`ports`].

pub mod error;
pub mod ports;
pub mod services;

pub use error::{AppError, StorageError};
pub use ports::{
    Clock, HealthStore, MigrationRecord, RoundTripProof, SchemaReport, StoreDiagnostics, WorldStore,
};
pub use services::health::{
    ApplicationInfo, DatabaseInfo, HealthReport, HealthService, HealthStatus, RoundTripInfo,
};
pub use services::world::{AwardXpOutcome, WorldOverview, WorldService, DEFAULT_LEDGER_LIMIT};

pub const LAYER_NAME: &str = "application";
