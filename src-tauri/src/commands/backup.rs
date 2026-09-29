//! User-facing backup, restore, and read-only integrity commands.

use std::path::PathBuf;

use lr_contracts::{
    backup::{
        BackupCompatibilityDto, BackupInfoDto, IntegrityReportDto, PlayerDescriptorDto,
        RestoreInfoDto,
    },
    CommandErrorDto,
};
use lr_persistence::backup::{
    BackupCompatibility, BackupError, BackupInfo, IntegrityReport, RestoreInfo,
};
use tauri::State;

use crate::state::AppState;

const MAX_USER_PATH_BYTES: usize = 4096;

fn command_error(code: &str, message: &str) -> CommandErrorDto {
    CommandErrorDto {
        code: code.to_string(),
        message: message.to_string(),
    }
}

fn backup_error(error: BackupError) -> CommandErrorDto {
    match error {
        BackupError::UnsupportedFormat => command_error("backup_unsupported_format", "This backup format is newer than this Life RPG version supports."),
        BackupError::UnsupportedSchema { .. } => command_error("backup_unsupported_schema", "This backup was created by a newer database schema. Update Life RPG before restoring it."),
        BackupError::ChecksumMismatch => command_error("backup_checksum_mismatch", "The backup checksum did not match. The selected file was not changed."),
        BackupError::WorldMismatch => command_error("backup_world_confirmation_required", "This backup belongs to a different world. Review its Player names and explicitly confirm the replacement."),
        BackupError::DestinationExists => command_error("backup_destination_exists", "That file already exists. Choose a new destination; existing files are never overwritten."),
        BackupError::TooLarge => command_error("backup_too_large", "This backup exceeds the supported 512 MiB size limit."),
        BackupError::NotFileBacked => command_error("storage_not_file_backed", "The current world is not available as a persistent file. No backup or restore was attempted."),
        BackupError::SafetyBackup => command_error("backup_safety_failed", "A verified safety backup could not be created, so the restore was not applied."),
        BackupError::ReplacementFailed => command_error("backup_restore_rolled_back", "The restore did not complete. The current world was restored from its verified safety backup; no restore was reported as successful."),
        BackupError::RollbackFailed => command_error("backup_restore_recovery_required", "The restore and automatic recovery both failed. Editing is paused; use the retained verified safety backup before continuing."),
        BackupError::Integrity | BackupError::Malformed | BackupError::InvalidMigrationLedger => command_error("backup_integrity_failed", "The selected file did not pass Life RPG backup, schema, and SQLite integrity checks. It was not restored or modified."),
        BackupError::Io(_) | BackupError::Json(_) | BackupError::Sqlite(_) | BackupError::Migration(_) => command_error("backup_operation_failed", "The backup operation failed. The current world was not intentionally replaced; check storage availability and try again."),
    }
}

fn storage_error(error: lr_application::StorageError) -> CommandErrorDto {
    match error {
        lr_application::StorageError::Unreachable(message) if message.contains("LR-RESTORE-01") => command_error("backup_restore_recovery_required", "Editing is paused because restore recovery could not be verified. Use the retained verified safety backup before continuing."),
        lr_application::StorageError::Unreachable(_) => command_error("storage_unreachable", "World storage is not available. The existing world file was left unchanged."),
        lr_application::StorageError::Schema(_) => command_error("storage_schema_problem", "The current world schema cannot be used by this application version."),
        lr_application::StorageError::Operation(message) if message.contains("different world") => command_error("backup_world_confirmation_required", "This backup belongs to a different world. Review its Player names and explicitly confirm the replacement."),
        lr_application::StorageError::Operation(message) if message.contains("safety backup") => command_error("backup_safety_failed", "A verified safety backup could not be created, so the restore was not applied."),
        lr_application::StorageError::Operation(message) if message.contains("restore failed; the live world was restored") => command_error("backup_restore_rolled_back", "The restore did not complete. The current world was restored from its verified safety backup; no restore was reported as successful."),
        lr_application::StorageError::Operation(message) if message.contains("restore and rollback failed") => command_error("backup_restore_recovery_required", "The restore and automatic recovery both failed. Editing is paused; use the retained verified safety backup before continuing."),
        lr_application::StorageError::Operation(message) if message.contains("newer schema") => command_error("backup_unsupported_schema", "This backup uses a newer database schema. Update Life RPG before restoring it."),
        lr_application::StorageError::Operation(_) => command_error("backup_operation_failed", "The backup operation failed. The current world was not intentionally replaced; check storage availability and try again."),
    }
}

fn dto(info: BackupInfo) -> BackupInfoDto {
    BackupInfoDto {
        product: info.product,
        application_version: info.application_version,
        format_version: info.format_version,
        schema_version: info.schema_version,
        created_at: info.created_at,
        players: info
            .players
            .into_iter()
            .map(|player| PlayerDescriptorDto {
                id: player.id,
                name: player.name,
            })
            .collect(),
        database_bytes: info.database_bytes,
        sha256: info.sha256,
        compatibility: match info.compatibility {
            BackupCompatibility::SameSchema => BackupCompatibilityDto::SameSchema,
            BackupCompatibility::OlderSchema => BackupCompatibilityDto::OlderSchema,
            BackupCompatibility::NewerUnsupported => BackupCompatibilityDto::NewerUnsupported,
        },
        integrity: integrity_dto(info.integrity),
    }
}

fn integrity_dto(report: IntegrityReport) -> IntegrityReportDto {
    IntegrityReportDto {
        healthy: report.healthy,
        schema_version: report.schema_version,
        integrity_check_ok: report.integrity_check_ok,
        foreign_key_violations: report.foreign_key_violations,
        missing_required_tables: report.missing_required_tables,
        malformed_configuration_rows: report.malformed_configuration_rows,
        cross_player_tag_relationships: report.cross_player_tag_relationships,
    }
}

/// Create a checksummed SQLite online-backup snapshot at a player-selected path.
#[tauri::command]
pub fn create_world_backup(
    state: State<'_, AppState>,
    destination: String,
) -> Result<BackupInfoDto, CommandErrorDto> {
    if destination.trim().is_empty() || destination.len() > MAX_USER_PATH_BYTES {
        return Err(command_error(
            "backup_invalid_destination",
            "Choose a valid backup file location.",
        ));
    }
    if state.health.store().location().is_none() {
        return Err(command_error(
            "storage_not_file_backed",
            "The current world is not available as a persistent file. No backup was created.",
        ));
    }
    state
        .health
        .store()
        .create_backup(
            &PathBuf::from(destination),
            &chrono::Utc::now().to_rfc3339(),
        )
        .map(dto)
        .map_err(storage_error)
}

/// Inspect a selected backup without modifying the source file or live world.
#[tauri::command]
pub fn inspect_world_backup(path: String) -> Result<BackupInfoDto, CommandErrorDto> {
    if path.trim().is_empty() || path.len() > MAX_USER_PATH_BYTES {
        return Err(command_error(
            "backup_invalid_path",
            "Choose a valid Life RPG backup file.",
        ));
    }
    lr_persistence::SqliteHealthStore::inspect_backup(&PathBuf::from(path))
        .map(dto)
        .map_err(backup_error)
}

/// Restore a previously inspected archive only if its exact digest still matches.
#[tauri::command]
pub fn restore_world_backup(
    state: State<'_, AppState>,
    backup_path: String,
    expected_sha256: String,
    confirm_different_world: bool,
) -> Result<RestoreInfoDto, CommandErrorDto> {
    if backup_path.trim().is_empty()
        || backup_path.len() > MAX_USER_PATH_BYTES
        || expected_sha256.len() != 64
    {
        return Err(command_error(
            "backup_invalid_request",
            "The selected backup request is invalid. Inspect the file again and retry.",
        ));
    }
    let store = if state.health.store().location().is_none() {
        state
            .recovery_store()
            .unwrap_or_else(|| state.health.store().clone())
    } else {
        state.health.store().clone()
    };
    let current_path = store.location().ok_or_else(|| {
        command_error(
            "storage_not_file_backed",
            "The current world is not available as a persistent file. No restore was attempted.",
        )
    })?;
    let safety_directory = PathBuf::from(current_path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("backups");
    let restored = store
        .restore_backup(
            &PathBuf::from(backup_path),
            &safety_directory,
            &expected_sha256,
            confirm_different_world,
            &chrono::Utc::now().to_rfc3339(),
        )
        .map_err(storage_error)?;
    Ok(restore_dto(restored))
}

/// Run an integrity check over the current database without writing rows or pragmas.
#[tauri::command]
pub fn check_world_integrity(
    state: State<'_, AppState>,
) -> Result<IntegrityReportDto, CommandErrorDto> {
    let store = if state.health.store().location().is_none() {
        state
            .recovery_store()
            .unwrap_or_else(|| state.health.store().clone())
    } else {
        state.health.store().clone()
    };
    store
        .check_integrity()
        .map(integrity_dto)
        .map_err(storage_error)
}

fn restore_dto(info: RestoreInfo) -> RestoreInfoDto {
    RestoreInfoDto {
        backup: dto(info.backup),
        safety_backup_path: info.safety_backup_path,
        restored_schema_version: info.restored_schema_version,
    }
}
