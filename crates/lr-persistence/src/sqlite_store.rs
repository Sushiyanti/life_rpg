//! The SQLite adapter for the application's ports.
//!
//! [`SqliteHealthStore`] owns the file-backed world's SQLite connection and is
//! deliberately an *enum of states* rather than an `Option<Connection>`:
//!
//! ```text
//! Ready(connection)               -> normal operation
//! Unavailable(reason, location)   -> fail closed, report, and allow guarded restore
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
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use lr_application::{HealthStore, RoundTripProof, SchemaReport, StorageError, StoreDiagnostics};
use rusqlite::Connection;

use crate::error::PersistenceError;
use crate::{backup, migrations, pragma};

/// SQLite-backed implementation of [`HealthStore`].
pub struct SqliteHealthStore {
    state: State,
    restore_requires_recovery: AtomicBool,
}

enum State {
    Ready {
        conn: Mutex<Connection>,
        location: Option<PathBuf>,
    },
    Unavailable {
        reason: String,
        location: Option<PathBuf>,
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
        let existed = path.exists();

        if let Some(parent) = path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                return Self::unavailable_at("World storage could not be prepared. Check storage permissions and available disk space. (LR-STORAGE-01)", Some(location.clone()));
            }
        }

        if existed {
            match std::fs::symlink_metadata(path) {
                Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                    return Self::unavailable_at("The existing world file is not a regular database file. It was left unchanged. (LR-STORAGE-02)", Some(location.clone()));
                }
                Ok(metadata) if metadata.len() == 0 => {
                    return Self::unavailable_at("The existing world file is empty or incomplete. It was left unchanged; restore a verified backup to continue. (LR-STORAGE-03)", Some(location.clone()));
                }
                Err(_) => return Self::unavailable_at("The existing world file could not be inspected. It was left unchanged. (LR-STORAGE-01)", Some(location.clone())),
                _ => {}
            }
        }

        let conn = match Connection::open(path) {
            Ok(conn) => conn,
            Err(err) => {
                return Self::unavailable_at(
                    startup_failure(&PersistenceError::Open {
                        path: String::new(),
                        source: err,
                    }),
                    Some(location.clone()),
                )
            }
        };

        match Self::finish_open(conn, now, existed, Some(path)) {
            Ok(conn) => Self::ready(conn, Some(location)),
            Err(err) => Self::unavailable_at(startup_failure(&err), Some(location)),
        }
    }

    /// Open an in-memory database, fully migrated. Used by tests and as the
    /// last-resort fallback so a first launch can never be a blank window.
    pub fn open_in_memory(now: &str) -> Self {
        let conn = match Connection::open_in_memory() {
            Ok(conn) => conn,
            Err(err) => return Self::unavailable(err.to_string()),
        };

        match Self::finish_open(conn, now, false, None) {
            Ok(conn) => Self::ready(conn, None),
            Err(err) => Self::unavailable(startup_failure(&err)),
        }
    }

    /// Build a store that is known to be unusable, carrying the reason.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::unavailable_at(reason, None)
    }

    fn unavailable_at(reason: impl Into<String>, location: Option<PathBuf>) -> Self {
        Self {
            state: State::Unavailable {
                reason: reason.into(),
                location,
            },
            restore_requires_recovery: AtomicBool::new(false),
        }
    }

    fn ready(conn: Connection, location: Option<PathBuf>) -> Self {
        Self {
            state: State::Ready {
                conn: Mutex::new(conn),
                location,
            },
            restore_requires_recovery: AtomicBool::new(false),
        }
    }

    fn finish_open(
        mut conn: Connection,
        now: &str,
        existed: bool,
        path: Option<&Path>,
    ) -> Result<Connection, PersistenceError> {
        pragma::configure_connection(&conn)?;
        if existed {
            migrations::validate_ledger(&conn)?;
            let current = migrations::applied_version(&conn)?;
            let expected = migrations::expected_version();
            if current > expected {
                return Err(PersistenceError::SchemaTooNew {
                    found: current,
                    expected,
                });
            }
            if current > 0 {
                let integrity =
                    backup::check_integrity(&conn).map_err(|_| PersistenceError::Integrity)?;
                if !integrity.healthy {
                    return Err(PersistenceError::Integrity);
                }
            }
            if current == 0 {
                let user_tables: u32 = conn.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name<>'schema_migrations'",
                    [], |row| row.get(0),
                )?;
                if user_tables != 0 {
                    return Err(PersistenceError::InvalidMigrationLedger);
                }
            } else if current < expected {
                let path = path.ok_or(PersistenceError::SafetyBackup)?;
                let safety_dir = path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join("backups");
                backup::create_migration_safety_backup(&conn, &safety_dir, current, now)
                    .map_err(|_| PersistenceError::SafetyBackup)?;
            }
        }
        // Change persistent journal mode only after the existing file has been
        // checked and its pre-migration recovery point has been captured.
        pragma::configure(&conn)?;
        migrations::run_migrations(&mut conn, now)?;
        Ok(conn)
    }

    /// Create and verify a portable backup from the currently open database.
    pub fn create_backup(
        &self,
        path: &Path,
        created_at: &str,
    ) -> Result<backup::BackupInfo, StorageError> {
        if self.location().is_none() {
            return Err(StorageError::Operation(
                "the current world is not file-backed".into(),
            ));
        }
        self.with_conn(|connection| {
            backup::create_backup(connection, path, created_at)
                .map_err(|error| StorageError::Operation(error.to_string()))
        })
    }

    /// Validate an external backup without modifying it.
    pub fn inspect_backup(path: &Path) -> Result<backup::BackupInfo, backup::BackupError> {
        backup::inspect_backup(path)
    }

    /// Run the bounded, read-only world integrity check.
    pub fn check_integrity(&self) -> Result<backup::IntegrityReport, StorageError> {
        self.with_conn(|connection| {
            backup::check_integrity(connection)
                .map_err(|error| StorageError::Operation(error.to_string()))
        })
    }

    /// Stage, validate, and restore a world, creating a verified pre-restore
    /// safety backup before replacement. A file-backed store whose startup
    /// migration failed may use this recovery-only path; other operations stay
    /// unavailable until the user reloads the application.
    pub fn restore_backup(
        &self,
        backup_path: &Path,
        safety_directory: &Path,
        expected_sha256: &str,
        confirm_different_world: bool,
        now: &str,
    ) -> Result<backup::RestoreInfo, StorageError> {
        if self.restore_requires_recovery.load(Ordering::Acquire) {
            return Err(StorageError::Unreachable("Restore and rollback could not be verified. Stop editing and use the retained safety backup. (LR-RESTORE-01)".into()));
        }
        let result = match &self.state {
            State::Unavailable {
                location: Some(path),
                ..
            } => {
                let metadata = std::fs::symlink_metadata(path).map_err(|_| {
                    StorageError::Operation(
                        "the existing world file could not be inspected; no restore was attempted"
                            .into(),
                    )
                })?;
                if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() == 0 {
                    return Err(StorageError::Operation("the existing world file is not a recoverable database file; no restore was attempted".into()));
                }
                let mut connection = Connection::open(path).map_err(|_| {
                    StorageError::Operation(
                        "the existing world file could not be opened; no restore was attempted"
                            .into(),
                    )
                })?;
                pragma::configure_connection(&connection)
                    .map_err(|_| StorageError::Operation("the existing world could not be prepared for recovery; no restore was attempted".into()))?;
                self.restore_into(
                    &mut connection,
                    backup_path,
                    safety_directory,
                    expected_sha256,
                    confirm_different_world,
                    now,
                )
            }
            State::Ready { .. } => self.with_conn_mut(|connection| {
                self.restore_into(
                    connection,
                    backup_path,
                    safety_directory,
                    expected_sha256,
                    confirm_different_world,
                    now,
                )
            }),
            State::Unavailable { location: None, .. } => {
                return Err(StorageError::Operation(
                    "the current world is not file-backed".into(),
                ));
            }
        };
        result
    }

    fn restore_into(
        &self,
        connection: &mut Connection,
        backup_path: &Path,
        safety_directory: &Path,
        expected_sha256: &str,
        confirm_different_world: bool,
        now: &str,
    ) -> Result<backup::RestoreInfo, StorageError> {
        let result = backup::restore_backup(
            connection,
            backup_path,
            safety_directory,
            expected_sha256,
            confirm_different_world,
            now,
        );
        if matches!(&result, Err(backup::BackupError::RollbackFailed)) {
            self.restore_requires_recovery
                .store(true, Ordering::Release);
        }
        result.map_err(|error| StorageError::Operation(error.to_string()))
    }

    /// Borrow the connection, or explain why we cannot.
    pub(crate) fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        if self.restore_requires_recovery.load(Ordering::Acquire) {
            return Err(StorageError::Unreachable("Restore and rollback could not be verified. Stop editing and use the retained safety backup. (LR-RESTORE-01)".into()));
        }
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
            State::Unavailable { reason, .. } => Err(StorageError::Unreachable(reason.clone())),
        }
    }

    /// Like [`Self::with_conn`], but yields a mutable borrow.
    ///
    /// Needed because `Connection::transaction` takes `&mut self`. Keeping the
    /// two helpers separate means read-only operations cannot accidentally
    /// take a write lock.
    pub(crate) fn with_conn_mut<T>(
        &self,
        f: impl FnOnce(&mut Connection) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        if self.restore_requires_recovery.load(Ordering::Acquire) {
            return Err(StorageError::Unreachable("Restore and rollback could not be verified. Stop editing and use the retained safety backup. (LR-RESTORE-01)".into()));
        }
        match &self.state {
            State::Ready { conn, .. } => {
                let mut guard = conn
                    .lock()
                    .map_err(|_| StorageError::Operation("connection mutex poisoned".into()))?;
                f(&mut guard)
            }
            State::Unavailable { reason, .. } => Err(StorageError::Unreachable(reason.clone())),
        }
    }

    /// Human-readable location of the store, when file-backed.
    pub fn location(&self) -> Option<String> {
        match &self.state {
            State::Ready { location, .. } => location.as_ref().map(|p| p.display().to_string()),
            State::Unavailable { location, .. } => {
                location.as_ref().map(|p| p.display().to_string())
            }
        }
    }

    /// Whether the store opened successfully.
    pub fn is_available(&self) -> bool {
        !self.restore_requires_recovery.load(Ordering::Acquire)
            && matches!(self.state, State::Ready { .. })
    }

    /// Why the store failed to open, if it did.
    pub fn unavailability_reason(&self) -> Option<&str> {
        if self.restore_requires_recovery.load(Ordering::Acquire) {
            return Some("Restore and rollback could not be verified. Stop editing and use the retained safety backup. (LR-RESTORE-01)");
        }
        match &self.state {
            State::Unavailable { reason, .. } => Some(reason.as_str()),
            State::Ready { .. } => None,
        }
    }
}

fn startup_failure(error: &PersistenceError) -> String {
    match error {
        PersistenceError::SchemaTooNew { .. } => "This world was created by a newer Life RPG version. Update the application to open it. No downgrade was attempted. (LR-VERSION-01)".into(),
        PersistenceError::SafetyBackup => "A verified safety backup could not be created, so the database upgrade was not attempted. Check available storage and try again. (LR-BACKUP-01)".into(),
        PersistenceError::Migration { .. } => "The database upgrade did not complete. A pre-upgrade recovery backup was preserved. Restart the app or restore that backup. (LR-MIGRATION-01)".into(),
        PersistenceError::InvalidMigrationLedger => "The database migration history is incomplete or inconsistent. The existing world was not reset. Restore a verified backup or seek help. (LR-INTEGRITY-01)".into(),
        PersistenceError::Integrity => "The existing world did not pass a read-only integrity check. The file was left unchanged. Inspect a verified backup or seek help before continuing. (LR-INTEGRITY-02)".into(),
        _ => "Life RPG could not open the local world database. The existing file was left in place. Check storage permissions and available disk space. (LR-STORAGE-01)".into(),
    }
}

impl HealthStore for SqliteHealthStore {
    fn diagnostics(&self) -> Result<StoreDiagnostics, StorageError> {
        self.with_conn(|conn| {
            Ok(StoreDiagnostics {
                backend: "sqlite".to_string(),
                location_hint: self
                    .location()
                    .map(|_| "Life RPG application data folder".to_string()),
                journal_mode: pragma::journal_mode(conn).map_err(StorageError::from)?,
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

    fn build_schema_file(path: &Path, version: u32) -> Connection {
        let mut connection = Connection::open(path).expect("open fixture");
        pragma::configure(&connection).expect("configure fixture");
        migrations::ensure_ledger(&connection).expect("ledger");
        for migration in migrations::MIGRATIONS.iter().take(version as usize) {
            let tx = connection.transaction().expect("migration transaction");
            tx.execute_batch(migration.sql).expect("migration SQL");
            tx.execute(
                "INSERT INTO schema_migrations(version,name,applied_at) VALUES(?1,?2,?3)",
                rusqlite::params![migration.version, migration.name, T0],
            )
            .expect("ledger insert");
            tx.commit().expect("migration commit");
        }
        connection
    }

    #[test]
    fn in_memory_store_is_available_and_migrated() {
        let store = SqliteHealthStore::open_in_memory(T0);
        assert!(store.is_available());

        let schema = store.schema_report().expect("schema");
        assert!(schema.is_current());
        assert_eq!(schema.migrations.len(), 17);
    }

    #[test]
    fn round_trip_writes_reads_and_counts() {
        let store = SqliteHealthStore::open_in_memory(T0);

        let first = store
            .verify_round_trip("probe-abc", T0)
            .expect("round trip");
        assert!(first.matches());
        assert_eq!(first.token, "probe-abc");
        assert_eq!(first.read_back_token, "probe-abc");
        assert_eq!(first.probe_rows, 1);
        assert_eq!(first.row_id, 1);

        let second = store
            .verify_round_trip("probe-def", T0)
            .expect("round trip");
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
    fn existing_v16_world_gets_verified_checkpoint_before_v17_upgrade() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("world.sqlite3");
        {
            let connection = build_schema_file(&path, 16);
            connection.execute(
                "INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES('preserved-player','Ada',4,321,?1,?1)",
                [T0],
            ).expect("seed old world");
        }

        let store = SqliteHealthStore::open_file(&path, T0);
        assert!(store.is_available(), "{:?}", store.unavailability_reason());
        assert_eq!(store.schema_report().unwrap().migrations.len(), 17);
        let name: String = store
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT name FROM players WHERE id='preserved-player'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| StorageError::Operation(error.to_string()))
            })
            .unwrap();
        assert_eq!(name, "Ada");

        let checkpoint_dir = dir.path().join("backups");
        let checkpoints: Vec<_> = std::fs::read_dir(&checkpoint_dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(
            checkpoints.len(),
            1,
            "one checkpoint is retained for the upgrade"
        );
        let info = SqliteHealthStore::inspect_backup(&checkpoints[0]).unwrap();
        assert_eq!(
            info.schema_version, 16,
            "checkpoint is from before the migration"
        );
        assert_eq!(
            info.players[0],
            backup::PlayerDescriptor {
                id: "preserved-player".into(),
                name: "Ada".into()
            }
        );
        assert!(info.integrity.healthy);
    }

    #[test]
    fn release_candidate_migration_matrix_upgrades_schema_1_and_13_through_16() {
        for version in [1, 13, 14, 15, 16] {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join(format!("schema-{version}.sqlite3"));
            {
                let connection = build_schema_file(&path, version);
                if version >= 13 {
                    connection
                        .execute(
                            "INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES(?1,'Preserved Ada',7,321,?2,?2)",
                            rusqlite::params![format!("preserved-v{version}"), T0],
                        )
                        .expect("seed historical Player");
                }
            }
            let store = SqliteHealthStore::open_file(&path, T0);
            assert!(
                store.is_available(),
                "schema {version}: {:?}",
                store.unavailability_reason()
            );
            let schema = store.schema_report().unwrap();
            assert_eq!(schema.current_version, 17, "schema {version}");
            assert!(store.check_integrity().unwrap().healthy);
            if version >= 13 {
                let name = store
                    .with_conn(|conn| {
                        conn.query_row(
                            "SELECT name FROM players WHERE id=?1",
                            [format!("preserved-v{version}")],
                            |row| row.get::<_, String>(0),
                        )
                        .map_err(|error| StorageError::Operation(error.to_string()))
                    })
                    .unwrap();
                assert_eq!(name, "Preserved Ada", "schema {version} data retained");
                let checkpoints: Vec<_> = std::fs::read_dir(dir.path().join("backups"))
                    .unwrap()
                    .map(|entry| entry.unwrap().path())
                    .collect();
                assert_eq!(checkpoints.len(), 1, "schema {version} safety checkpoint");
                let info = SqliteHealthStore::inspect_backup(&checkpoints[0]).unwrap();
                assert_eq!(info.schema_version, version);
                assert!(info.integrity.healthy);
            }
        }

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("schema-17.sqlite3");
        drop(build_schema_file(&path, 17));
        let store = SqliteHealthStore::open_file(&path, T0);
        assert!(store.is_available());
        assert!(store.schema_report().unwrap().is_current());
        assert!(!dir.path().join("backups").exists());
    }

    #[cfg(unix)]
    #[test]
    fn abrupt_child_termination_rolls_back_an_uncommitted_world_transaction() {
        use std::{
            process::Command,
            thread,
            time::{Duration, Instant},
        };

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("crash-world.sqlite3");
        {
            let connection = build_schema_file(&path, 17);
            connection
                .execute(
                    "INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES('crash-player','Crash Test',1,321,?1,?1)",
                    [T0],
                )
                .unwrap();
        }
        let marker = dir.path().join("transaction-open.marker");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("sqlite_store::tests::phase13_crash_child_holds_uncommitted_world_transition")
            .arg("--ignored")
            .arg("--nocapture")
            .env("LR_PHASE13_CRASH_DB", &path)
            .env("LR_PHASE13_CRASH_MARKER", &marker)
            .spawn()
            .expect("spawn isolated crash child");
        let deadline = Instant::now() + Duration::from_secs(15);
        while !marker.exists() && Instant::now() < deadline {
            if let Some(status) = child.try_wait().unwrap() {
                panic!("crash child exited before transaction marker: {status}");
            }
            thread::sleep(Duration::from_millis(20));
        }
        assert!(marker.exists(), "child reached its open-transaction marker");
        child.kill().expect("kill child during open transaction");
        let status = child.wait().unwrap();
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            status.signal(),
            Some(9),
            "child was killed, not gracefully stopped"
        );

        let reopened = SqliteHealthStore::open_file(&path, T0);
        assert!(
            reopened.is_available(),
            "{:?}",
            reopened.unavailability_reason()
        );
        let (xp, transaction_count) = reopened
            .with_conn(|conn| {
                let xp = conn
                    .query_row(
                        "SELECT current_xp FROM players WHERE id='crash-player'",
                        [],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(|error| StorageError::Operation(error.to_string()))?;
                let count = conn
                    .query_row(
                        "SELECT COUNT(*) FROM transactions WHERE player_id='crash-player'",
                        [],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(|error| StorageError::Operation(error.to_string()))?;
                Ok((xp, count))
            })
            .unwrap();
        assert_eq!(xp, 321, "uncommitted Player state rolled back");
        assert_eq!(transaction_count, 0, "no orphaned transaction survived");
        let integrity = reopened.check_integrity().unwrap();
        assert!(integrity.integrity_check_ok);
        assert_eq!(integrity.foreign_key_violations, 0);
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "spawned only by the deterministic parent SIGKILL test"]
    fn phase13_crash_child_holds_uncommitted_world_transition() {
        let Ok(path) = std::env::var("LR_PHASE13_CRASH_DB") else {
            return;
        };
        let marker = std::env::var("LR_PHASE13_CRASH_MARKER").unwrap();
        let mut connection = Connection::open(path).unwrap();
        pragma::configure_connection(&connection).unwrap();
        let tx = connection.transaction().unwrap();
        tx.execute(
            "UPDATE players SET current_xp=999 WHERE id='crash-player'",
            [],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO transactions(player_id,transaction_type_code,resource,amount,applied_amount,occurred_at,captured_at,reason) VALUES('crash-player','xp','xp',678,678,?1,?1,'phase13_crash_fixture')",
            [T0],
        )
        .unwrap();
        std::fs::write(marker, b"transaction remains uncommitted\n").unwrap();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }

    #[test]
    fn invalid_existing_world_is_not_migrated_when_integrity_check_fails() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("world.sqlite3");
        {
            let connection = build_schema_file(&path, 16);
            connection
                .execute_batch("PRAGMA ignore_check_constraints=ON")
                .unwrap();
            connection.execute(
                "INSERT INTO players(id,name,level,current_xp,metadata_json,created_at,updated_at) VALUES('invalid-player','Invalid',1,0,'not-json',?1,?1)",
                [T0],
            ).unwrap();
        }

        let store = SqliteHealthStore::open_file(&path, T0);
        assert!(!store.is_available());
        assert!(store
            .unavailability_reason()
            .unwrap()
            .contains("LR-INTEGRITY-02"));
        let backup_dir = dir.path().join("backups");
        if let Ok(mut entries) = std::fs::read_dir(&backup_dir) {
            assert!(
                entries.next().is_none(),
                "an unverified checkpoint must not be presented as recoverable"
            );
        }

        let preserved = Connection::open(&path).unwrap();
        assert_eq!(migrations::applied_version(&preserved).unwrap(), 16);
        let metadata: String = preserved
            .query_row(
                "SELECT metadata_json FROM players WHERE id='invalid-player'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            metadata, "not-json",
            "migration failure must leave the original record untouched"
        );
    }

    #[test]
    fn startup_rejects_a_missing_schema17_table_without_recreating_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("world.sqlite3");
        let connection = build_schema_file(&path, 17);
        connection
            .execute("DROP TABLE workspace_panels", [])
            .unwrap();
        drop(connection);

        let store = SqliteHealthStore::open_file(&path, T0);
        assert!(!store.is_available());
        assert!(store
            .unavailability_reason()
            .unwrap()
            .contains("LR-INTEGRITY-02"));
        let unchanged = Connection::open(&path).unwrap();
        let exists: bool = unchanged
            .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='workspace_panels')", [], |row| row.get(0))
            .unwrap();
        assert!(!exists);
    }

    #[test]
    fn user_can_restore_a_verified_backup_after_startup_migration_failure() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("world.sqlite3");
        {
            let old = build_schema_file(&path, 16);
            old.execute(
                "INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES('old-player','Old world',1,0,?1,?1)",
                [T0],
            ).unwrap();
            old.execute_batch("CREATE TRIGGER test_abort_v17 BEFORE INSERT ON schema_migrations WHEN NEW.version=17 BEGIN SELECT RAISE(ABORT,'injected startup migration failure'); END;").unwrap();
        }

        let unavailable = SqliteHealthStore::open_file(&path, T0);
        assert!(!unavailable.is_available());
        assert!(unavailable
            .unavailability_reason()
            .unwrap()
            .contains("LR-MIGRATION-01"));
        assert_eq!(
            unavailable.location().as_deref(),
            Some(path.to_str().unwrap())
        );

        let mut replacement = Connection::open_in_memory().unwrap();
        pragma::configure(&replacement).unwrap();
        migrations::run_migrations(&mut replacement, T0).unwrap();
        replacement.execute(
            "INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES('new-player','Restored world',2,45,?1,?1)",
            [T0],
        ).unwrap();
        let backup_path = dir.path().join("verified-restore.liferpg-backup");
        let info = backup::create_backup(&replacement, &backup_path, T0).unwrap();

        let safety_directory = dir.path().join("backups");
        let restore = unavailable
            .restore_backup(&backup_path, &safety_directory, &info.sha256, true, T0)
            .unwrap();
        assert_eq!(restore.restored_schema_version, 17);
        assert!(
            !unavailable.is_available(),
            "reload is required before ordinary editing resumes"
        );
        let checkpoints: Vec<_> = std::fs::read_dir(&safety_directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(
            checkpoints.len(),
            2,
            "the pre-migration and pre-restore safety checkpoints are both retained"
        );
        assert!(checkpoints
            .iter()
            .any(|checkpoint| SqliteHealthStore::inspect_backup(checkpoint)
                .unwrap()
                .players
                .iter()
                .any(|player| player.id == "old-player")));

        let reopened = SqliteHealthStore::open_file(&path, T0);
        assert!(
            reopened.is_available(),
            "{:?}",
            reopened.unavailability_reason()
        );
        let name: String = reopened
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT name FROM players WHERE id='new-player'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| StorageError::Operation(error.to_string()))
            })
            .unwrap();
        assert_eq!(name, "Restored world");
    }

    #[test]
    fn file_store_uses_wal_and_enforces_foreign_keys() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("world.sqlite3");
        let store = SqliteHealthStore::open_file(&path, T0);

        assert!(
            store.is_available(),
            "parent directory must be auto-created"
        );
        let diag = store.diagnostics().expect("diagnostics");
        assert_eq!(diag.backend, "sqlite");
        assert_eq!(diag.journal_mode.to_lowercase(), "wal");
        assert!(diag.foreign_keys);
        assert_eq!(
            diag.location_hint.as_deref(),
            Some("Life RPG application data folder")
        );
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

        assert_eq!(
            store.unavailability_reason(),
            Some("simulated disk failure")
        );
    }

    #[test]
    fn unverified_restore_state_fails_closed_for_every_store_operation() {
        let store = SqliteHealthStore::open_in_memory(T0);
        store
            .restore_requires_recovery
            .store(true, Ordering::Release);
        assert!(!store.is_available());
        assert!(store
            .unavailability_reason()
            .unwrap()
            .contains("LR-RESTORE-01"));
        assert!(matches!(
            store.diagnostics(),
            Err(StorageError::Unreachable(_))
        ));
        assert!(matches!(
            store.schema_report(),
            Err(StorageError::Unreachable(_))
        ));
        assert!(matches!(
            store.verify_round_trip("must-not-write", T0),
            Err(StorageError::Unreachable(_))
        ));
        assert!(matches!(
            store.check_integrity(),
            Err(StorageError::Unreachable(_))
        ));
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
            17,
            "status screen shows real migration history"
        );
    }
}
