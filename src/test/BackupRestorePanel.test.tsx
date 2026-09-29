import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { BackupInfo, RestoreInfo } from '../domain/backup';
import type { CoreClient } from '../domain/ipc';

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
  save: vi.fn(),
}));

import { open } from '@tauri-apps/plugin-dialog';
import { BackupRestorePanel } from '../features/status/BackupRestorePanel';

const backup: BackupInfo = {
  product: 'Life RPG',
  applicationVersion: '0.1.0',
  formatVersion: 1,
  schemaVersion: 17,
  createdAt: '2026-09-29T00:00:00+00:00',
  players: [{ id: 'backup-player', name: 'Backup World' }],
  databaseBytes: 4096,
  sha256: 'a'.repeat(64),
  compatibility: 'sameSchema',
  integrity: {
    healthy: true,
    schemaVersion: 17,
    integrityCheckOk: true,
    foreignKeyViolations: 0,
    missingRequiredTables: [],
    malformedConfigurationRows: 0,
    crossPlayerTagRelationships: 0,
  },
};

const restored: RestoreInfo = {
  backup,
  safetyBackupPath: '/private/app-data/backups/pre-restore.liferpg-backup',
  restoredSchemaVersion: 17,
};

beforeEach(() => vi.clearAllMocks());

describe('BackupRestorePanel', () => {
  it('inspects without mutating and requires typed confirmation before replacing a different world', async () => {
    const user = userEvent.setup();
    vi.mocked(open).mockResolvedValue('/selected/world.liferpg-backup');
    const inspectWorldBackup = vi.fn(async () => backup);
    const restoreWorldBackup = vi.fn(async () => restored);
    const onRestored = vi.fn();
    const client = { inspectWorldBackup, restoreWorldBackup } as unknown as CoreClient;

    render(<BackupRestorePanel client={client} players={[{ id: 'current-player', name: 'Current World' }]} onRestored={onRestored} />);
    await user.click(screen.getByRole('button', { name: /inspect \/ restore backup/i }));

    expect(await screen.findByRole('heading', { name: /verified backup preview/i })).toBeInTheDocument();
    expect(inspectWorldBackup).toHaveBeenCalledWith('/selected/world.liferpg-backup');
    expect(restoreWorldBackup).not.toHaveBeenCalled();
    expect(screen.getByText(/different world identity/i)).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /review restore consequences/i }));
    const confirm = screen.getByRole('button', { name: /confirm restore/i });
    expect(confirm).toBeDisabled();
    await user.type(screen.getByLabelText(/type RESTORE to confirm replacement/i), 'RESTORE');
    expect(confirm).toBeEnabled();

    await user.click(confirm);
    await waitFor(() => expect(restoreWorldBackup).toHaveBeenCalledWith(
      '/selected/world.liferpg-backup', 'a'.repeat(64), true,
    ));
    expect(onRestored).toHaveBeenCalledWith(restored);
  });

  it('requires typed confirmation when the existing world identity cannot be inspected', async () => {
    const user = userEvent.setup();
    vi.mocked(open).mockResolvedValue('/selected/world.liferpg-backup');
    const inspectWorldBackup = vi.fn(async () => backup);
    const restoreWorldBackup = vi.fn(async () => restored);
    const client = { inspectWorldBackup, restoreWorldBackup } as unknown as CoreClient;

    render(<BackupRestorePanel client={client} players={[]} currentIdentityKnown={false} onRestored={vi.fn()} />);
    await user.click(screen.getByRole('button', { name: /inspect \/ restore backup/i }));
    expect(await screen.findByText(/current world identity unavailable/i)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /review restore consequences/i }));
    const confirm = screen.getByRole('button', { name: /confirm restore/i });
    expect(confirm).toBeDisabled();
    await user.type(screen.getByLabelText(/type RESTORE to confirm replacement/i), 'RESTORE');
    expect(confirm).toBeEnabled();

    await user.click(confirm);
    await waitFor(() => expect(restoreWorldBackup).toHaveBeenCalledWith(
      '/selected/world.liferpg-backup', 'a'.repeat(64), true,
    ));
  });

  it('blocks a backup newer than the application from entering the restore flow', async () => {
    const user = userEvent.setup();
    vi.mocked(open).mockResolvedValue('/selected/newer.liferpg-backup');
    const inspectWorldBackup = vi.fn(async () => ({ ...backup, schemaVersion: 18, compatibility: 'newerUnsupported' as const }));
    const restoreWorldBackup = vi.fn();
    const client = { inspectWorldBackup, restoreWorldBackup } as unknown as CoreClient;
    render(<BackupRestorePanel client={client} players={[{ id: 'backup-player', name: 'Backup World' }]} onRestored={vi.fn()} />);

    await user.click(screen.getByRole('button', { name: /inspect \/ restore backup/i }));
    expect(await screen.findByText(/newer app required/i)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /review restore consequences/i })).not.toBeInTheDocument();
    expect(restoreWorldBackup).not.toHaveBeenCalled();
  });
});
