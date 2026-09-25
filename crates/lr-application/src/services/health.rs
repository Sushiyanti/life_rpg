//! `HealthService` — the Phase 1 use case.
//!
//! It answers one question: *"is this world actually alive?"* — the app booted,
//! the store is reachable, the schema is where we expect it, and a real
//! write/read round trip succeeded.
//!
//! Design notes:
//!
//! * `run()` **never fails**. A status screen that panics when storage is down
//!   is useless; instead every failure is captured as a structured
//!   [`HealthStatus`] plus a list of human-readable problems, and the UI renders
//!   it. Failures are data here, not control flow.
//! * The probe token is derived from the injected [`Clock`], so the report is
//!   fully deterministic in tests.

use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::StorageError;
use crate::ports::{Clock, HealthStore, MigrationRecord, SchemaReport, StoreDiagnostics};

/// Overall verdict for the status screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Everything checked out.
    Ok,
    /// The app runs, but something is off (schema mismatch, unverifiable write).
    Degraded,
    /// A hard failure: the store is unreachable or the schema is broken.
    Failed,
}

impl HealthStatus {
    /// Lowercase wire label (`"ok"` / `"degraded"` / `"failed"`).
    pub fn as_str(self) -> &'static str {
        match self {
            HealthStatus::Ok => "ok",
            HealthStatus::Degraded => "degraded",
            HealthStatus::Failed => "failed",
        }
    }
}

/// Identity and build facts about the running application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationInfo {
    /// Human-facing product name.
    pub name: String,
    /// Crate/package version.
    pub version: String,
    /// Which development phase this build represents.
    pub phase: String,
    /// Where the code is executing.
    pub runtime: String,
    /// How UI and core talk in this build.
    pub ipc_transport: String,
    /// Whether the app is expected to work with no network.
    pub offline_first: bool,
}

/// Everything known about the persistent store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseInfo {
    /// Backend identifier, e.g. `sqlite`.
    pub backend: String,
    /// File location hint, when file-backed.
    pub location_hint: Option<String>,
    /// Highest applied migration.
    pub schema_version: u32,
    /// Highest migration this build ships.
    pub expected_schema_version: u32,
    /// Whether store and build agree on the schema version.
    pub schema_current: bool,
    /// Every migration with its applied state.
    pub migrations: Vec<MigrationRecord>,
    /// SQLite journal mode in effect.
    pub journal_mode: String,
    /// Whether foreign keys are enforced.
    pub foreign_keys: bool,
}

/// The round-trip evidence, flattened for reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundTripInfo {
    /// Value written.
    pub token: String,
    /// Value read back.
    pub read_back_token: String,
    /// Whether they matched.
    pub matches: bool,
    /// Row id assigned by the store.
    pub row_id: i64,
    /// Probe rows present after the write.
    pub probe_rows: u64,
    /// Timestamp recorded alongside the write.
    pub written_at: String,
}

/// Complete result of a health run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthReport {
    /// Overall verdict.
    pub status: HealthStatus,
    /// One-line summary for the status screen header.
    pub headline: String,
    /// Build/identity facts.
    pub application: ApplicationInfo,
    /// Store facts, or `None` when the store could not be inspected at all.
    pub database: Option<DatabaseInfo>,
    /// Round-trip evidence, or `None` when the probe could not run.
    pub round_trip: Option<RoundTripInfo>,
    /// Human-readable problems; empty when nothing was wrong.
    pub problems: Vec<String>,
    /// When this check ran (RFC 3339).
    pub checked_at: String,
}

/// The health use case.
pub struct HealthService<S, C>
where
    S: HealthStore,
    C: Clock,
{
    store: S,
    clock: C,
    probe_counter: AtomicU64,
}

impl<S, C> HealthService<S, C>
where
    S: HealthStore,
    C: Clock,
{
    /// Build the service from its ports.
    pub fn new(store: S, clock: C) -> Self {
        Self {
            store,
            clock,
            probe_counter: AtomicU64::new(0),
        }
    }

    /// Borrow the store (used by adapters that need raw access later).
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Execute the check. Always returns a report.
    pub fn run(&self) -> HealthReport {
        let checked_at = self.clock.now_rfc3339();
        let mut problems: Vec<String> = Vec::new();

        let application = ApplicationInfo {
            name: "Life RPG".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            phase: "phase-1 foundation".to_string(),
            runtime: "Tauri v2 desktop shell".to_string(),
            ipc_transport: "tauri::command (in-process IPC, no HTTP)".to_string(),
            offline_first: true,
        };

        // --- store diagnostics ------------------------------------------------
        let diagnostics: Option<StoreDiagnostics> = match self.store.diagnostics() {
            Ok(d) => Some(d),
            Err(err) => {
                problems.push(format!("Storage diagnostics unavailable — {err}"));
                None
            }
        };

        // --- schema -----------------------------------------------------------
        let schema: Option<SchemaReport> = match self.store.schema_report() {
            Ok(s) => Some(s),
            Err(err) => {
                problems.push(format!("Schema inspection failed — {err}"));
                None
            }
        };

        if let Some(schema) = &schema {
            if schema.current_version < schema.expected_version {
                let pending: Vec<String> = schema
                    .migrations
                    .iter()
                    .filter(|m| !m.applied)
                    .map(|m| format!("{} ({})", m.version, m.name))
                    .collect();
                problems.push(format!(
                    "Schema is {} migration(s) behind the build; pending: {}",
                    pending.len(),
                    pending.join(", ")
                ));
            } else if schema.current_version > schema.expected_version {
                problems.push(format!(
                    "Store schema v{} is NEWER than this build's v{} — refusing to touch it",
                    schema.current_version, schema.expected_version
                ));
            }
        }

        // --- round trip -------------------------------------------------------
        let token = self.next_probe_token();
        let written_at = self.clock.now_rfc3339();
        let round_trip = match self.store.verify_round_trip(&token, &written_at) {
            Ok(proof) => {
                let matches = proof.matches();
                if !matches {
                    problems.push(format!(
                        "Write/read round trip MISMATCH: wrote `{}`, read `{}`",
                        proof.token, proof.read_back_token
                    ));
                }
                Some(RoundTripInfo {
                    token: proof.token,
                    read_back_token: proof.read_back_token,
                    matches,
                    row_id: proof.row_id,
                    probe_rows: proof.probe_rows,
                    written_at: proof.written_at,
                })
            }
            Err(err) => {
                problems.push(format!("Write/read round trip failed — {err}"));
                None
            }
        };

        // --- verdict ----------------------------------------------------------
        let storage_broken = schema.is_none()
            || matches!(
                self.store.diagnostics(),
                Err(StorageError::Unreachable(_)) | Err(StorageError::Schema(_))
            );
        let schema_current = schema.as_ref().map(SchemaReport::is_current).unwrap_or(false);
        let round_trip_ok = round_trip.as_ref().map(|r| r.matches).unwrap_or(false);

        let status = if storage_broken || schema.is_none() {
            HealthStatus::Failed
        } else if !schema_current || !round_trip_ok {
            HealthStatus::Degraded
        } else {
            HealthStatus::Ok
        };

        let headline = match status {
            HealthStatus::Ok => format!(
                "All systems nominal — SQLite schema {} verified, write/read round trip committed",
                schema
                    .as_ref()
                    .map(|s| format!("v{}", s.current_version))
                    .unwrap_or_else(|| "v?".to_string())
            ),
            HealthStatus::Degraded => {
                "Running, but something needs attention — see the problems below".to_string()
            }
            HealthStatus::Failed => {
                "World storage is unavailable — the application cannot persist anything".to_string()
            }
        };

        let database = schema.map(|schema| DatabaseInfo {
            backend: diagnostics
                .as_ref()
                .map(|d| d.backend.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            location_hint: diagnostics.as_ref().and_then(|d| d.location_hint.clone()),
            schema_version: schema.current_version,
            expected_schema_version: schema.expected_version,
            schema_current: schema.is_current(),
            migrations: schema.migrations,
            journal_mode: diagnostics
                .as_ref()
                .map(|d| d.journal_mode.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            foreign_keys: diagnostics.as_ref().map(|d| d.foreign_keys).unwrap_or(false),
        });

        HealthReport {
            status,
            headline,
            application,
            database,
            round_trip,
            problems,
            checked_at,
        }
    }

    fn next_probe_token(&self) -> String {
        let seq = self.probe_counter.fetch_add(1, Ordering::SeqCst);
        let nanos = self.clock.now_unix_nanos();
        let mut token = String::with_capacity(48);
        let _ = write!(token, "probe-{nanos:x}-{seq:x}");
        token
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::RoundTripProof;
    use std::sync::Mutex;

    struct FakeClock {
        rfc3339: String,
        nanos: u128,
    }

    impl Clock for FakeClock {
        fn now_rfc3339(&self) -> String {
            self.rfc3339.clone()
        }
        fn now_unix_nanos(&self) -> u128 {
            self.nanos
        }
    }

    /// Test double that records what it was asked and returns whatever the test
    /// scripted — including failures.
    struct FakeStore {
        diagnostics: Mutex<Result<StoreDiagnostics, StorageError>>,
        schema: Mutex<Result<SchemaReport, StorageError>>,
        round_trip: Mutex<Result<RoundTripProof, StorageError>>,
        calls: Mutex<Vec<(String, String)>>,
    }

    impl FakeStore {
        fn healthy() -> Self {
            Self {
                diagnostics: Mutex::new(Ok(StoreDiagnostics {
                    backend: "sqlite".into(),
                    location_hint: Some("/tmp/fake.sqlite3".into()),
                    journal_mode: "wal".into(),
                    foreign_keys: true,
                })),
                schema: Mutex::new(Ok(SchemaReport {
                    current_version: 1,
                    expected_version: 1,
                    migrations: vec![MigrationRecord {
                        version: 1,
                        name: "0001_init".into(),
                        applied: true,
                        applied_at: Some("2026-09-25T00:00:00+00:00".into()),
                    }],
                })),
                round_trip: Mutex::new(Ok(RoundTripProof {
                    token: "probe-deadbeef-0".into(),
                    read_back_token: "probe-deadbeef-0".into(),
                    row_id: 1,
                    probe_rows: 1,
                    written_at: "2026-09-25T00:00:00+00:00".into(),
                })),
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl HealthStore for FakeStore {
        fn diagnostics(&self) -> Result<StoreDiagnostics, StorageError> {
            self.diagnostics.lock().unwrap().clone()
        }
        fn schema_report(&self) -> Result<SchemaReport, StorageError> {
            self.schema.lock().unwrap().clone()
        }
        fn verify_round_trip(
            &self,
            token: &str,
            written_at: &str,
        ) -> Result<RoundTripProof, StorageError> {
            self.calls
                .lock()
                .unwrap()
                .push((token.to_string(), written_at.to_string()));

            // Echo the caller's token, exactly as the real SQLite adapter does
            // (it reads the row back). A double that returned a hardcoded proof
            // would make this suite untestable for token determinism.
            match self.round_trip.lock().unwrap().clone() {
                Ok(mut proof) => {
                    // A proof whose stored token and read-back disagree is the
                    // test's scripted corruption — preserve it.
                    let scripted_mismatch = proof.token != proof.read_back_token;
                    proof.token = token.to_string();
                    if !scripted_mismatch {
                        proof.read_back_token = token.to_string();
                    }
                    proof.written_at = written_at.to_string();
                    Ok(proof)
                }
                Err(err) => Err(err),
            }
        }
    }

    fn clock() -> FakeClock {
        FakeClock {
            rfc3339: "2026-09-25T00:00:00+00:00".into(),
            nanos: 0xdead_beef,
        }
    }

    #[test]
    fn healthy_store_reports_ok() {
        let service = HealthService::new(FakeStore::healthy(), clock());
        let report = service.run();

        assert_eq!(report.status, HealthStatus::Ok);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert_eq!(report.checked_at, "2026-09-25T00:00:00+00:00");
        assert_eq!(report.database.as_ref().unwrap().schema_version, 1);
        assert!(report.database.as_ref().unwrap().schema_current);
        assert_eq!(report.round_trip.as_ref().unwrap().probe_rows, 1);
        assert!(report.round_trip.as_ref().unwrap().matches);
        assert!(report.application.offline_first);
    }

    #[test]
    fn probe_token_is_deterministic_for_a_frozen_clock() {
        let service = HealthService::new(FakeStore::healthy(), clock());
        let report = service.run();
        assert_eq!(report.round_trip.as_ref().unwrap().token, "probe-deadbeef-0");

        // second run increments the counter but keeps the frozen clock
        let second = service.run();
        assert_eq!(second.round_trip.as_ref().unwrap().token, "probe-deadbeef-1");
    }

    #[test]
    fn behind_schema_is_degraded_and_lists_pending_migrations() {
        let store = FakeStore::healthy();
        *store.schema.lock().unwrap() = Ok(SchemaReport {
            current_version: 1,
            expected_version: 2,
            migrations: vec![
                MigrationRecord {
                    version: 1,
                    name: "0001_init".into(),
                    applied: true,
                    applied_at: Some("2026-09-25T00:00:00+00:00".into()),
                },
                MigrationRecord {
                    version: 2,
                    name: "0002_future".into(),
                    applied: false,
                    applied_at: None,
                },
            ],
        });

        let report = HealthService::new(store, clock()).run();
        assert_eq!(report.status, HealthStatus::Degraded);
        assert!(report
            .problems
            .iter()
            .any(|p| p.contains("0002_future") && p.contains("behind")));
        assert!(!report.database.as_ref().unwrap().schema_current);
    }

    #[test]
    fn newer_store_schema_is_flagged_not_silently_accepted() {
        let store = FakeStore::healthy();
        *store.schema.lock().unwrap() = Ok(SchemaReport {
            current_version: 99,
            expected_version: 1,
            migrations: vec![],
        });
        let report = HealthService::new(store, clock()).run();
        assert_eq!(report.status, HealthStatus::Degraded);
        assert!(report.problems.iter().any(|p| p.contains("NEWER")));
    }

    #[test]
    fn mismatched_read_back_is_degraded() {
        let store = FakeStore::healthy();
        *store.round_trip.lock().unwrap() = Ok(RoundTripProof {
            token: "probe-a-0".into(),
            read_back_token: "probe-b-0".into(),
            row_id: 7,
            probe_rows: 7,
            written_at: "2026-09-25T00:00:00+00:00".into(),
        });
        let report = HealthService::new(store, clock()).run();
        assert_eq!(report.status, HealthStatus::Degraded);
        assert!(report.problems.iter().any(|p| p.contains("MISMATCH")));
    }

    #[test]
    fn unreachable_store_is_failed_and_report_still_returned() {
        let store = FakeStore::healthy();
        *store.diagnostics.lock().unwrap() = Err(StorageError::Unreachable("disk gone".into()));
        *store.schema.lock().unwrap() = Err(StorageError::Unreachable("disk gone".into()));
        *store.round_trip.lock().unwrap() = Err(StorageError::Unreachable("disk gone".into()));

        let report = HealthService::new(store, clock()).run();
        assert_eq!(report.status, HealthStatus::Failed);
        assert_eq!(report.problems.len(), 3, "{:?}", report.problems);
        assert!(report.database.is_none());
        assert!(report.round_trip.is_none());
    }

    #[test]
    fn service_passes_clock_timestamp_into_the_round_trip() {
        let store = FakeStore::healthy();
        let service = HealthService::new(store, clock());
        service.run();
        let inner = service.store();
        let calls = inner.calls.lock().unwrap();
        assert_eq!(
            calls.first().unwrap().1,
            "2026-09-25T00:00:00+00:00".to_string()
        );
    }
}
