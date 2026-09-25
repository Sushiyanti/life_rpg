/**
 * TypeScript mirror of the Rust DTOs in `crates/lr-contracts/src/lib.rs`.
 *
 * This file is the frontend's half of the IPC contract. The Rust side has a
 * test (`contract_drift_matches_the_typescript_mirror`) that pins the exact
 * JSON keys, so if either side is edited alone, that test fails. Keeping both
 * halves honest is why the contract is a first-class module rather than inline
 * types buried in components.
 *
 * No imports from `@tauri-apps/api` here — this module is pure types so it can
 * be unit tested in plain Node.
 */

/** Overall verdict from the core. */
export type HealthStatus = 'ok' | 'degraded' | 'failed';

/** Build/identity facts about the running application. */
export interface ApplicationInfo {
  name: string;
  version: string;
  phase: string;
  runtime: string;
  ipcTransport: string;
  offlineFirst: boolean;
}

/** One migration step in the ledger. */
export interface MigrationRecord {
  version: number;
  name: string;
  applied: boolean;
  appliedAt: string | null;
  state: string;
}

/** Facts about the persistent store. */
export interface DatabaseInfo {
  backend: string;
  locationHint: string | null;
  schemaVersion: number;
  expectedSchemaVersion: number;
  schemaCurrent: boolean;
  migrations: MigrationRecord[];
  journalMode: string;
  foreignKeys: boolean;
}

/** Evidence that a write was committed and read back. */
export interface RoundTrip {
  token: string;
  readBackToken: string;
  matches: boolean;
  rowId: number;
  probeRows: number;
  writtenAt: string;
}

/** The complete status payload. */
export interface HealthReport {
  status: HealthStatus;
  headline: string;
  application: ApplicationInfo;
  database: DatabaseInfo | null;
  roundTrip: RoundTrip | null;
  problems: string[];
  checkedAt: string;
}

/** Error shape for any failed command. */
export interface CommandError {
  code: string;
  message: string;
}

/** Narrow an unknown thrown value into a `CommandError`. */
export function toCommandError(value: unknown): CommandError {
  if (
    typeof value === 'object' &&
    value !== null &&
    'code' in value &&
    'message' in value &&
    typeof (value as CommandError).code === 'string'
  ) {
    return value as CommandError;
  }
  if (value instanceof Error) {
    return { code: 'unknown_error', message: value.message };
  }
  return {
    code: 'unknown_error',
    message: typeof value === 'string' ? value : 'An unknown error occurred.',
  };
}

/** True when the report says the world is fully healthy. */
export function isHealthy(report: HealthReport): boolean {
  return report.status === 'ok';
}

/** Progress of migrations as a 0-1 ratio, for a progress meter. */
export function migrationProgress(report: HealthReport): number {
  const db = report.database;
  if (!db || db.expectedSchemaVersion === 0) return 0;
  return Math.min(1, db.schemaVersion / db.expectedSchemaVersion);
}
