//! Versioned, embedded migrations.
//!
//! Strategy choice (documented per the brief's "choose a sensible default and
//! document it" rule): we keep an explicit **`schema_migrations` ledger table**
//! rather than only SQLite's `user_version` pragma. Reasons:
//!
//! * The ledger records *when* each migration ran and its name — real history,
//!   which is the whole point of this application. A single integer cannot
//!   answer "when did the type-definition table appear?".
//! * It gives the status screen something honest to display per migration.
//! * Version numbers are still monotonic and still let us refuse to open a
//!   store written by a newer build (downgrade guard).
//!
//! Migrations are `&'static str` embedded in the binary, so no file needs to
//! ship beside the executable and no network is ever involved.
//!
//! **Never edit an applied migration.** Add a new one.

use lr_application::{MigrationRecord, SchemaReport};
use rusqlite::Connection;

use crate::error::PersistenceError;

/// One embedded migration step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Migration {
    /// Monotonic version, starting at 1.
    pub version: u32,
    /// Stable name, recorded in the ledger.
    pub name: &'static str,
    /// The SQL body, executed in a single transaction.
    pub sql: &'static str,
}

/// The full, ordered migration history known to this build.
///
/// Phase 1 shipped three foundation steps; Phase 2 and Phase 2.1 append
/// persistent world state and integrity improvements. Phase 3 adds declarative
/// rules; Phase 3.5 adds typed Concepts and history/query structures. Applied
/// bodies never change.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "0001_core_ledger",
        sql: include_str!("migrations/0001_core_ledger.sql"),
    },
    Migration {
        version: 2,
        name: "0002_health_probe",
        sql: include_str!("migrations/0002_health_probe.sql"),
    },
    Migration {
        version: 3,
        name: "0003_type_definition_registry",
        sql: include_str!("migrations/0003_type_definition_registry.sql"),
    },
    Migration {
        version: 4,
        name: "0004_phase2_domain",
        sql: include_str!("migrations/0004_phase2_domain.sql"),
    },
    Migration {
        version: 5,
        name: "0005_phase21_integrity",
        sql: include_str!("migrations/0005_phase21_integrity.sql"),
    },
    Migration {
        version: 6,
        name: "0006_phase3_rules",
        sql: include_str!("migrations/0006_phase3_rules.sql"),
    },
    Migration {
        version: 7,
        name: "0007_phase35_concepts",
        sql: include_str!("migrations/0007_phase35_concepts.sql"),
    },
];

/// Highest version this build ships.
pub fn expected_version() -> u32 {
    MIGRATIONS.iter().map(|m| m.version).max().unwrap_or(0)
}

/// Create the ledger table if it does not exist.
///
/// Kept separate from the migration bodies so the runner can always read the
/// ledger before deciding what to do.
fn ensure_ledger(conn: &Connection) -> Result<(), PersistenceError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
             version    INTEGER PRIMARY KEY,
             name       TEXT    NOT NULL,
             applied_at TEXT    NOT NULL
         );",
    )?;
    Ok(())
}

/// Highest version present in the ledger (0 when the store is empty).
pub fn applied_version(conn: &Connection) -> Result<u32, PersistenceError> {
    ensure_ledger(conn)?;
    let version: u32 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    Ok(version)
}

/// Bring the store up to [`expected_version`].
///
/// Guarantees:
///
/// * **Idempotent** — already-applied versions are skipped, so calling this on
///   every application start is correct and cheap.
/// * **Atomic per step** — each migration runs inside its own transaction
///   together with its ledger insert, so a crash can never leave a
///   half-applied step recorded as done.
/// * **Guarded against downgrade** — a store written by a newer build is
///   refused instead of silently corrupted.
///
/// Returns the versions that were applied by *this* call (empty when the store
/// was already current).
pub fn run_migrations(
    conn: &mut Connection,
    applied_at: &str,
) -> Result<Vec<u32>, PersistenceError> {
    ensure_ledger(conn)?;

    let current = applied_version(conn)?;
    let expected = expected_version();

    if current > expected {
        return Err(PersistenceError::SchemaTooNew {
            found: current,
            expected,
        });
    }

    let mut newly_applied = Vec::new();

    for migration in MIGRATIONS {
        if migration.version <= current {
            continue;
        }

        let tx = conn.transaction()?;
        tx.execute_batch(migration.sql)
            .map_err(|source| PersistenceError::Migration {
                version: migration.version,
                name: migration.name.to_string(),
                source,
            })?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![migration.version, migration.name, applied_at],
        )
        .map_err(|source| PersistenceError::Migration {
            version: migration.version,
            name: migration.name.to_string(),
            source,
        })?;
        tx.commit()?;

        newly_applied.push(migration.version);
    }

    Ok(newly_applied)
}

/// Snapshot of the schema for the status screen.
pub fn schema_report(conn: &Connection) -> Result<SchemaReport, PersistenceError> {
    ensure_ledger(conn)?;

    let mut stmt = conn.prepare("SELECT version, applied_at FROM schema_migrations")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut ledger: Vec<(u32, String)> = Vec::new();
    for row in rows {
        ledger.push(row?);
    }

    let migrations = MIGRATIONS
        .iter()
        .map(|m| {
            let applied_at = ledger
                .iter()
                .find(|(v, _)| *v == m.version)
                .map(|(_, at)| at.clone());
            MigrationRecord {
                version: m.version,
                name: m.name.to_string(),
                applied: applied_at.is_some(),
                applied_at,
            }
        })
        .collect();

    Ok(SchemaReport {
        current_version: applied_version(conn)?,
        expected_version: expected_version(),
        migrations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pragma;

    fn open_memory() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        pragma::configure(&conn).expect("configure pragmas");
        conn
    }

    const T0: &str = "2026-09-25T00:00:00+00:00";

    #[test]
    fn fresh_store_migrates_to_expected_version() {
        let mut conn = open_memory();
        assert_eq!(applied_version(&conn).unwrap(), 0);

        let applied = run_migrations(&mut conn, T0).expect("migrate");
        assert_eq!(applied, vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(applied_version(&conn).unwrap(), expected_version());
    }

    #[test]
    fn migrations_are_idempotent() {
        let mut conn = open_memory();
        let first = run_migrations(&mut conn, T0).expect("first run");
        assert_eq!(first.len(), 7);

        let second = run_migrations(&mut conn, T0).expect("second run");
        assert!(second.is_empty(), "re-run must be a no-op, got {second:?}");

        let count: u32 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 7, "ledger must not accumulate duplicates");
    }

    #[test]
    fn ledger_records_history_with_timestamps() {
        let mut conn = open_memory();
        run_migrations(&mut conn, T0).unwrap();

        let report = schema_report(&conn).unwrap();
        assert!(report.is_current());
        assert_eq!(report.migrations.len(), 7);
        assert!(report.migrations.iter().all(|m| m.applied));
        assert_eq!(
            report.migrations[0].applied_at.as_deref(),
            Some(T0),
            "applied_at is part of the historical record"
        );
        assert_eq!(report.migrations[0].state(), "applied");
    }

    #[test]
    fn partial_store_resumes_from_where_it_stopped() {
        let mut conn = open_memory();
        ensure_ledger(&conn).unwrap();

        // Simulate a store that only ever got migration 1.
        conn.execute_batch(MIGRATIONS[0].sql).unwrap();
        conn.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (1, ?1, ?2)",
            rusqlite::params![MIGRATIONS[0].name, T0],
        )
        .unwrap();

        let applied = run_migrations(&mut conn, T0).unwrap();
        assert_eq!(
            applied,
            vec![2, 3, 4, 5, 6, 7],
            "must apply only the missing steps"
        );

        let report = schema_report(&conn).unwrap();
        assert!(report.is_current());
        // The pre-existing row keeps its original timestamp: history is intact.
        let m1 = report.migrations.iter().find(|m| m.version == 1).unwrap();
        assert_eq!(m1.applied_at.as_deref(), Some(T0));
    }

    #[test]
    fn phase1_database_upgrades_to_phase2_without_losing_data() {
        let mut conn = open_memory();
        ensure_ledger(&conn).unwrap();

        // Reproduce a real Phase 1 database exactly: all original migration
        // bodies and their ledger rows, plus existing user/infrastructure data.
        for migration in &MIGRATIONS[..3] {
            conn.execute_batch(migration.sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![migration.version, migration.name, T0],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO health_probe (token, written_at) VALUES ('phase1-proof', ?1)",
            rusqlite::params![T0],
        )
        .unwrap();

        assert_eq!(run_migrations(&mut conn, T0).unwrap(), vec![4, 5, 6, 7]);
        assert!(schema_report(&conn).unwrap().is_current());

        let proof_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM health_probe WHERE token = 'phase1-proof'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(proof_rows, 1, "migration must preserve Phase 1 rows");

        let player_table: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'players'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            player_table, 1,
            "Phase 2 schema must be available after upgrade"
        );
    }

    #[test]
    fn phase21_database_upgrades_through_phase3_without_losing_world_or_rule_history() {
        let mut conn = open_memory();
        ensure_ledger(&conn).unwrap();
        for migration in &MIGRATIONS[..6] {
            conn.execute_batch(migration.sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations(version,name,applied_at) VALUES (?1,?2,?3)",
                rusqlite::params![migration.version, migration.name, T0],
            )
            .unwrap();
        }
        conn.execute("INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES ('phase21-player','Ada',2,125,?1,?1)",[T0]).unwrap();
        conn.execute("INSERT INTO player_stat_definitions(id,code,name,minimum,maximum,created_at,updated_at) VALUES ('stat-focus','focus','Focus',0,10,?1,?1)",[T0]).unwrap();
        conn.execute("INSERT INTO player_stats(player_id,stat_code,current_value,updated_at) VALUES ('phase21-player','focus',7.5,?1)",[T0]).unwrap();
        conn.execute("INSERT INTO player_state_snapshots(player_id,snapshot_date,level,current_xp,state_json,created_at) VALUES ('phase21-player','2026-09-25',2,125,'{\"schemaVersion\":1,\"stats\":[],\"activeEffects\":[]}',?1)",[T0]).unwrap();
        conn.execute("INSERT INTO transactions(player_id,transaction_type_code,resource,amount,applied_amount,occurred_at) VALUES ('phase21-player','xp','xp',125,125,?1)",[T0]).unwrap();
        conn.execute("INSERT INTO rules(id,name,trigger_kind,schema_version,definition_json,created_at,updated_at) VALUES ('rule-legacy','Preserved rule','quest_completed',1,'{\"schemaVersion\":1,\"trigger\":\"quest_completed\",\"condition\":{\"op\":\"always\"},\"actions\":[{\"action\":\"award_xp\",\"amount\":10}]}',?1,?1)",[T0]).unwrap();
        conn.execute("INSERT INTO rule_execution_history(id,chain_id,rule_id,event_kind,event_json,condition_passed,actions_json,status,depth,executed_at) VALUES ('audit-legacy','chain-legacy','rule-legacy','quest_completed','{}',1,'[]','succeeded',0,?1)",[T0]).unwrap();

        assert_eq!(run_migrations(&mut conn, T0).unwrap(), vec![7]);
        assert!(schema_report(&conn).unwrap().is_current());
        let player: (i64, i32) = conn
            .query_row(
                "SELECT current_xp,level FROM players WHERE id='phase21-player'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(player, (125, 2));
        let stat:f64=conn.query_row("SELECT current_value FROM player_stats WHERE player_id='phase21-player' AND stat_code='focus'",[],|row|row.get(0)).unwrap();
        assert_eq!(stat, 7.5);
        let snapshot: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM player_state_snapshots WHERE player_id='phase21-player'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(snapshot, 1);
        let tx: (i64, i64) = conn
            .query_row(
                "SELECT amount,applied_amount FROM transactions WHERE player_id='phase21-player'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(tx, (125, 125));
        let legacy_capture: Option<String> = conn
            .query_row(
                "SELECT captured_at FROM transactions WHERE player_id='phase21-player'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            legacy_capture, None,
            "legacy capture time stays unknown instead of being inferred"
        );
        let rules:i64=conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('rules','rule_execution_history')",[],|row|row.get(0)).unwrap();
        assert_eq!(rules, 2);
        let retained_rule: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM rules WHERE id='rule-legacy'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let retained_audit: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM rule_execution_history WHERE id='audit-legacy'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!((retained_rule, retained_audit), (1, 1));
    }

    #[test]
    fn phase21_migration_reconciles_legacy_negative_xp_with_history() {
        let mut conn = open_memory();
        ensure_ledger(&conn).unwrap();
        for migration in &MIGRATIONS[..4] {
            conn.execute_batch(migration.sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations(version,name,applied_at) VALUES (?1,?2,?3)",
                rusqlite::params![migration.version, migration.name, T0],
            )
            .unwrap();
        }
        conn.execute("INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES ('old-player','Old',1,-35,?1,?1)", [T0]).unwrap();
        conn.execute("INSERT INTO transactions(player_id,transaction_type_code,resource,amount,occurred_at) VALUES ('old-player','xp','xp',-35,?1)", [T0]).unwrap();
        assert_eq!(run_migrations(&mut conn, T0).unwrap(), vec![5, 6, 7]);
        let player: (i64, i32) = conn
            .query_row(
                "SELECT current_xp,level FROM players WHERE id='old-player'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(player, (0, 1));
        let ledger:i64=conn.query_row("SELECT COALESCE(SUM(applied_amount),0) FROM transactions WHERE player_id='old-player' AND resource='xp'",[],|r|r.get(0)).unwrap();
        assert_eq!(ledger, 0, "migration writes a matching correction event");
        assert!(conn
            .execute("UPDATE players SET current_xp=-1 WHERE id='old-player'", [])
            .is_err());
    }

    #[test]
    fn newer_store_is_refused_rather_than_corrupted() {
        let mut conn = open_memory();
        run_migrations(&mut conn, T0).unwrap();
        conn.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (999, 'from_the_future', ?1)",
            rusqlite::params![T0],
        )
        .unwrap();

        let err = run_migrations(&mut conn, T0).expect_err("must refuse");
        assert!(
            matches!(err, PersistenceError::SchemaTooNew { found: 999, .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn migration_creates_the_expected_tables() {
        let mut conn = open_memory();
        run_migrations(&mut conn, T0).unwrap();

        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();

        for expected in [
            "schema_migrations",
            "app_meta",
            "health_probe",
            "type_definitions",
            "players",
            "quests",
            "skill_trees",
            "skills",
            "transactions",
            "player_state_snapshots",
            "skill_state_snapshots",
            "comments",
            "narrative_entries",
            "effects",
        ] {
            assert!(
                tables.iter().any(|t| t == expected),
                "missing table `{expected}` in {tables:?}"
            );
        }
    }

    #[test]
    fn type_definitions_are_seeded_and_extensible_without_a_migration() {
        let mut conn = open_memory();
        run_migrations(&mut conn, T0).unwrap();

        let quest_types: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM type_definitions WHERE namespace = 'quest'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(quest_types, 5, "main/side/daily/long_term/challenge");

        // Adding a brand-new conceptual type is a data INSERT, not a migration
        // and not a source-code change. This is the extensibility requirement.
        conn.execute(
            "INSERT INTO type_definitions (namespace, code, label, sort_order, is_active, metadata_json)
             VALUES ('quest', 'raid', 'Raid', 60, 1, '{}')",
            [],
        )
        .unwrap();

        let now: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM type_definitions WHERE namespace = 'quest'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(now, 6);
    }

    #[test]
    fn type_definition_namespace_and_code_are_unique() {
        let mut conn = open_memory();
        run_migrations(&mut conn, T0).unwrap();

        let dup = conn.execute(
            "INSERT INTO type_definitions (namespace, code, label) VALUES ('quest', 'main', 'Duplicate')",
            [],
        );
        assert!(dup.is_err(), "UNIQUE(namespace, code) must be enforced");
    }
}
