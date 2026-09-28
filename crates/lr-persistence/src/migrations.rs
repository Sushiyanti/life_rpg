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
/// rules; Phase 3.5 adds typed Concepts and history/query structures; Phase 3.6
/// adds progression authority, activity, recovery, and contextual preferences;
/// Phase 5.2 adds stable Concept-only workspace-transfer references; Phase 6.1
/// adds append-only Effect history and explicit Session-to-Effect context.
/// Applied bodies never change.
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
    Migration {
        version: 8,
        name: "0008_phase36_world_semantics",
        sql: include_str!("migrations/0008_phase36_world_semantics.sql"),
    },
    Migration {
        version: 9,
        name: "0009_phase5_workspaces",
        sql: include_str!("migrations/0009_phase5_workspaces.sql"),
    },
    Migration {
        version: 10,
        name: "0010_phase51_workspace_capabilities",
        sql: include_str!("migrations/0010_phase51_workspace_capabilities.sql"),
    },
    Migration {
        version: 11,
        name: "0011_phase52_concept_transfer_keys",
        sql: include_str!("migrations/0011_phase52_concept_transfer_keys.sql"),
    },
    Migration {
        version: 12,
        name: "0012_phase61_effect_history_sessions",
        sql: include_str!("migrations/0012_phase61_effect_history_sessions.sql"),
    },
    Migration {
        version: 13,
        name: "0013_phase7_content_guidance",
        sql: include_str!("migrations/0013_phase7_content_guidance.sql"),
    },
    Migration {
        version: 14,
        name: "0014_phase8_timeline_indexes",
        sql: include_str!("migrations/0014_phase8_timeline_indexes.sql"),
    },
    Migration {
        version: 15,
        name: "0015_phase9_timeline_workspace_panels",
        sql: include_str!("migrations/0015_phase9_timeline_workspace_panels.sql"),
    },
    Migration {
        version: 16,
        name: "0016_phase10_gameplay_rules_progression",
        sql: include_str!("migrations/0016_phase10_gameplay_rules_progression.sql"),
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
        assert_eq!(
            applied,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        );
        assert_eq!(applied_version(&conn).unwrap(), expected_version());
    }

    #[test]
    fn migrations_are_idempotent() {
        let mut conn = open_memory();
        let first = run_migrations(&mut conn, T0).expect("first run");
        assert_eq!(first.len(), 16);

        let second = run_migrations(&mut conn, T0).expect("second run");
        assert!(second.is_empty(), "re-run must be a no-op, got {second:?}");

        let count: u32 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 16, "ledger must not accumulate duplicates");
    }

    #[test]
    fn ledger_records_history_with_timestamps() {
        let mut conn = open_memory();
        run_migrations(&mut conn, T0).unwrap();

        let report = schema_report(&conn).unwrap();
        assert!(report.is_current());
        assert_eq!(report.migrations.len(), 16);
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
            vec![2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
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

        assert_eq!(
            run_migrations(&mut conn, T0).unwrap(),
            vec![4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        );
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

        assert_eq!(
            run_migrations(&mut conn, T0).unwrap(),
            vec![7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        );
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
        assert_eq!(
            run_migrations(&mut conn, T0).unwrap(),
            vec![5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        );
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
    fn phase5_v9_panels_upgrade_losslessly_and_allow_repeated_sources() {
        let mut conn = open_memory();
        ensure_ledger(&conn).unwrap();
        for migration in &MIGRATIONS[..9] {
            conn.execute_batch(migration.sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations(version,name,applied_at) VALUES(?1,?2,?3)",
                rusqlite::params![migration.version, migration.name, T0],
            )
            .unwrap();
        }
        conn.execute("INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES('p','Ada',1,0,?1,?1)",[T0]).unwrap();
        conn.execute("INSERT INTO concepts(id,player_id,concept_type_code,name,created_at,updated_at) VALUES('c','p','subject','Reading',?1,?1)",[T0]).unwrap();
        conn.execute("INSERT INTO workspaces(id,player_id,name,template,is_default,created_at,updated_at) VALUES('w','p','Learning','learning',1,?1,?1)",[T0]).unwrap();
        conn.execute("INSERT INTO workspace_panels(id,workspace_id,panel_type,title,variant,density,filter_status,item_limit,sort_order,is_pinned,is_collapsed,created_at,updated_at) VALUES('old-panel','w','quests','In progress','cards','cozy','in_progress',8,3,1,0,?1,?1)",[T0]).unwrap();
        conn.execute("INSERT INTO quest_sessions(id,player_id,concept_id,started_at,status,is_active,created_at,updated_at) VALUES('old-session','p','c',?1,'in_progress',1,?1,?1)",[T0]).unwrap();
        assert_eq!(
            run_migrations(&mut conn, T0).unwrap(),
            vec![10, 11, 12, 13, 14, 15, 16]
        );
        let legacy_key: String = conn
            .query_row("SELECT transfer_key FROM concepts WHERE id='c'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(legacy_key.starts_with("concept-ref-v1-"));
        let legacy:(String,String,i64,i64,i64)=conn.query_row("SELECT title,filter_status,item_limit,sort_order,is_pinned FROM workspace_panels WHERE id='old-panel'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).unwrap();
        assert_eq!(
            legacy,
            ("In progress".into(), "active".into(), 8, 3, 1),
            "legacy Phase 5 aliases normalize to the actual Quest status without losing layout"
        );
        let indexed_session:i64=conn.query_row("SELECT COUNT(*) FROM world_search_fts WHERE kind='quest_session' AND entity_id='old-session'",[],|r|r.get(0)).unwrap();
        assert_eq!(
            indexed_session, 1,
            "existing Session search triggers remain intact after v10"
        );
        conn.execute("INSERT INTO workspace_panels(id,workspace_id,panel_type,title,variant,density,filter_status,sort_by,item_limit,sort_order,grid_span,is_visible,is_pinned,is_collapsed,created_at,updated_at) VALUES('second','w','quests','Completed','rows','compact','completed','created_desc',12,4,2,1,0,0,?1,?1)",[T0]).unwrap();
        let count:i64=conn.query_row("SELECT COUNT(*) FROM workspace_panels WHERE workspace_id='w' AND panel_type='quests'",[],|r|r.get(0)).unwrap();
        assert_eq!(count, 2);
        assert!(schema_report(&conn).unwrap().is_current());
    }

    #[test]
    fn phase10_upgrade_backfills_unique_stable_concept_transfer_keys() {
        let mut conn = open_memory();
        ensure_ledger(&conn).unwrap();
        for migration in &MIGRATIONS[..10] {
            conn.execute_batch(migration.sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations(version,name,applied_at) VALUES(?1,?2,?3)",
                rusqlite::params![migration.version, migration.name, T0],
            )
            .unwrap();
        }
        conn.execute("INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES('p','Ada',1,0,?1,?1)",[T0]).unwrap();
        conn.execute("INSERT INTO concepts(id,player_id,concept_type_code,name,created_at,updated_at) VALUES('c1','p','subject','Reading',?1,?1),('c2','p','subject','Reading',?1,?1)",[T0]).unwrap();
        assert_eq!(
            run_migrations(&mut conn, T0).unwrap(),
            vec![11, 12, 13, 14, 15, 16]
        );
        let keys: Vec<String> = conn
            .prepare("SELECT transfer_key FROM concepts ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.iter().all(|key| key.starts_with("concept-ref-v1-")));
        assert_ne!(keys[0], keys[1]);
        conn.execute("INSERT INTO concepts(id,player_id,concept_type_code,name,created_at,updated_at) VALUES('c3','p','subject','Writing',?1,?1)",[T0]).unwrap();
        let inserted: String = conn
            .query_row("SELECT transfer_key FROM concepts WHERE id='c3'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(inserted.starts_with("concept-ref-v1-"));
        assert!(conn
            .execute(
                "UPDATE concepts SET transfer_key='concept-ref-v1-changed' WHERE id='c1'",
                []
            )
            .is_err());
        assert_eq!(applied_version(&conn).unwrap(), 16);
    }

    #[test]
    fn phase12_upgrade_adds_effect_history_without_changing_existing_effects() {
        let mut conn = open_memory();
        ensure_ledger(&conn).unwrap();
        for migration in &MIGRATIONS[..11] {
            conn.execute_batch(migration.sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations(version,name,applied_at) VALUES(?1,?2,?3)",
                rusqlite::params![migration.version, migration.name, T0],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES('p','Ada',1,0,?1,?1)",
            [T0],
        ).unwrap();
        conn.execute(
            "INSERT INTO effects(id,player_id,effect_type_code,name,started_at,expires_at,intensity,created_at,updated_at) VALUES('e','p','buff','Focus',?1,NULL,1,?1,?1)",
            [T0],
        ).unwrap();

        assert_eq!(
            run_migrations(&mut conn, T0).unwrap(),
            vec![12, 13, 14, 15, 16]
        );
        let preserved: (String, Option<String>) = conn
            .query_row(
                "SELECT name,expires_at FROM effects WHERE id='e'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(preserved, ("Focus".into(), None));
        for table in ["effect_history", "session_effects"] {
            let exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "missing {table}");
        }
        assert_eq!(applied_version(&conn).unwrap(), 16);
    }

    #[test]
    fn phase7_upgrade_preserves_attachment_facts_and_closes_cross_world_updates() {
        let mut conn = open_memory();
        ensure_ledger(&conn).unwrap();
        for migration in &MIGRATIONS[..12] {
            conn.execute_batch(migration.sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations(version,name,applied_at) VALUES(?1,?2,?3)",
                rusqlite::params![migration.version, migration.name, T0],
            )
            .unwrap();
        }
        for (id, name) in [("p1", "Ada"), ("p2", "Grace")] {
            conn.execute(
                "INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES(?1,?2,1,0,?3,?3)",
                rusqlite::params![id, name, T0],
            )
            .unwrap();
        }
        for (id, player_id) in [("q1", "p1"), ("q2", "p2")] {
            conn.execute(
                "INSERT INTO quests(id,player_id,quest_type_code,title,created_at,updated_at) VALUES(?1,?2,'main',?1,?3,?3)",
                rusqlite::params![id, player_id, T0],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO narrative_entries(id,player_id,kind_namespace,kind_code,title,content,metadata_json,created_at,updated_at) VALUES('content-1','p1','narrative_entry','guide','Guide','Body','{}',?1,?1)",
            [T0],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO content_attachments(content_id,player_id,target_kind,target_id,role_code,is_active,created_at,updated_at) VALUES('content-1','p1','quest','q1','guidance',1,?1,?1),('content-1','p1','quest','q1','reading',0,?1,'2026-09-26T00:00:00Z')",
            [T0],
        )
        .unwrap();

        assert_eq!(run_migrations(&mut conn, T0).unwrap(), vec![13, 14, 15, 16]);
        let rows: Vec<(String, Option<String>)> = conn
            .prepare("SELECT role_code,removed_at FROM content_attachments ORDER BY role_code")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            rows,
            vec![
                ("guidance".into(), None),
                ("reading".into(), Some("2026-09-26T00:00:00Z".into()))
            ]
        );
        let introduction: i64 = conn.query_row("SELECT count(*) FROM type_definitions WHERE namespace='narrative_entry' AND code='introduction'", [], |row| row.get(0)).unwrap();
        assert_eq!(introduction, 1);
        assert!(conn
            .execute(
                "UPDATE content_attachments SET target_id='q2' WHERE content_id='content-1'",
                []
            )
            .is_err());
        conn.execute("DELETE FROM quests WHERE id='q1'", [])
            .unwrap();
        let retained: i64 = conn
            .query_row("SELECT count(*) FROM content_attachments", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            retained, 2,
            "target deletion must retain historical relationship facts"
        );
        let active: i64 = conn
            .query_row(
                "SELECT count(*) FROM content_attachments WHERE removed_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            active, 0,
            "target deletion removes active usage without erasing relationship history"
        );
    }

    #[test]
    fn schema_13_upgrade_adds_timeline_indexes_without_copying_world_records() {
        let mut conn = open_memory();
        ensure_ledger(&conn).unwrap();
        for migration in &MIGRATIONS[..13] {
            conn.execute_batch(migration.sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations(version,name,applied_at) VALUES(?1,?2,?3)",
                rusqlite::params![migration.version, migration.name, T0],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES('p','Ada',1,0,?1,?1)",
            [T0],
        )
        .unwrap();

        assert_eq!(run_migrations(&mut conn, T0).unwrap(), vec![14, 15, 16]);
        assert_eq!(applied_version(&conn).unwrap(), 16);
        let preserved: String = conn
            .query_row("SELECT name FROM players WHERE id='p'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(preserved, "Ada");
        for index in [
            "idx_timeline_effect_history_player_time",
            "idx_timeline_revisions_player_time",
            "idx_timeline_concept_progress_time",
            "idx_timeline_content_player_updated",
            "idx_timeline_player_snapshots_capture",
            "idx_timeline_skill_snapshots_capture",
            "idx_timeline_concept_snapshots_capture",
            "idx_timeline_quests_started",
            "idx_timeline_quests_completed",
            "idx_timeline_skills_started",
            "idx_timeline_skills_completed",
            "idx_timeline_lifecycle_player_time",
            "idx_timeline_content_relationship_created",
            "idx_timeline_content_relationship_removed",
            "idx_timeline_concepts_player_created",
            "idx_timeline_skill_trees_player_created",
            "idx_timeline_quest_stages_player_created",
            "idx_timeline_quest_branches_player_created",
        ] {
            let found: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
                    [index],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(found, 1, "missing index {index}");
        }
        let event_tables: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='timeline_events'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            event_tables, 0,
            "Timeline is a projection, not a second event store"
        );
    }

    #[test]
    fn schema_14_upgrade_preserves_workspace_panels_and_adds_closed_timeline_filters() {
        let mut conn = open_memory();
        ensure_ledger(&conn).unwrap();
        for migration in &MIGRATIONS[..14] {
            conn.execute_batch(migration.sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations(version,name,applied_at) VALUES(?1,?2,?3)",
                rusqlite::params![migration.version, migration.name, T0],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES('p','Ada',1,0,?1,?1)",
            [T0],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO concepts(id,player_id,concept_type_code,name,created_at,updated_at) VALUES('c','p','subject','Garden',?1,?1)",
            [T0],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO workspaces(id,player_id,name,template,is_default,created_at,updated_at) VALUES('w','p','Focus','focus',1,?1,?1)",
            [T0],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO workspace_panels(id,workspace_id,panel_type,title,variant,density,filter_status,filter_active,filter_type_code,filter_concept_id,filter_recent_days,sort_by,item_limit,sort_order,grid_span,is_visible,is_pinned,is_collapsed,created_at,updated_at) VALUES('existing','w','quests','Garden goals','cards','compact','active',NULL,'main','c',30,'updated_desc',12,4,2,0,1,1,?1,?1)",
            [T0],
        )
        .unwrap();

        assert_eq!(run_migrations(&mut conn, T0).unwrap(), vec![15, 16]);
        assert_eq!(applied_version(&conn).unwrap(), 16);
        let preserved: (String, String, String, i64, i64, i64, String, i64) = conn
            .query_row(
                "SELECT title,filter_status,filter_concept_id,is_visible,is_pinned,grid_span,sort_by,item_limit FROM workspace_panels WHERE id='existing'",
                [],
                |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?)),
            )
            .unwrap();
        assert_eq!(
            preserved,
            (
                "Garden goals".into(),
                "active".into(),
                "c".into(),
                0,
                1,
                2,
                "updated_desc".into(),
                12
            )
        );

        conn.execute(
            "INSERT INTO workspace_panels(id,workspace_id,panel_type,title,variant,density,filter_concept_id,filter_timeline_category,filter_timeline_entity_kind,filter_timeline_entity_id,filter_timeline_from,filter_timeline_through,sort_by,item_limit,sort_order,grid_span,is_visible,is_pinned,is_collapsed,created_at,updated_at) VALUES('timeline','w','timeline','Recent sessions','timeline','cozy','c','session','quest_session','session-1','2026-09-01','2026-09-30','timeline_oldest',10,5,2,1,0,0,?1,?1)",
            [T0],
        )
        .unwrap();
        let timeline: (String, String, String, String, String, String, String) = conn
            .query_row(
                "SELECT panel_type,filter_timeline_category,filter_timeline_entity_kind,filter_timeline_entity_id,filter_timeline_from,filter_timeline_through,sort_by FROM workspace_panels WHERE id='timeline'",
                [],
                |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?)),
            )
            .unwrap();
        assert_eq!(
            timeline,
            (
                "timeline".into(),
                "session".into(),
                "quest_session".into(),
                "session-1".into(),
                "2026-09-01".into(),
                "2026-09-30".into(),
                "timeline_oldest".into()
            )
        );
        assert!(conn.execute("INSERT INTO workspace_panels(id,workspace_id,panel_type,variant,density,filter_timeline_category,sort_by,created_at,updated_at) VALUES('bad-category','w','timeline','timeline','cozy','arbitrary','timeline_newest',?1,?1)",[T0]).is_err());
        assert!(conn.execute("INSERT INTO workspace_panels(id,workspace_id,panel_type,variant,density,filter_timeline_entity_id,sort_by,created_at,updated_at) VALUES('bad-identity','w','timeline','timeline','cozy','session-1','timeline_newest',?1,?1)",[T0]).is_err());
        assert!(conn.execute("INSERT INTO workspace_panels(id,workspace_id,panel_type,variant,density,filter_timeline_from,filter_timeline_through,sort_by,created_at,updated_at) VALUES('bad-range','w','timeline','timeline','cozy','2026-10-01','2026-09-01','timeline_newest',?1,?1)",[T0]).is_err());
        assert!(conn.execute("INSERT INTO workspace_panels(id,workspace_id,panel_type,variant,density,filter_timeline_category,sort_by,created_at,updated_at) VALUES('wrong-source','w','quests','cards','cozy','session','updated_desc',?1,?1)",[T0]).is_err());
        let fk_violations: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(fk_violations, 0);
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
            "effect_history",
            "session_effects",
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

    #[test]
    fn schema_15_upgrade_preserves_gameplay_records_and_backfills_manual_authority() {
        let mut conn = open_memory();
        ensure_ledger(&conn).unwrap();
        for migration in &MIGRATIONS[..15] {
            conn.execute_batch(migration.sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations(version,name,applied_at) VALUES(?1,?2,?3)",
                rusqlite::params![migration.version, migration.name, T0],
            )
            .unwrap();
        }
        conn.execute("INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES('p10','Ada',3,42,?1,?1)", [T0]).unwrap();
        conn.execute("INSERT INTO skill_trees(id,player_id,tree_type_namespace,tree_type_code,name,created_at,updated_at) VALUES('tree10','p10','skill_tree','programming','Programming',?1,?1)", [T0]).unwrap();
        conn.execute("INSERT INTO skills(id,skill_tree_id,skill_type_namespace,skill_type_code,name,level,current_xp,created_at,updated_at) VALUES('skill10','tree10','skill','core','Rust',4,120,?1,?1)", [T0]).unwrap();
        conn.execute("INSERT INTO effects(id,player_id,effect_type_namespace,effect_type_code,name,started_at,deactivated_at,created_at,updated_at) VALUES('effect10','p10','effect','buff','Focus',?1,?1,?1,?1)", [T0]).unwrap();
        conn.execute("INSERT INTO effect_history(id,player_id,effect_id,event_kind,recorded_at,previous_state_json,current_state_json) VALUES('effect-created','p10','effect10','created',?1,NULL,'{}'),('effect-off','p10','effect10','manually_deactivated',?1,'{}','{}')", [T0]).unwrap();
        conn.execute("INSERT INTO rules(id,name,trigger_kind,schema_version,definition_json,created_at,updated_at) VALUES('legacy-rule','Legacy rule','quest_completed',1,'{\"schemaVersion\":1,\"trigger\":\"quest_completed\",\"condition\":{\"op\":\"always\"},\"actions\":[{\"kind\":\"award_xp\",\"amount\":5,\"reason\":null}]}',?1,?1)", [T0]).unwrap();
        conn.execute("INSERT INTO rule_execution_history(id,chain_id,rule_id,event_kind,event_json,condition_passed,actions_json,status,depth,executed_at) VALUES('legacy-audit','legacy-chain','legacy-rule','quest_completed','{}',1,'[]','succeeded',0,?1)", [T0]).unwrap();

        assert_eq!(run_migrations(&mut conn, T0).unwrap(), vec![16]);
        assert_eq!(applied_version(&conn).unwrap(), 16);
        let skill: (i32, i64, String, String) = conn.query_row(
            "SELECT level,current_xp,availability,availability_control FROM skills WHERE id='skill10'",
            [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)),
        ).unwrap();
        assert_eq!(skill, (4, 120, "available".into(), "manual".into()));
        let effect_source: String = conn
            .query_row(
                "SELECT deactivation_source FROM effects WHERE id='effect10'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(effect_source, "manual");
        let old_history: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM effect_history WHERE effect_id='effect10'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(old_history, 2, "append-only lifecycle facts remain present");
        let rule_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM rules WHERE id='legacy-rule'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let audit_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM rule_execution_history WHERE id='legacy-audit'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!((rule_count, audit_count), (1, 1));

        conn.execute("INSERT INTO skill_history(id,player_id,skill_id,event_kind,source,recorded_at,previous_state_json,current_state_json) VALUES('skill-hist','p10','skill10','availability_changed','manual',?1,'{}','{}')", [T0]).unwrap();
        assert!(
            conn.execute(
                "UPDATE skill_history SET source='rule' WHERE id='skill-hist'",
                []
            )
            .is_err(),
            "Skill history is append-only"
        );
        assert!(
            conn.execute(
                "UPDATE effects SET deactivated_at=NULL WHERE id='effect10'",
                []
            )
            .is_err(),
            "source and deactivation timestamp must remain paired"
        );

        conn.execute("INSERT INTO rules(id,name,trigger_kind,schema_version,definition_json,created_at,updated_at) VALUES('new-rule','Skill XP rule','skill_xp_changed',1,'{\"schemaVersion\":1,\"trigger\":\"skill_xp_changed\",\"condition\":{\"op\":\"always\"},\"actions\":[]}',?1,?1)", [T0]).unwrap();
        conn.execute("INSERT INTO rule_execution_history(id,chain_id,rule_id,event_kind,event_json,condition_passed,actions_json,status,depth,executed_at) VALUES('new-audit','new-chain','new-rule','skill_xp_changed','{}',1,'[]','succeeded',0,?1)", [T0]).unwrap();
        let new_rule: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM rule_execution_history WHERE event_kind='skill_xp_changed'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            new_rule, 1,
            "schema 16 admits new typed gameplay event kinds"
        );
    }
}
