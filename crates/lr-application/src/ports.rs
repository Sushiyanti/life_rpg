//! Ports — the application's requirements, expressed as traits.
//!
//! These are **driven ports** (the application calls outward). Phase 1 needs
//! only two: somewhere to persist data, and a clock. Later phases add ports for
//! the repositories (`PlayerRepository`, `QuestRepository`, …) following this
//! exact shape: plain request/response DTOs + a `Result<_, StorageError>`.
//!
//! Note what is *absent*: no `Connection`, no `Row`, no SQL, no URLs. The
//! adapter in `lr-persistence` is free to be SQLite today and something else
//! tomorrow without a single line changing here.

use crate::error::StorageError;

/// One row of the migration ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRecord {
    /// Monotonic migration version.
    pub version: u32,
    /// Stable migration name (also the ledger label).
    pub name: String,
    /// Whether the store has this migration applied.
    pub applied: bool,
    /// RFC 3339 instant the migration was applied, if it was.
    pub applied_at: Option<String>,
}

impl MigrationRecord {
    /// `"applied"` / `"pending"` — presentation-friendly state label.
    pub fn state(&self) -> &'static str {
        if self.applied {
            "applied"
        } else {
            "pending"
        }
    }
}

/// Full story of the store's schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaReport {
    /// Highest applied migration version (0 when the store is empty).
    pub current_version: u32,
    /// Highest version this build knows about.
    pub expected_version: u32,
    /// Every known migration, applied or not, in version order.
    pub migrations: Vec<MigrationRecord>,
}

impl SchemaReport {
    /// True when the store is exactly at this build's schema version.
    pub fn is_current(&self) -> bool {
        self.current_version == self.expected_version
    }
}

/// Runtime facts about the store, for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreDiagnostics {
    /// e.g. `"sqlite"`.
    pub backend: String,
    /// Where the data lives, when the store is file-backed. `None` for
    /// in-memory stores.
    pub location_hint: Option<String>,
    /// e.g. `"wal"`.
    pub journal_mode: String,
    /// Whether FK enforcement is on for this connection.
    pub foreign_keys: bool,
}

/// Evidence that a write was durably committed and read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundTripProof {
    /// Token that was written.
    pub token: String,
    /// Token that came back out of the store.
    pub read_back_token: String,
    /// Primary key assigned by the store.
    pub row_id: i64,
    /// Total probe rows in the store after the write.
    pub probe_rows: u64,
    /// RFC 3339 instant recorded with the row.
    pub written_at: String,
}

impl RoundTripProof {
    /// True when the read-back value matches what was written.
    pub fn matches(&self) -> bool {
        self.token == self.read_back_token
    }
}

/// A persistent store the application can check the health of.
///
/// Phase 1 deliberately defines one narrow port for the *status screen*. Real
/// repositories arrive in Phase 2 — this trait is the template they follow.
pub trait HealthStore: Send + Sync {
    /// Report runtime facts about the store (backend, location, pragmas).
    fn diagnostics(&self) -> Result<StoreDiagnostics, StorageError>;

    /// Report the migration state of the store.
    fn schema_report(&self) -> Result<SchemaReport, StorageError>;

    /// Perform a real write followed by a read-back **inside one transaction**,
    /// then commit. This is the proof that persistence actually works rather
    /// than a checkbox.
    ///
    /// `written_at` is supplied by the caller (the [`Clock`]) so this operation
    /// is deterministic under test.
    fn verify_round_trip(&self, token: &str, written_at: &str)
        -> Result<RoundTripProof, StorageError>;
}

/// Source of time.
///
/// Injected everywhere instead of calling `chrono::Utc::now()` inline, so tests
/// can freeze time and get byte-identical reports.
pub trait Clock: Send + Sync {
    /// Current instant as RFC 3339 / UTC.
    fn now_rfc3339(&self) -> String;

    /// Current instant as nanoseconds since the Unix epoch.
    fn now_unix_nanos(&self) -> u128;
}
