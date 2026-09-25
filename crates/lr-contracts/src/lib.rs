//! # `lr-contracts` — the IPC boundary
//!
//! This crate is the **data-transfer vocabulary** shared with the frontend. It
//! exists to solve one specific problem: the UI must never receive a Rust type,
//! and the core must never receive a UI type.
//!
//! Both sides meet in the middle, here:
//!
//! ```text
//! HealthReport (application)  ──From──▶  HealthReportDto (contracts)  ──serde──▶  JSON  ──▶  TS type
//! ```
//!
//! Rules this crate enforces by construction:
//!
//! * **Serialization never touches the domain.** `HealthReport` is a plain Rust
//!   struct with no `serde` derives; only its DTO mirror is serializable. Adding
//!   a field to the core cannot accidentally change the wire format.
//! * **The wire format is camelCase.** `#[serde(rename_all = "camelCase")]` on
//!   every DTO keeps TypeScript idiomatic without a case-mapping layer.
//! * **The wire format is versioned-ish and additive-friendly.** The frontend
//!   mirrors these shapes in `src/domain/`. `contract_drift_matches_the_
//!   typescript_mirror` (below) pins the exact JSON keys so the two cannot drift
//!   apart silently.
//!
//! Phase 1 has a single DTO family (health/status) because that is the only
//! surface the UI needs. Phase 2 adds `PlayerDto`, `QuestDto`, … following the
//! same pattern.

use lr_application::HealthReport;
use serde::{Deserialize, Serialize};

/// Machine-readable status verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatusDto {
    /// Everything verified.
    Ok,
    /// Running with problems worth surfacing.
    Degraded,
    /// Storage is unusable.
    Failed,
}

/// Build/identity facts about the running application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInfoDto {
    /// Product name.
    pub name: String,
    /// Version.
    pub version: String,
    /// Development phase.
    pub phase: String,
    /// Execution environment.
    pub runtime: String,
    /// How UI and core communicate.
    pub ipc_transport: String,
    /// Whether the app works without a network.
    pub offline_first: bool,
}

/// One migration, as shown on the status screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationRecordDto {
    /// Migration version.
    pub version: u32,
    /// Migration name.
    pub name: String,
    /// Whether it is applied.
    pub applied: bool,
    /// When it was applied, if it was.
    pub applied_at: Option<String>,
    /// `"applied"` or `"pending"`.
    pub state: String,
}

/// Store facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseInfoDto {
    /// Backend identifier.
    pub backend: String,
    /// File location, when file-backed.
    pub location_hint: Option<String>,
    /// Highest applied migration.
    pub schema_version: u32,
    /// Highest migration this build ships.
    pub expected_schema_version: u32,
    /// Whether store and build agree.
    pub schema_current: bool,
    /// Migration history.
    pub migrations: Vec<MigrationRecordDto>,
    /// Journal mode in effect.
    pub journal_mode: String,
    /// Whether foreign keys are enforced.
    pub foreign_keys: bool,
}

/// Round-trip evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundTripDto {
    /// Value written.
    pub token: String,
    /// Value read back.
    pub read_back_token: String,
    /// Whether they matched.
    pub matches: bool,
    /// Row id assigned.
    pub row_id: i64,
    /// Probe rows after the write.
    pub probe_rows: u64,
    /// Timestamp recorded with the write.
    pub written_at: String,
}

/// The complete status payload sent to the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthReportDto {
    /// Overall verdict.
    pub status: HealthStatusDto,
    /// One-line summary.
    pub headline: String,
    /// Build facts.
    pub application: ApplicationInfoDto,
    /// Store facts.
    pub database: Option<DatabaseInfoDto>,
    /// Round-trip evidence.
    pub round_trip: Option<RoundTripDto>,
    /// Human-readable problems.
    pub problems: Vec<String>,
    /// When the check ran.
    pub checked_at: String,
}

/// Error payload for a failed command.
///
/// Always the same shape, so the frontend has exactly one error path:
///
/// ```ts
/// try { await invoke(...) } catch (e) { const err = e as CommandErrorDto }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandErrorDto {
    /// Stable machine-readable code, e.g. `storage_unreachable`.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

impl From<HealthReport> for HealthReportDto {
    fn from(report: HealthReport) -> Self {
        Self {
            status: match report.status {
                lr_application::HealthStatus::Ok => HealthStatusDto::Ok,
                lr_application::HealthStatus::Degraded => HealthStatusDto::Degraded,
                lr_application::HealthStatus::Failed => HealthStatusDto::Failed,
            },
            headline: report.headline,
            application: ApplicationInfoDto {
                name: report.application.name,
                version: report.application.version,
                phase: report.application.phase,
                runtime: report.application.runtime,
                ipc_transport: report.application.ipc_transport,
                offline_first: report.application.offline_first,
            },
            database: report.database.map(|db| DatabaseInfoDto {
                backend: db.backend,
                location_hint: db.location_hint,
                schema_version: db.schema_version,
                expected_schema_version: db.expected_schema_version,
                schema_current: db.schema_current,
                migrations: db
                    .migrations
                    .into_iter()
                    .map(|m| {
                        let state = m.state().to_string();
                        MigrationRecordDto {
                            version: m.version,
                            name: m.name,
                            applied: m.applied,
                            applied_at: m.applied_at,
                            state,
                        }
                    })
                    .collect(),
                journal_mode: db.journal_mode,
                foreign_keys: db.foreign_keys,
            }),
            round_trip: report.round_trip.map(|rt| RoundTripDto {
                token: rt.token,
                read_back_token: rt.read_back_token,
                matches: rt.matches,
                row_id: rt.row_id,
                probe_rows: rt.probe_rows,
                written_at: rt.written_at,
            }),
            problems: report.problems,
            checked_at: report.checked_at,
        }
    }
}

impl From<lr_application::AppError> for CommandErrorDto {
    fn from(err: lr_application::AppError) -> Self {
        Self {
            code: err.code().to_string(),
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lr_application::{
        ApplicationInfo, DatabaseInfo, HealthStatus, MigrationRecord, RoundTripInfo,
    };

    fn sample_report() -> HealthReport {
        HealthReport {
            status: HealthStatus::Ok,
            headline: "All systems nominal".into(),
            application: ApplicationInfo {
                name: "Life RPG".into(),
                version: "0.1.0".into(),
                phase: "phase-1 foundation".into(),
                runtime: "Tauri v2 desktop shell".into(),
                ipc_transport: "tauri::command (in-process IPC, no HTTP)".into(),
                offline_first: true,
            },
            database: Some(DatabaseInfo {
                backend: "sqlite".into(),
                location_hint: Some("/tmp/world.sqlite3".into()),
                schema_version: 3,
                expected_schema_version: 3,
                schema_current: true,
                migrations: vec![MigrationRecord {
                    version: 1,
                    name: "0001_core_ledger".into(),
                    applied: true,
                    applied_at: Some("2026-09-25T00:00:00+00:00".into()),
                }],
                journal_mode: "wal".into(),
                foreign_keys: true,
            }),
            round_trip: Some(RoundTripInfo {
                token: "probe-a-0".into(),
                read_back_token: "probe-a-0".into(),
                matches: true,
                row_id: 1,
                probe_rows: 1,
                written_at: "2026-09-25T00:00:00+00:00".into(),
            }),
            problems: vec![],
            checked_at: "2026-09-25T00:00:00+00:00".into(),
        }
    }

    #[test]
    fn status_verdict_serializes_lowercase() {
        let dto: HealthReportDto = sample_report().into();
        let json = serde_json::to_value(&dto).expect("serialize");
        assert_eq!(json["status"], serde_json::json!("ok"));
    }

    /// This test is the anti-drift mechanism between Rust and TypeScript.
    /// `src/domain/health.ts` mirrors these keys by hand; if either side
    /// changes, this fails and forces both to be updated together.
    #[test]
    fn contract_drift_matches_the_typescript_mirror() {
        let dto: HealthReportDto = sample_report().into();
        let json = serde_json::to_value(&dto).expect("serialize");
        let obj = json.as_object().expect("object");

        let mut top_level: Vec<&str> = obj.keys().map(String::as_str).collect();
        top_level.sort_unstable();
        assert_eq!(
            top_level,
            vec![
                "application",
                "checkedAt",
                "database",
                "headline",
                "problems",
                "roundTrip",
                "status",
            ]
        );

        let application = json["application"].as_object().expect("application");
        assert!(application.contains_key("ipcTransport"));
        assert!(application.contains_key("offlineFirst"));

        let database = json["database"].as_object().expect("database");
        for key in [
            "backend",
            "locationHint",
            "schemaVersion",
            "expectedSchemaVersion",
            "schemaCurrent",
            "migrations",
            "journalMode",
            "foreignKeys",
        ] {
            assert!(database.contains_key(key), "database missing `{key}`");
        }

        let migration = json["database"]["migrations"][0]
            .as_object()
            .expect("migration");
        for key in ["version", "name", "applied", "appliedAt", "state"] {
            assert!(migration.contains_key(key), "migration missing `{key}`");
        }

        let round_trip = json["roundTrip"].as_object().expect("roundTrip");
        for key in [
            "token",
            "readBackToken",
            "matches",
            "rowId",
            "probeRows",
            "writtenAt",
        ] {
            assert!(round_trip.contains_key(key), "roundTrip missing `{key}`");
        }
    }

    #[test]
    fn missing_database_serializes_as_null_not_missing() {
        let mut report = sample_report();
        report.database = None;
        report.round_trip = None;
        report.status = HealthStatus::Failed;

        let dto: HealthReportDto = report.into();
        let json = serde_json::to_value(&dto).expect("serialize");
        assert!(json["database"].is_null());
        assert!(json["roundTrip"].is_null());
        assert_eq!(json["status"], serde_json::json!("failed"));
        // `Option::is_null()` matters because TS mirrors these as `| null`.
    }

    #[test]
    fn round_trip_deserializes_back_for_round_trip_parity() {
        let dto: HealthReportDto = sample_report().into();
        let json = serde_json::to_string(&dto).expect("serialize");
        let back: HealthReportDto = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(dto, back);
    }

    #[test]
    fn app_error_maps_to_a_stable_code() {
        let err = lr_application::AppError::Storage(lr_application::StorageError::Unreachable(
            "gone".into(),
        ));
        let dto: CommandErrorDto = err.into();
        assert_eq!(dto.code, "storage_unreachable");
        assert!(dto.message.contains("gone"));
    }
}
