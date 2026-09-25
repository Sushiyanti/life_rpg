//! The SQLite adapter for the application's ports.
//!
//! [`SqliteHealthStore`] is the whole of Phase 1's storage surface. It is
//! deliberately an *enum of two states* rather than an `Option<Connection>`:
//!
//! ```text
//! Ready(connection)   -> normal operation
//! Unavailable(reason) -> the app still runs and reports the failure
//! ```
//!
//! That is what makes the status screen trustworthy: if the data directory is
//! read-only or the disk is gone, the desktop shell must still open a window and
//! *say so* instead of refusing to start. A `Result` at construction time would
//! force the UI to handle a bootstrap failure it cannot even display.
//!
//! Concurrency: a single `Mutex<Connection>` is the right call for this
//! application. There is one user, all access is in-process, and SQLite is
//! fastest when writes are serialized. WAL mode means a future read-only path
//! can be added without changing this design.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use lr_application::{HealthStore, RoundTripProof, SchemaReport, StoreDiagnostics, StorageError};
use rusqlite::Connection;

use crate::error::PersistenceError;
use crate::{migrations, pragma};

/// SQLite-backed implementation of [`HealthStore`].
pub struct SqliteHealthStore {
    state: State,
}

enum State {
    Ready {
        conn: Mutex<Connection>,
        location: Option<PathBuf>,
    },
    Unavailable {
        reason: String,
    },
}

impl SqliteHealthStore {
    /// Open (creating if needed) a database file and bring it up to date.
    ///
    /// `now` is injected so migration timestamps are deterministic in tests.
    /// Failures do **not** propagate: this returns an `Unavailable` store,
    /// because that is exactly the situation the status screen exists to report.
    pub fn open_file(path: impl AsRef<Path>, now: &str) -> Self {
        let path = path.as_ref();
        let location = path.to_path_buf();

        if let Some(parent) = path.parent() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                return Self::unavailable(format!(
                    "could not create data directory `{}`: {err}",
                    parent.display()
                ));
            }
        }

        let conn = match Connection::open(path) {
            Ok(conn) => conn,
            Err(err) => {
                return Self::unavailable(
                    PersistenceError::Open {
                        path: path.display().to_string(),
                        source: err,
                    }
                    .to_string(),
                )
            }
        };

        match Self::finish_open(conn, now) {
            Ok(conn) => Self::ready(conn, Some(location)),
            Err(err) => Self::unavailable(err.to_string()),
        }
    }

    /// Open an in-memory database, fully migrated. Used by tests and as the
    /// last-resort fallback so a first launch can never be a blank window.
    pub fn open_in_memory(now: &str) -> Self {
        let conn = match Connection::open_in_memory() {
            Ok(conn) => conn,
            Err(err) => return Self::unavailable(err.to_string()),
        };

        match Self::finish_open(conn, now) {
            Ok(conn) => Self::ready(conn, None),
            Err(err) => Self::unavailable(err.to_string()),
        }
    }

    /// Build a store that is known to be unusable, carrying the reason.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            state: State::Unavailable {
                reason: reason.into(),
            },
        }
    }

    fn ready(conn: Connection, location: Option<PathBuf>) -> Self {
        Self {
            state: State::Ready {
                conn: Mutex::new(conn),
                location,
            },
        }
    }

    fn finish_open(mut conn: Connection, now: &str) -> Result<Connection, PersistenceError> {
        pragma::configure(&conn)?;
        migrations::run_migrations(&mut conn, now)?;
        Ok(conn)
    }

    /// Borrow the connection, or explain why we cannot.
    fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        match &self.state {
            State::Ready { conn, .. } => {
                // A poisoned mutex means another thread panicked mid-statement.
                // Treat it as an operational failure rather than panicking the
                // UI thread.
                let guard = conn
                    .lock()
                    .map_err(|_| StorageError::Operation("connection mutex poisoned".into()))?;
                f(&guard)
            }
            State::Unavailable { reason } => Err(StorageError::Unreachable(reason.clone())),
        }
    }

    /// Like [`Self::with_conn`], but yields a mutable borrow.
    ///
    /// Needed because `Connection::transaction` takes `&mut self`. Keeping the
    /// two helpers separate means read-only operations cannot accidentally
    /// take a write lock.
    fn with_conn_mut<T>(
        &self,
        f: impl FnOnce(&mut Connection) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        match &self.state {
            State::Ready { conn, .. } => {
                let mut guard = conn
                    .lock()
                    .map_err(|_| StorageError::Operation("connection mutex poisoned".into()))?;
                f(&mut guard)
            }
            State::Unavailable { reason } => Err(StorageError::Unreachable(reason.clone())),
        }
    }

    /// Human-readable location of the store, when file-backed.
    pub fn location(&self) -> Option<String> {
        match &self.state {
            State::Ready { location, .. } => {
                location.as_ref().map(|p| p.display().to_string())
            }
            State::Unavailable { .. } => None,
        }
    }

    /// Whether the store opened successfully.
    pub fn is_available(&self) -> bool {
        matches!(self.state, State::Ready { .. })
    }

    /// Why the store failed to open, if it did.
    pub fn unavailability_reason(&self) -> Option<&str> {
        match &self.state {
            State::Unavailable { reason } => Some(reason.as_str()),
            State::Ready { .. } => None,
        }
    }
}

impl HealthStore for SqliteHealthStore {
    fn diagnostics(&self) -> Result<StoreDiagnostics, StorageError> {
        self.with_conn(|conn| {
            Ok(StoreDiagnostics {
                backend: "sqlite".to_string(),
                location_hint: self.location(),
                journal_mode: pragma::journal_mode(conn)
                    .map_err(StorageError::from)?,
                foreign_keys: pragma::foreign_keys_enabled(conn).map_err(StorageError::from)?,
            })
        })
    }

    fn schema_report(&self) -> Result<SchemaReport, StorageError> {
        self.with_conn(|conn| migrations::schema_report(conn).map_err(StorageError::from))
    }

    fn verify_round_trip(
        &self,
        token: &str,
        written_at: &str,
    ) -> Result<RoundTripProof, StorageError> {
        self.with_conn_mut(|conn| {
            // Everything below happens inside ONE transaction:
            // write -> read back -> count -> commit. If any part fails, nothing
            // is written, so the status screen can never report a half-truth.
            let tx = conn
                .transaction()
                .map_err(|e| StorageError::Operation(e.to_string()))?;

            tx.execute(
                "INSERT INTO health_probe (token, written_at) VALUES (?1, ?2)",
                rusqlite::params![token, written_at],
            )
            .map_err(|e| StorageError::Operation(format!("write failed: {e}")))?;

            let row_id = tx.last_insert_rowid();

            let read_back_token: String = tx
                .query_row(
                    "SELECT token FROM health_probe WHERE id = ?1",
                    rusqlite::params![row_id],
                    |row| row.get(0),
                )
                .map_err(|e| StorageError::Operation(format!("read-back failed: {e}")))?;

            let probe_rows: u64 = tx
                .query_row("SELECT COUNT(*) FROM health_probe", [], |row| row.get(0))
                .map_err(|e| StorageError::Operation(format!("count failed: {e}")))?;

            tx.commit()
                .map_err(|e| StorageError::Operation(format!("commit failed: {e}")))?;

            Ok(RoundTripProof {
                token: token.to_string(),
                read_back_token,
                row_id,
                probe_rows,
                written_at: written_at.to_string(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lr_application::HealthStore;

    const T0: &str = "2026-09-25T00:00:00+00:00";

    #[test]
    fn in_memory_store_is_available_and_migrated() {
        let store = SqliteHealthStore::open_in_memory(T0);
        assert!(store.is_available());

        let schema = store.schema_report().expect("schema");
        assert!(schema.is_current());
        assert_eq!(schema.migrations.len(), 3);
    }

    #[test]
    fn round_trip_writes_reads_and_counts() {
        let store = SqliteHealthStore::open_in_memory(T0);

        let first = store.verify_round_trip("probe-abc", T0).expect("round trip");
        assert!(first.matches());
        assert_eq!(first.token, "probe-abc");
        assert_eq!(first.read_back_token, "probe-abc");
        assert_eq!(first.probe_rows, 1);
        assert_eq!(first.row_id, 1);

        let second = store.verify_round_trip("probe-def", T0).expect("round trip");
        assert_eq!(second.probe_rows, 2, "probe rows accumulate");
        assert_ne!(second.row_id, first.row_id);
    }

    #[test]
    fn data_survives_reopening_the_same_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("world.sqlite3");

        {
            let store = SqliteHealthStore::open_file(&path, T0);
            assert!(store.is_available(), "{:?}", store.unavailability_reason());
            store
                .verify_round_trip("persisted-token", T0)
                .expect("write");
        }

        // Fresh process-equivalent: brand new connection to the same file.
        let reopened = SqliteHealthStore::open_file(&path, T0);
        assert!(reopened.is_available());

        let proof = reopened
            .verify_round_trip("second-run", T0)
            .expect("write after reopen");
        assert_eq!(
            proof.probe_rows, 2,
            "the row written before reopen must still be there"
        );

        let schema = reopened.schema_report().expect("schema");
        assert!(
            schema.is_current(),
            "migrations must not re-run destructively on an existing store"
        );
    }

    #[test]
    fn file_store_uses_wal_and_enforces_foreign_keys() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("world.sqlite3");
        let store = SqliteHealthStore::open_file(&path, T0);

        assert!(store.is_available(), "parent directory must be auto-created");
        let diag = store.diagnostics().expect("diagnostics");
        assert_eq!(diag.backend, "sqlite");
        assert_eq!(diag.journal_mode.to_lowercase(), "wal");
        assert!(diag.foreign_keys);
        assert!(diag.location_hint.as_deref().unwrap().ends_with("world.sqlite3"));
    }

    #[test]
    fn unavailable_store_reports_unreachable_for_every_operation() {
        let store = SqliteHealthStore::unavailable("simulated disk failure");

        let diag = store.diagnostics().expect_err("must fail");
        assert!(matches!(diag, StorageError::Unreachable(_)), "got {diag:?}");

        let schema = store.schema_report().expect_err("must fail");
        assert!(matches!(schema, StorageError::Unreachable(_)));

        let rt = store.verify_round_trip("x", T0).expect_err("must fail");
        assert!(matches!(rt, StorageError::Unreachable(_)));

        assert_eq!(store.unavailability_reason(), Some("simulated disk failure"));
    }

    #[test]
    fn unwritable_path_degrades_instead_of_panicking() {
        // `/proc/self/mem` cannot host a SQLite database; opening must fail
        // gracefully and produce an Unavailable store, never a panic.
        let store = SqliteHealthStore::open_file("/proc/self/mem/world.sqlite3", T0);
        assert!(!store.is_available());
        assert!(store.unavailability_reason().is_some());
    }

    #[test]
    fn end_to_end_through_the_application_service() {
        // The full seam: persistence adapter -> application port -> use case.
        use lr_application::{Clock, HealthService, HealthStatus};

        struct FrozenClock;
        impl Clock for FrozenClock {
            fn now_rfc3339(&self) -> String {
                T0.to_string()
            }
            fn now_unix_nanos(&self) -> u128 {
                42
            }
        }

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("world.sqlite3");
        let service = HealthService::new(SqliteHealthStore::open_file(&path, T0), FrozenClock);
        let report = service.run();

        assert_eq!(report.status, HealthStatus::Ok, "{:?}", report.problems);
        assert!(report.problems.is_empty());
        assert!(report.round_trip.as_ref().unwrap().matches);
        assert!(report.database.as_ref().unwrap().schema_current);
        assert_eq!(
            report.database.as_ref().unwrap().migrations.len(),
            3,
            "status screen shows real migration history"
        );
    }
}
