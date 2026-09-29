//! DTOs for backup inspection, integrity checks, and world restoration.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDescriptorDto {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackupCompatibilityDto {
    SameSchema,
    OlderSchema,
    NewerUnsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReportDto {
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
pub struct BackupInfoDto {
    pub product: String,
    pub application_version: String,
    pub format_version: u16,
    pub schema_version: u32,
    pub created_at: String,
    pub players: Vec<PlayerDescriptorDto>,
    pub database_bytes: u64,
    pub sha256: String,
    pub compatibility: BackupCompatibilityDto,
    pub integrity: IntegrityReportDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreInfoDto {
    pub backup: BackupInfoDto,
    pub safety_backup_path: String,
    pub restored_schema_version: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_wire_shape_uses_the_typescript_camel_case_contract() {
        let dto = BackupInfoDto {
            product: "Life RPG".into(),
            application_version: "0.1.0".into(),
            format_version: 1,
            schema_version: 17,
            created_at: "2026-09-29T00:00:00+00:00".into(),
            players: vec![PlayerDescriptorDto {
                id: "player-a".into(),
                name: "Ada".into(),
            }],
            database_bytes: 123,
            sha256: "a".repeat(64),
            compatibility: BackupCompatibilityDto::NewerUnsupported,
            integrity: IntegrityReportDto {
                healthy: false,
                schema_version: 17,
                integrity_check_ok: true,
                foreign_key_violations: 0,
                missing_required_tables: vec![],
                malformed_configuration_rows: 0,
                cross_player_tag_relationships: 0,
            },
        };
        let value = serde_json::to_value(dto).unwrap();
        let root = value.as_object().unwrap();
        for key in [
            "product",
            "applicationVersion",
            "formatVersion",
            "schemaVersion",
            "createdAt",
            "players",
            "databaseBytes",
            "sha256",
            "compatibility",
            "integrity",
        ] {
            assert!(root.contains_key(key), "missing `{key}`");
        }
        assert_eq!(value["compatibility"], "newerUnsupported");
        assert_eq!(value["players"][0]["id"], "player-a");
        for key in [
            "integrityCheckOk",
            "foreignKeyViolations",
            "missingRequiredTables",
            "malformedConfigurationRows",
            "crossPlayerTagRelationships",
        ] {
            assert!(
                value["integrity"].get(key).is_some(),
                "integrity missing `{key}`"
            );
        }
        assert!(value.get("application_version").is_none());
    }
}
