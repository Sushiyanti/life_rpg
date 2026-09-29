//! Transaction-consistent world backups, validation, and staged restore.
//!
//! Backup files are a small versioned envelope around a standalone SQLite
//! snapshot produced with SQLite's online-backup API. The manifest contains no
//! installation path or credentials; the database payload is the complete world.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::DateTime;
use rusqlite::{backup::Backup, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use thiserror::Error;

use crate::{migrations, pragma};

const MAGIC: &[u8] = b"LIFERPG-BACKUP\0";
const FORMAT_VERSION: u16 = 1;
const HEADER_SIZE: u64 = MAGIC.len() as u64 + 2 + 4;
const MAX_BACKUP_BYTES: u64 = 512 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u32 = 1024 * 1024;
const MAX_PLAYERS: usize = 10_000;
const DB_CHUNK: usize = 64 * 1024;
const REQUIRED_SCHEMA_17_TABLES: &[&str] = &[
    "app_meta",
    "comments",
    "concept_association_types",
    "concept_associations",
    "concept_entity_links",
    "concept_progress_history",
    "concept_progress_tracks",
    "concept_relationship_types",
    "concept_relationships",
    "concept_state_snapshots",
    "concepts",
    "content_attachment_roles",
    "content_attachments",
    "effect_history",
    "effects",
    "entity_lifecycle",
    "entity_lifecycle_history",
    "entity_revision_restores",
    "entity_revisions",
    "health_probe",
    "narrative_entries",
    "player_stat_definitions",
    "player_state_snapshots",
    "player_stats",
    "players",
    "presentation_preferences",
    "progress_suggestions",
    "progress_track_definitions",
    "quest_branches",
    "quest_sessions",
    "quest_stages",
    "quests",
    "rule_execution_history",
    "rules",
    "schema_migrations",
    "session_effects",
    "skill_history",
    "skill_state_snapshots",
    "skill_trees",
    "skills",
    "tag_relationships",
    "tags",
    "transactions",
    "type_definitions",
    "workspace_panels",
    "workspaces",
    "world_search_fts",
];

#[derive(Debug, Error)]
pub enum BackupError {
    #[error("backup file could not be read or written")]
    Io(#[from] std::io::Error),
    #[error("backup metadata is invalid")]
    Json(#[from] serde_json::Error),
    #[error("SQLite backup or inspection failed")]
    Sqlite(#[from] rusqlite::Error),
    #[error("schema migration failed during restore staging")]
    Migration(#[from] crate::error::PersistenceError),
    #[error("backup file is malformed or incomplete")]
    Malformed,
    #[error("backup format is newer than this application supports")]
    UnsupportedFormat,
    #[error("backup was created by a newer schema this application cannot open")]
    UnsupportedSchema { found: u32, expected: u32 },
    #[error("backup checksum does not match its database snapshot")]
    ChecksumMismatch,
    #[error("backup integrity checks did not pass")]
    Integrity,
    #[error("backup belongs to a different world and needs explicit confirmation")]
    WorldMismatch,
    #[error("backup destination already exists; choose a new file name")]
    DestinationExists,
    #[error("the current world is not available as a file-backed database")]
    NotFileBacked,
    #[error("backup is larger than this application allows")]
    TooLarge,
    #[error("a safety backup could not be created; the restore or migration was not applied")]
    SafetyBackup,
    #[error("restore failed; the live world was restored from its verified safety backup")]
    ReplacementFailed,
    #[error("restore and rollback failed; the verified pre-restore backup remains available")]
    RollbackFailed,
    #[error("an existing database has no valid migration history")]
    InvalidMigrationLedger,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayerDescriptor {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackupCompatibility {
    SameSchema,
    OlderSchema,
    NewerUnsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub product: String,
    pub application_version: String,
    pub format_version: u16,
    pub schema_version: u32,
    pub created_at: String,
    pub players: Vec<PlayerDescriptor>,
    pub database_bytes: u64,
    pub sha256: String,
    pub compatibility: BackupCompatibility,
    pub integrity: IntegrityReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub healthy: bool,
    pub schema_version: u32,
    pub integrity_check_ok: bool,
    pub foreign_key_violations: u64,
    pub missing_required_tables: Vec<String>,
    pub malformed_configuration_rows: u64,
    pub cross_player_tag_relationships: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreInfo {
    pub backup: BackupInfo,
    pub safety_backup_path: String,
    pub restored_schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    product: String,
    application_version: String,
    format_version: u16,
    schema_version: u32,
    created_at: String,
    players: Vec<PlayerDescriptor>,
    database_bytes: u64,
    sha256: String,
}

pub fn create_backup(
    source: &Connection,
    destination: &Path,
    created_at: &str,
) -> Result<BackupInfo, BackupError> {
    let snapshot_dir = tempfile::tempdir()?;
    let snapshot_path = snapshot_dir.path().join("world.sqlite3");
    {
        let mut snapshot = Connection::open(&snapshot_path)?;
        let backup = Backup::new(source, &mut snapshot)?;
        backup.run_to_completion(128, Duration::from_millis(10), None)?;
    }

    let (schema_version, players, integrity) = inspect_connection_path(&snapshot_path)?;
    if !integrity.healthy {
        return Err(BackupError::Integrity);
    }
    let (sha256, database_bytes) = hash_file(&snapshot_path)?;
    if database_bytes > MAX_BACKUP_BYTES {
        return Err(BackupError::TooLarge);
    }
    let manifest = Manifest {
        product: "Life RPG".into(),
        application_version: env!("CARGO_PKG_VERSION").into(),
        format_version: FORMAT_VERSION,
        schema_version,
        created_at: canonical_timestamp(created_at)?,
        players,
        database_bytes,
        sha256: hex_digest(&sha256),
    };
    let manifest_bytes = serde_json::to_vec(&manifest)?;
    if manifest_bytes.len() > MAX_MANIFEST_BYTES as usize {
        return Err(BackupError::TooLarge);
    }

    let parent = parent_dir(destination);
    if !parent.is_dir() {
        return Err(BackupError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "backup destination directory is unavailable",
        )));
    }
    let temporary = NamedTempFile::new_in(parent)?;
    {
        let mut writer = temporary.as_file();
        writer.write_all(MAGIC)?;
        writer.write_all(&FORMAT_VERSION.to_be_bytes())?;
        writer.write_all(&(manifest_bytes.len() as u32).to_be_bytes())?;
        writer.write_all(&manifest_bytes)?;
        let mut snapshot = File::open(&snapshot_path)?;
        std::io::copy(&mut snapshot, &mut writer)?;
        writer.flush()?;
        writer.sync_all()?;
    }

    // Validate the complete temporary artifact before it becomes visible at the
    // user-selected destination. Existing files remain protected by the
    // no-clobber publication below.
    let _ = read_and_validate(temporary.path())?;

    temporary.persist_noclobber(destination).map_err(|error| {
        if error.error.kind() == std::io::ErrorKind::AlreadyExists {
            BackupError::DestinationExists
        } else {
            BackupError::Io(error.error)
        }
    })?;
    sync_directory(parent)?;

    match read_and_validate(destination) {
        Ok((info, _extracted)) => Ok(info),
        Err(error) => {
            // This path was created exclusively by this call. Remove it if
            // possible so a failed new backup cannot be mistaken for a valid
            // archive; never touch a prior destination.
            let _ = fs::remove_file(destination);
            let _ = sync_directory(parent);
            Err(error)
        }
    }
}

/// Validate a backup without changing the supplied file.
pub fn inspect_backup(path: &Path) -> Result<BackupInfo, BackupError> {
    let (info, _extracted) = read_and_validate(path)?;
    Ok(info)
}

pub fn check_integrity(connection: &Connection) -> Result<IntegrityReport, BackupError> {
    let (schema_version, _players, report) = inspect_connection(connection)?;
    let mut report = report;
    report.schema_version = schema_version;
    Ok(report)
}

pub fn restore_backup(
    destination: &mut Connection,
    backup_path: &Path,
    safety_directory: &Path,
    expected_sha256: &str,
    confirm_different_world: bool,
    now: &str,
) -> Result<RestoreInfo, BackupError> {
    let (backup, extracted) = read_and_validate(backup_path)?;
    if backup.sha256 != expected_sha256 {
        return Err(BackupError::ChecksumMismatch);
    }
    if matches!(backup.compatibility, BackupCompatibility::NewerUnsupported) {
        return Err(BackupError::UnsupportedSchema {
            found: backup.schema_version,
            expected: migrations::expected_version(),
        });
    }

    let current_players = read_players(destination)?;
    let current_ids: Vec<_> = current_players
        .iter()
        .map(|player| player.id.as_str())
        .collect();
    let backup_ids: Vec<_> = backup
        .players
        .iter()
        .map(|player| player.id.as_str())
        .collect();
    if current_ids != backup_ids && !confirm_different_world {
        return Err(BackupError::WorldMismatch);
    }

    // Preflight on a separate staged database so invalid content and failed
    // migrations never touch the live world or the selected backup.
    let staged_dir = tempfile::tempdir()?;
    let staged_path = staged_dir.path().join("staged.sqlite3");
    fs::copy(extracted.path(), &staged_path)?;
    let mut staged = Connection::open(&staged_path)?;
    pragma::configure(&staged)?;
    migrations::run_migrations(&mut staged, now)?;
    let (staged_schema, staged_players, staged_integrity) = inspect_connection(&staged)?;
    if staged_schema != migrations::expected_version()
        || staged_players != backup.players
        || !staged_integrity.healthy
    {
        return Err(BackupError::Integrity);
    }

    fs::create_dir_all(safety_directory).map_err(|_| BackupError::SafetyBackup)?;
    let safety_path = unique_safety_path(safety_directory, "pre-restore", now);
    create_backup(destination, &safety_path, now).map_err(|_| BackupError::SafetyBackup)?;

    // The live replacement is the only phase that can alter the current world.
    // If copying, reconfiguration, or post-copy validation fails, immediately
    // restore from the already validated safety archive. Never report success
    // unless the restored database passes the complete bounded integrity check.
    let replacement = (|| -> Result<(u32, Vec<PlayerDescriptor>), BackupError> {
        copy_database(&staged, destination)?;
        pragma::configure(destination)?;
        let (schema, players, integrity) = inspect_connection(destination)?;
        if schema != migrations::expected_version()
            || players != backup.players
            || !integrity.healthy
        {
            return Err(BackupError::Integrity);
        }
        Ok((schema, players))
    })();
    let restored_schema = match replacement {
        Ok((schema, _players)) => schema,
        Err(_) => {
            if restore_safety_snapshot(destination, &safety_path).is_ok() {
                return Err(BackupError::ReplacementFailed);
            }
            return Err(BackupError::RollbackFailed);
        }
    };

    Ok(RestoreInfo {
        backup,
        safety_backup_path: safety_path.to_string_lossy().into_owned(),
        restored_schema_version: restored_schema,
    })
}

fn copy_database(source: &Connection, destination: &mut Connection) -> Result<(), BackupError> {
    let backup = Backup::new(source, destination)?;
    backup.run_to_completion(128, Duration::from_millis(10), None)?;
    Ok(())
}

fn restore_safety_snapshot(
    destination: &mut Connection,
    safety_path: &Path,
) -> Result<(), BackupError> {
    let (safety_info, extracted) = read_and_validate(safety_path)?;
    let source = Connection::open_with_flags(extracted.path(), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    copy_database(&source, destination)?;
    pragma::configure(destination)?;
    let (schema, players, integrity) = inspect_connection(destination)?;
    if schema != safety_info.schema_version || players != safety_info.players || !integrity.healthy
    {
        return Err(BackupError::Integrity);
    }
    Ok(())
}

/// Create a verified pre-migration checkpoint before an existing database is
/// upgraded. It is intentionally retained after a successful migration.
pub fn create_migration_safety_backup(
    source: &Connection,
    safety_directory: &Path,
    from_version: u32,
    now: &str,
) -> Result<PathBuf, BackupError> {
    fs::create_dir_all(safety_directory).map_err(|_| BackupError::SafetyBackup)?;
    let path = unique_safety_path(
        safety_directory,
        &format!("pre-migration-v{from_version}"),
        now,
    );
    create_backup(source, &path, now).map_err(|_| BackupError::SafetyBackup)?;
    Ok(path)
}

/// Read and verify a backup archive, returning an extracted, temporary SQLite
/// file whose lifetime is tied to the returned guard.
fn read_and_validate(path: &Path) -> Result<(BackupInfo, NamedTempFile), BackupError> {
    let mut archive = File::open(path)?;
    let file_len = archive.metadata()?.len();
    if file_len < HEADER_SIZE + 100
        || file_len > MAX_BACKUP_BYTES + MAX_MANIFEST_BYTES as u64 + HEADER_SIZE
    {
        return Err(
            if file_len > MAX_BACKUP_BYTES + MAX_MANIFEST_BYTES as u64 + HEADER_SIZE {
                BackupError::TooLarge
            } else {
                BackupError::Malformed
            },
        );
    }
    let mut magic = vec![0u8; MAGIC.len()];
    archive
        .read_exact(&mut magic)
        .map_err(|_| BackupError::Malformed)?;
    if magic != MAGIC {
        return Err(BackupError::Malformed);
    }
    let mut version = [0u8; 2];
    archive
        .read_exact(&mut version)
        .map_err(|_| BackupError::Malformed)?;
    if u16::from_be_bytes(version) != FORMAT_VERSION {
        return Err(BackupError::UnsupportedFormat);
    }
    let mut manifest_len = [0u8; 4];
    archive
        .read_exact(&mut manifest_len)
        .map_err(|_| BackupError::Malformed)?;
    let manifest_len = u32::from_be_bytes(manifest_len);
    if manifest_len == 0 || manifest_len > MAX_MANIFEST_BYTES {
        return Err(BackupError::Malformed);
    }
    let mut manifest_bytes = vec![0u8; manifest_len as usize];
    archive
        .read_exact(&mut manifest_bytes)
        .map_err(|_| BackupError::Malformed)?;
    let manifest: Manifest =
        serde_json::from_slice(&manifest_bytes).map_err(|_| BackupError::Malformed)?;
    if manifest.product != "Life RPG"
        || manifest.format_version != FORMAT_VERSION
        || canonical_timestamp(&manifest.created_at).is_err()
        || manifest.players.len() > MAX_PLAYERS
    {
        return Err(BackupError::Malformed);
    }
    let data_offset = HEADER_SIZE + manifest_len as u64;
    let actual_db_len = file_len
        .checked_sub(data_offset)
        .ok_or(BackupError::Malformed)?;
    if actual_db_len < 100
        || actual_db_len > MAX_BACKUP_BYTES
        || actual_db_len != manifest.database_bytes
    {
        return Err(if actual_db_len > MAX_BACKUP_BYTES {
            BackupError::TooLarge
        } else {
            BackupError::Malformed
        });
    }

    let extracted = NamedTempFile::new()?;
    let mut writer = extracted.as_file();
    let mut hasher = Sha256::new();
    let mut remaining = actual_db_len;
    let mut buffer = vec![0u8; DB_CHUNK];
    while remaining > 0 {
        let take = remaining.min(buffer.len() as u64) as usize;
        archive
            .read_exact(&mut buffer[..take])
            .map_err(|_| BackupError::Malformed)?;
        hasher.update(&buffer[..take]);
        writer.write_all(&buffer[..take])?;
        remaining -= take as u64;
    }
    writer.flush()?;
    writer.sync_all()?;
    if hex_digest(&hasher.finalize()) != manifest.sha256 {
        return Err(BackupError::ChecksumMismatch);
    }

    let (schema_version, players, integrity) = inspect_connection_path(extracted.path())?;
    if schema_version != manifest.schema_version
        || players != manifest.players
        || !integrity.healthy
    {
        return Err(BackupError::Integrity);
    }
    let compatibility = if schema_version > migrations::expected_version() {
        BackupCompatibility::NewerUnsupported
    } else if schema_version == migrations::expected_version() {
        BackupCompatibility::SameSchema
    } else {
        BackupCompatibility::OlderSchema
    };
    Ok((
        BackupInfo {
            product: manifest.product,
            application_version: manifest.application_version,
            format_version: manifest.format_version,
            schema_version,
            created_at: manifest.created_at,
            players,
            database_bytes: actual_db_len,
            sha256: manifest.sha256,
            compatibility,
            integrity,
        },
        extracted,
    ))
}

fn inspect_connection_path(
    path: &Path,
) -> Result<(u32, Vec<PlayerDescriptor>, IntegrityReport), BackupError> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    inspect_connection(&connection)
}

fn inspect_connection(
    connection: &Connection,
) -> Result<(u32, Vec<PlayerDescriptor>, IntegrityReport), BackupError> {
    let schema_version = read_schema_version(connection)?;
    let players = if table_exists(connection, "players")? {
        read_players(connection)?
    } else {
        Vec::new()
    };
    let integrity_check_ok = {
        let mut statement = connection.prepare("PRAGMA integrity_check")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut okay = true;
        let mut any = false;
        for row in rows {
            any = true;
            if row? != "ok" {
                okay = false;
            }
        }
        any && okay
    };
    let foreign_key_violations = {
        let mut statement = connection.prepare("PRAGMA foreign_key_check")?;
        let rows = statement.query_map([], |_| Ok(()))?;
        let mut count = 0u64;
        for row in rows {
            row?;
            count = count.saturating_add(1);
        }
        count
    };
    let mut missing_required_tables = Vec::new();
    let mut required = vec!["schema_migrations"];
    if schema_version >= 4 {
        required.extend(["players", "quests", "skill_trees"]);
    }
    if schema_version >= 17 {
        required.extend(REQUIRED_SCHEMA_17_TABLES.iter().copied());
    }
    for table in required {
        if !table_exists(connection, table)? {
            missing_required_tables.push(table.to_string());
        }
    }

    let malformed_configuration_rows = malformed_config_count(connection, schema_version)?;
    let cross_player_tag_relationships = if schema_version >= 17
        && table_exists(connection, "tag_relationships")?
    {
        connection.query_row(
            "SELECT COUNT(*) FROM tag_relationships r WHERE NOT (
                (r.target_kind='quest' AND EXISTS(SELECT 1 FROM quests t WHERE t.id=r.target_id AND t.player_id=r.player_id)) OR
                (r.target_kind='quest_stage' AND EXISTS(SELECT 1 FROM quest_stages t WHERE t.id=r.target_id AND t.player_id=r.player_id)) OR
                (r.target_kind='quest_branch' AND EXISTS(SELECT 1 FROM quest_branches t WHERE t.id=r.target_id AND t.player_id=r.player_id)) OR
                (r.target_kind='quest_session' AND EXISTS(SELECT 1 FROM quest_sessions t WHERE t.id=r.target_id AND t.player_id=r.player_id)) OR
                (r.target_kind='skill_tree' AND EXISTS(SELECT 1 FROM skill_trees t WHERE t.id=r.target_id AND t.player_id=r.player_id)) OR
                (r.target_kind='skill' AND EXISTS(SELECT 1 FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE s.id=r.target_id AND t.player_id=r.player_id)) OR
                (r.target_kind='concept' AND EXISTS(SELECT 1 FROM concepts t WHERE t.id=r.target_id AND t.player_id=r.player_id)) OR
                (r.target_kind='effect' AND EXISTS(SELECT 1 FROM effects t WHERE t.id=r.target_id AND t.player_id=r.player_id)) OR
                (r.target_kind='narrative_entry' AND EXISTS(SELECT 1 FROM narrative_entries t WHERE t.id=r.target_id AND t.player_id=r.player_id)) OR
                (r.target_kind='comment' AND EXISTS(SELECT 1 FROM comments t WHERE CAST(t.id AS TEXT)=r.target_id AND t.author_player_id=r.player_id))
            )",
            [],
            |row| row.get::<_, u64>(0),
        )?
    } else {
        0
    };

    let healthy = integrity_check_ok
        && foreign_key_violations == 0
        && missing_required_tables.is_empty()
        && malformed_configuration_rows == 0
        && cross_player_tag_relationships == 0;
    Ok((
        schema_version,
        players,
        IntegrityReport {
            healthy,
            schema_version,
            integrity_check_ok,
            foreign_key_violations,
            missing_required_tables,
            malformed_configuration_rows,
            cross_player_tag_relationships,
        },
    ))
}

fn malformed_config_count(
    connection: &Connection,
    schema_version: u32,
) -> Result<u64, BackupError> {
    let mut count = 0u64;
    if schema_version >= 4 && table_exists(connection, "players")? {
        count = count.saturating_add(connection.query_row(
            "SELECT COUNT(*) FROM players WHERE NOT json_valid(metadata_json)",
            [],
            |row| row.get(0),
        )?);
    }
    if schema_version >= 6 && table_exists(connection, "rules")? {
        count = count.saturating_add(connection.query_row(
            "SELECT COUNT(*) FROM rules WHERE NOT json_valid(definition_json) OR NOT json_valid(metadata_json)", [], |row| row.get(0),
        )?);
    }
    if schema_version >= 17 && table_exists(connection, "workspace_panels")? {
        count = count.saturating_add(connection.query_row(
            "SELECT COUNT(*) FROM workspace_panels WHERE NOT json_valid(filter_tag_ids_json) OR json_type(filter_tag_ids_json)<>'array'", [], |row| row.get(0),
        )?);
    }
    Ok(count)
}

fn read_schema_version(connection: &Connection) -> Result<u32, BackupError> {
    if !table_exists(connection, "schema_migrations")? {
        return Err(BackupError::Malformed);
    }
    let version: u32 = connection.query_row(
        "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    if version == 0 {
        return Err(BackupError::Malformed);
    }
    migrations::validate_ledger(connection).map_err(|_| BackupError::InvalidMigrationLedger)?;
    Ok(version)
}

fn table_exists(connection: &Connection, table: &str) -> Result<bool, BackupError> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [table],
        |row| row.get(0),
    )?)
}

fn read_players(connection: &Connection) -> Result<Vec<PlayerDescriptor>, BackupError> {
    let mut statement = connection.prepare("SELECT id,name FROM players ORDER BY id LIMIT ?1")?;
    let rows = statement.query_map([MAX_PLAYERS as i64 + 1], |row| {
        Ok(PlayerDescriptor {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;
    let mut players = Vec::new();
    for row in rows {
        players.push(row?);
    }
    if players.len() > MAX_PLAYERS {
        return Err(BackupError::TooLarge);
    }
    Ok(players)
}

fn canonical_timestamp(value: &str) -> Result<String, BackupError> {
    let parsed = DateTime::parse_from_rfc3339(value).map_err(|_| BackupError::Malformed)?;
    Ok(parsed.to_rfc3339())
}

fn hash_file(path: &Path) -> Result<([u8; 32], u64), BackupError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = vec![0u8; DB_CHUNK];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read as u64);
        hasher.update(&buffer[..read]);
    }
    Ok((hasher.finalize().into(), total))
}

fn hex_digest(digest: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 15) as usize] as char);
    }
    out
}

fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn sync_directory(path: &Path) -> Result<(), BackupError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

fn unique_safety_path(directory: &Path, kind: &str, now: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let stamp = DateTime::parse_from_rfc3339(now)
        .map(|value| value.format("%Y%m%dT%H%M%S").to_string())
        .unwrap_or_else(|_| "unknown-time".into());
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    loop {
        let suffix = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = directory.join(format!(
            "life-rpg-{kind}-{stamp}-{}-{nanos}-{suffix}.liferpg-backup",
            std::process::id()
        ));
        if !path.exists() {
            return path;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{migrations, pragma};

    const T0: &str = "2026-09-29T00:00:00+00:00";

    fn populated_store(path: &Path) -> Connection {
        let mut connection = Connection::open(path).unwrap();
        pragma::configure(&connection).unwrap();
        migrations::run_migrations(&mut connection, T0).unwrap();
        connection.execute(
            "INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES('player-a','Ada',3,125,?1,?1)", [T0],
        ).unwrap();
        connection.execute(
            "INSERT INTO quests(id,player_id,quest_type_code,title,created_at,updated_at) VALUES('quest-a','player-a','main','World backup test',?1,?1)", [T0],
        ).unwrap();
        connection
    }

    #[test]
    fn valid_backup_contains_identity_schema_checksum_and_consistent_sqlite_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let source_path = dir.path().join("world.sqlite3");
        let source = populated_store(&source_path);
        source.execute_batch("PRAGMA wal_autocheckpoint=0; BEGIN; UPDATE players SET current_xp=777 WHERE id='player-a'; COMMIT;").unwrap();
        let destination = dir.path().join("world.liferpg-backup");
        let info = create_backup(&source, &destination, T0).unwrap();
        assert_eq!(info.product, "Life RPG");
        assert_eq!(info.schema_version, 17);
        assert_eq!(
            info.players,
            vec![PlayerDescriptor {
                id: "player-a".into(),
                name: "Ada".into()
            }]
        );
        assert!(info.integrity.healthy);
        assert_eq!(inspect_backup(&destination).unwrap(), info);
        let (validated, extracted) = read_and_validate(&destination).unwrap();
        assert_eq!(validated.sha256, info.sha256);
        let snapshot = Connection::open(extracted.path()).unwrap();
        let xp: i64 = snapshot
            .query_row(
                "SELECT current_xp FROM players WHERE id='player-a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(xp, 777, "committed WAL state must be included");
        let quests: i64 = snapshot
            .query_row(
                "SELECT COUNT(*) FROM quests WHERE id='quest-a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(quests, 1);
    }

    #[test]
    fn backup_never_overwrites_an_existing_destination() {
        let dir = tempfile::tempdir().unwrap();
        let source = populated_store(&dir.path().join("world.sqlite3"));
        let destination = dir.path().join("existing.liferpg-backup");
        fs::write(&destination, b"preserve this").unwrap();
        assert!(matches!(
            create_backup(&source, &destination, T0),
            Err(BackupError::DestinationExists)
        ));
        assert_eq!(fs::read(destination).unwrap(), b"preserve this");
    }

    #[test]
    fn malformed_and_checksum_tampered_backups_are_rejected_without_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let source = populated_store(&dir.path().join("world.sqlite3"));
        let backup = dir.path().join("world.liferpg-backup");
        create_backup(&source, &backup, T0).unwrap();
        let mut bytes = fs::read(&backup).unwrap();
        *bytes.last_mut().unwrap() ^= 0x01;
        fs::write(&backup, &bytes).unwrap();
        assert!(matches!(
            inspect_backup(&backup),
            Err(BackupError::ChecksumMismatch)
        ));
        assert_eq!(
            fs::read(&backup).unwrap(),
            bytes,
            "inspection must not mutate the selected backup"
        );
        let malformed = dir.path().join("malformed.liferpg-backup");
        fs::write(&malformed, b"not a Life RPG backup").unwrap();
        assert!(matches!(
            inspect_backup(&malformed),
            Err(BackupError::Malformed)
        ));
    }

    #[test]
    fn restore_preserves_the_current_world_on_wrong_world_and_preflight_failure() {
        let dir = tempfile::tempdir().unwrap();
        let source = populated_store(&dir.path().join("source.sqlite3"));
        let backup_path = dir.path().join("backup.liferpg-backup");
        let info = create_backup(&source, &backup_path, T0).unwrap();
        let mut current = populated_store(&dir.path().join("current.sqlite3"));
        current.execute("INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES('player-b','Different world',1,0,?1,?1)", [T0]).unwrap();
        let result = restore_backup(
            &mut current,
            &backup_path,
            &dir.path().join("safety"),
            &info.sha256,
            false,
            T0,
        );
        assert!(matches!(result, Err(BackupError::WorldMismatch)));
        let name: String = current
            .query_row("SELECT name FROM players WHERE id='player-a'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(name, "Ada");
        assert_eq!(
            current
                .query_row::<String, _, _>(
                    "SELECT name FROM players WHERE id='player-b'",
                    [],
                    |row| row.get(0)
                )
                .unwrap(),
            "Different world"
        );
        assert!(
            !dir.path().join("safety").exists(),
            "rejected preview must not create or replace safety files"
        );
    }

    #[test]
    fn restore_refuses_to_replace_an_invalid_current_world_without_a_verified_checkpoint() {
        let dir = tempfile::tempdir().unwrap();
        let source = populated_store(&dir.path().join("source.sqlite3"));
        let backup_path = dir.path().join("backup.liferpg-backup");
        let info = create_backup(&source, &backup_path, T0).unwrap();
        let backup_bytes = fs::read(&backup_path).unwrap();
        let mut current = populated_store(&dir.path().join("current.sqlite3"));
        current
            .execute_batch("PRAGMA ignore_check_constraints=ON")
            .unwrap();
        current
            .execute(
                "UPDATE players SET metadata_json='not-json' WHERE id='player-a'",
                [],
            )
            .unwrap();

        let result = restore_backup(
            &mut current,
            &backup_path,
            &dir.path().join("safety"),
            &info.sha256,
            true,
            T0,
        );
        assert!(matches!(result, Err(BackupError::SafetyBackup)));
        let metadata: String = current
            .query_row(
                "SELECT metadata_json FROM players WHERE id='player-a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            metadata, "not-json",
            "a failed safety checkpoint must leave the live file untouched"
        );
        assert_eq!(
            fs::read(&backup_path).unwrap(),
            backup_bytes,
            "the selected archive is never changed"
        );
        assert!(fs::read_dir(dir.path().join("safety"))
            .unwrap()
            .next()
            .is_none());
    }

    #[test]
    fn valid_restore_migrates_stage_then_replaces_current_with_verified_safety_backup() {
        let dir = tempfile::tempdir().unwrap();
        let source = populated_store(&dir.path().join("source.sqlite3"));
        let backup_path = dir.path().join("backup.liferpg-backup");
        let info = create_backup(&source, &backup_path, T0).unwrap();
        let mut current = populated_store(&dir.path().join("current.sqlite3"));
        current
            .execute(
                "UPDATE players SET name='Before restore' WHERE id='player-a'",
                [],
            )
            .unwrap();
        let result = restore_backup(
            &mut current,
            &backup_path,
            &dir.path().join("safety"),
            &info.sha256,
            true,
            T0,
        )
        .unwrap();
        assert_eq!(result.restored_schema_version, 17);
        let name: String = current
            .query_row("SELECT name FROM players WHERE id='player-a'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(name, "Ada");
        let safety = inspect_backup(Path::new(&result.safety_backup_path)).unwrap();
        assert_eq!(safety.players[0].name, "Before restore");
    }

    #[test]
    fn verified_safety_snapshot_recovers_a_simulated_interrupted_replacement() {
        let dir = tempfile::tempdir().unwrap();
        let mut current = populated_store(&dir.path().join("current.sqlite3"));
        current
            .execute(
                "UPDATE players SET name='Before restore' WHERE id='player-a'",
                [],
            )
            .unwrap();
        let safety_path = dir.path().join("safety.liferpg-backup");
        create_backup(&current, &safety_path, T0).unwrap();

        // Model a process interrupted after some replacement writes: the live
        // database is no longer the previous coherent world.
        current
            .execute("DELETE FROM quests WHERE id='quest-a'", [])
            .unwrap();
        current
            .execute(
                "UPDATE players SET name='Partially replaced' WHERE id='player-a'",
                [],
            )
            .unwrap();
        restore_safety_snapshot(&mut current, &safety_path).unwrap();

        assert_eq!(
            current
                .query_row::<String, _, _>(
                    "SELECT name FROM players WHERE id='player-a'",
                    [],
                    |row| row.get(0)
                )
                .unwrap(),
            "Before restore"
        );
        assert_eq!(
            current
                .query_row::<i64, _, _>(
                    "SELECT COUNT(*) FROM quests WHERE id='quest-a'",
                    [],
                    |row| row.get(0)
                )
                .unwrap(),
            1
        );
        assert!(check_integrity(&current).unwrap().healthy);
    }

    #[test]
    fn player_rename_does_not_turn_the_same_stable_world_into_a_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let source = populated_store(&dir.path().join("source.sqlite3"));
        let backup_path = dir.path().join("backup.liferpg-backup");
        let info = create_backup(&source, &backup_path, T0).unwrap();
        let mut current = populated_store(&dir.path().join("current.sqlite3"));
        current
            .execute(
                "UPDATE players SET name='Ada (renamed)' WHERE id='player-a'",
                [],
            )
            .unwrap();
        let result = restore_backup(
            &mut current,
            &backup_path,
            &dir.path().join("safety"),
            &info.sha256,
            false,
            T0,
        )
        .unwrap();
        assert_eq!(result.backup.players[0].id, "player-a");
        assert_eq!(
            current
                .query_row::<String, _, _>(
                    "SELECT name FROM players WHERE id='player-a'",
                    [],
                    |row| row.get(0)
                )
                .unwrap(),
            "Ada"
        );
    }

    #[test]
    fn integrity_checker_is_read_only_and_detects_dangling_tag_relationships() {
        let dir = tempfile::tempdir().unwrap();
        let connection = populated_store(&dir.path().join("world.sqlite3"));
        let before: i64 = connection
            .query_row("SELECT COUNT(*) FROM players", [], |row| row.get(0))
            .unwrap();
        let healthy = check_integrity(&connection).unwrap();
        assert!(healthy.healthy);
        let after: i64 = connection
            .query_row("SELECT COUNT(*) FROM players", [], |row| row.get(0))
            .unwrap();
        assert_eq!(before, after);
        connection.execute("INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES('player-b','Grace',1,0,?1,?1)", [T0]).unwrap();
        connection.execute("INSERT INTO tags(id,player_id,transfer_key,name,normalized_name,created_at,updated_at) VALUES('tag-a','player-a','tag-transfer-a','Focus','focus',?1,?1)", [T0]).unwrap();
        connection.execute("INSERT INTO tag_relationships(id,player_id,tag_id,target_kind,target_id,added_at) VALUES('bad-rel','player-a','tag-a','quest','quest-a',?1)", [T0]).unwrap();
        connection
            .execute(
                "UPDATE quests SET player_id='player-b' WHERE id='quest-a'",
                [],
            )
            .unwrap();
        let report = check_integrity(&connection).unwrap();
        assert!(!report.healthy);
        assert_eq!(report.foreign_key_violations, 0);
        assert!(report.cross_player_tag_relationships > 0);
    }

    #[test]
    fn integrity_checker_reports_missing_required_tables_without_repairing_them() {
        let dir = tempfile::tempdir().unwrap();
        let connection = populated_store(&dir.path().join("world.sqlite3"));
        connection
            .execute("DROP TABLE workspace_panels", [])
            .unwrap();
        let report = check_integrity(&connection).unwrap();
        assert!(!report.healthy);
        assert!(report
            .missing_required_tables
            .iter()
            .any(|name| name == "workspace_panels"));
        let still_missing: bool = !table_exists(&connection, "workspace_panels").unwrap();
        assert!(
            still_missing,
            "integrity inspection must not reconstruct missing schema"
        );
    }

    #[test]
    fn old_schema_restore_migrates_staged_copy_without_modifying_backup_archive() {
        let dir = tempfile::tempdir().unwrap();
        let old_path = dir.path().join("old.sqlite3");
        let old = Connection::open(&old_path).unwrap();
        pragma::configure(&old).unwrap();
        migrations::ensure_ledger(&old).unwrap();
        for migration in &migrations::MIGRATIONS[..16] {
            old.execute_batch(migration.sql).unwrap();
            old.execute(
                "INSERT INTO schema_migrations(version,name,applied_at) VALUES(?1,?2,?3)",
                rusqlite::params![migration.version, migration.name, T0],
            )
            .unwrap();
        }
        old.execute("INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES('old-player','Grace',1,0,?1,?1)", [T0]).unwrap();
        let backup = dir.path().join("schema16.liferpg-backup");
        let old_info = create_backup(&old, &backup, T0).unwrap();
        assert_eq!(old_info.compatibility, BackupCompatibility::OlderSchema);
        let original_bytes = fs::read(&backup).unwrap();
        let mut current = populated_store(&dir.path().join("current.sqlite3"));
        restore_backup(
            &mut current,
            &backup,
            &dir.path().join("safety"),
            &old_info.sha256,
            true,
            T0,
        )
        .unwrap();
        let current_schema = migrations::applied_version(&current).unwrap();
        assert_eq!(current_schema, 17);
        let retained: String = current
            .query_row(
                "SELECT name FROM players WHERE id='old-player'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(retained, "Grace");
        assert_eq!(
            fs::read(&backup).unwrap(),
            original_bytes,
            "restore validation must not mutate the backup"
        );
    }

    #[test]
    fn newer_schema_is_reported_as_unsupported_and_never_downgraded() {
        let dir = tempfile::tempdir().unwrap();
        let future = populated_store(&dir.path().join("future.sqlite3"));
        future.execute(
            "INSERT INTO schema_migrations(version,name,applied_at) VALUES(18,'0018_future_migration',?1)",
            [T0],
        ).unwrap();
        let backup_path = dir.path().join("future.liferpg-backup");
        let info = create_backup(&future, &backup_path, T0).unwrap();
        assert_eq!(info.compatibility, BackupCompatibility::NewerUnsupported);
        assert!(matches!(
            inspect_backup(&backup_path).unwrap().compatibility,
            BackupCompatibility::NewerUnsupported
        ));

        let mut current = populated_store(&dir.path().join("current.sqlite3"));
        let before: String = current
            .query_row("SELECT name FROM players WHERE id='player-a'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(matches!(
            restore_backup(
                &mut current,
                &backup_path,
                &dir.path().join("safety"),
                &info.sha256,
                true,
                T0
            ),
            Err(BackupError::UnsupportedSchema {
                found: 18,
                expected: 17
            })
        ));
        let after: String = current
            .query_row("SELECT name FROM players WHERE id='player-a'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            after, before,
            "unsupported backup must not change the current world"
        );
        assert!(!dir.path().join("safety").exists());
    }

    #[test]
    fn failed_staged_migration_preserves_current_world_and_backup_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let mut old = Connection::open(dir.path().join("old.sqlite3")).unwrap();
        pragma::configure(&old).unwrap();
        migrations::ensure_ledger(&old).unwrap();
        for migration in &migrations::MIGRATIONS[..16] {
            let tx = old.transaction().unwrap();
            tx.execute_batch(migration.sql).unwrap();
            tx.execute(
                "INSERT INTO schema_migrations(version,name,applied_at) VALUES(?1,?2,?3)",
                rusqlite::params![migration.version, migration.name, T0],
            )
            .unwrap();
            tx.commit().unwrap();
        }
        old.execute("INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES('old-player','Grace',1,0,?1,?1)", [T0]).unwrap();
        old.execute_batch("CREATE TRIGGER test_abort_next_migration BEFORE INSERT ON schema_migrations WHEN NEW.version=17 BEGIN SELECT RAISE(ABORT,'injected staged migration failure'); END;").unwrap();
        let backup_path = dir.path().join("schema16.liferpg-backup");
        let info = create_backup(&old, &backup_path, T0).unwrap();
        let original_archive = fs::read(&backup_path).unwrap();
        let mut current = populated_store(&dir.path().join("current.sqlite3"));
        current
            .execute(
                "UPDATE players SET name='Current world' WHERE id='player-a'",
                [],
            )
            .unwrap();

        let result = restore_backup(
            &mut current,
            &backup_path,
            &dir.path().join("safety"),
            &info.sha256,
            true,
            T0,
        );
        assert!(matches!(result, Err(BackupError::Migration(_))));
        let current_name: String = current
            .query_row("SELECT name FROM players WHERE id='player-a'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(current_name, "Current world");
        assert_eq!(
            fs::read(&backup_path).unwrap(),
            original_archive,
            "staging never edits the selected archive"
        );
        assert!(
            !dir.path().join("safety").exists(),
            "stage failure happens before the safety checkpoint or live replacement"
        );
    }
}
