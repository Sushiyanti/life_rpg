/** IPC mirror of `lr-contracts::backup`; keys and enum values are pinned by tests. */
export type BackupCompatibility = 'sameSchema' | 'olderSchema' | 'newerUnsupported';

export interface BackupPlayerDescriptor {
  id: string;
  name: string;
}

export interface IntegrityReport {
  healthy: boolean;
  schemaVersion: number;
  integrityCheckOk: boolean;
  foreignKeyViolations: number;
  missingRequiredTables: string[];
  malformedConfigurationRows: number;
  crossPlayerTagRelationships: number;
}

export interface BackupInfo {
  product: string;
  applicationVersion: string;
  formatVersion: number;
  schemaVersion: number;
  createdAt: string;
  players: BackupPlayerDescriptor[];
  databaseBytes: number;
  sha256: string;
  compatibility: BackupCompatibility;
  integrity: IntegrityReport;
}

export interface RestoreInfo {
  backup: BackupInfo;
  safetyBackupPath: string;
  restoredSchemaVersion: number;
}
