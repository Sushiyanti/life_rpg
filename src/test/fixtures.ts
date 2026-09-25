/**
 * Test fixtures mirroring the wire contract.
 *
 * These are typed as the real `HealthReport`, so if the contract changes the
 * fixtures stop compiling — which is exactly the alarm we want.
 */

import type { HealthReport } from '../domain/health';

const T0 = '2026-09-25T00:00:00+00:00';

export function healthyReport(overrides: Partial<HealthReport> = {}): HealthReport {
  const base: HealthReport = {
    status: 'ok',
    headline: 'All systems nominal — SQLite schema v3 verified, write/read round trip committed',
    application: {
      name: 'Life RPG',
      version: '0.1.0',
      phase: 'phase-1 foundation',
      runtime: 'Tauri v2 desktop shell',
      ipcTransport: 'tauri::command (in-process IPC, no HTTP)',
      offlineFirst: true,
    },
    database: {
      backend: 'sqlite',
      locationHint: '/home/user/.local/share/com.liferpg.desktop/life-rpg.sqlite3',
      schemaVersion: 3,
      expectedSchemaVersion: 3,
      schemaCurrent: true,
      migrations: [
        {
          version: 1,
          name: '0001_core_ledger',
          applied: true,
          appliedAt: T0,
          state: 'applied',
        },
        {
          version: 2,
          name: '0002_health_probe',
          applied: true,
          appliedAt: T0,
          state: 'applied',
        },
        {
          version: 3,
          name: '0003_type_definition_registry',
          applied: true,
          appliedAt: T0,
          state: 'applied',
        },
      ],
      journalMode: 'wal',
      foreignKeys: true,
    },
    roundTrip: {
      token: 'probe-deadbeef-0',
      readBackToken: 'probe-deadbeef-0',
      matches: true,
      rowId: 1,
      probeRows: 1,
      writtenAt: T0,
    },
    problems: [],
    checkedAt: T0,
  };

  return { ...base, ...overrides };
}

export function failedReport(): HealthReport {
  return {
    ...healthyReport(),
    status: 'failed',
    headline: 'World storage is unavailable — the application cannot persist anything',
    database: null,
    roundTrip: null,
    problems: [
      'Storage diagnostics unavailable — store is not reachable: disk gone',
      'Schema inspection failed — store is not reachable: disk gone',
      'Write/read round trip failed — store is not reachable: disk gone',
    ],
  };
}
