import { useMemo, useState } from 'react';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { BackupInfo, IntegrityReport, RestoreInfo } from '../../domain/backup';
import type { CoreClient } from '../../domain/ipc';

interface BackupRestorePanelProps {
  client: CoreClient;
  players: { id: string; name: string }[];
  currentIdentityKnown?: boolean;
  onRestored: (result: RestoreInfo) => void;
}

const BACKUP_FILTER = [{ name: 'Life RPG world backup', extensions: ['liferpg-backup'] }];

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : 'The backup operation could not be completed.';
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
}

export function BackupRestorePanel({ client, players, currentIdentityKnown = true, onRestored }: BackupRestorePanelProps) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [savedPath, setSavedPath] = useState('');
  const [sourcePath, setSourcePath] = useState('');
  const [preview, setPreview] = useState<BackupInfo | null>(null);
  const [confirmationOpen, setConfirmationOpen] = useState(false);
  const [typedConfirmation, setTypedConfirmation] = useState('');
  const [integrity, setIntegrity] = useState<IntegrityReport | null>(null);

  const sameWorld = useMemo(() => {
    if (!preview || !currentIdentityKnown) return false;
    const current = players.map((player) => player.id).sort();
    const backup = preview.players.map((player) => player.id).sort();
    return current.length === backup.length && current.every((id, index) => id === backup[index]);
  }, [currentIdentityKnown, players, preview]);
  const needsTypedConfirmation = !currentIdentityKnown || !sameWorld;
  const restoreSupported = preview?.compatibility !== 'newerUnsupported';
  const canConfirmRestore = Boolean(
    confirmationOpen && preview && restoreSupported && !busy && (!needsTypedConfirmation || typedConfirmation === 'RESTORE'),
  );

  const createBackup = async () => {
    setBusy(true); setError(''); setNotice(''); setSavedPath('');
    try {
      const path = await save({
        title: 'Save a Life RPG world backup',
        defaultPath: 'life-rpg-backup.liferpg-backup',
        filters: BACKUP_FILTER,
      });
      if (!path || typeof path !== 'string') return;
      const result = await client.createWorldBackup(path);
      setSavedPath(path);
      setNotice(`Backup created and verified (schema v${result.schemaVersion}, ${formatBytes(result.databaseBytes)}).`);
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setBusy(false);
    }
  };

  const chooseBackup = async () => {
    setBusy(true); setError(''); setNotice(''); setPreview(null); setSourcePath(''); setConfirmationOpen(false); setTypedConfirmation('');
    try {
      const path = await open({
        title: 'Inspect a Life RPG backup',
        multiple: false,
        directory: false,
        filters: BACKUP_FILTER,
      });
      if (!path || typeof path !== 'string') return;
      const result = await client.inspectWorldBackup(path);
      setSourcePath(path);
      setPreview(result);
      setNotice('Backup integrity and checksum verified. The file has not been modified.');
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setBusy(false);
    }
  };

  const runIntegrity = async () => {
    setBusy(true); setError(''); setNotice(''); setIntegrity(null);
    try {
      const report = await client.checkWorldIntegrity();
      setIntegrity(report);
      setNotice(report.healthy ? 'Current world passed its read-only integrity check.' : 'Integrity issues were found; no data was changed.');
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setBusy(false);
    }
  };

  const restore = async () => {
    if (!preview || !sourcePath || !canConfirmRestore) return;
    setBusy(true); setError(''); setNotice('');
    try {
      const result = await client.restoreWorldBackup(sourcePath, preview.sha256, !sameWorld);
      setNotice('Restore completed. The previous world was saved to a verified safety backup. Reload before continuing.');
      setConfirmationOpen(false);
      onRestored(result);
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="backup-panel" aria-labelledby="backup-panel-title">
      <div className="backup-panel__header">
        <div>
          <p className="backup-panel__eyebrow">LOCAL DATA SAFETY</p>
          <h2 id="backup-panel-title">Backups &amp; recovery</h2>
          <p className="backup-panel__intro">
            Create a consistent snapshot of the open world, verify a backup without changing it, or restore a previously validated world.
          </p>
        </div>
        <span className="backup-panel__badge">Offline · local files</span>
      </div>

      <div className="backup-panel__actions">
        <button type="button" className="status__button" onClick={() => void createBackup()} disabled={busy}>
          {busy ? 'Working…' : 'Create verified backup'}
        </button>
        <button type="button" className="status__button" onClick={() => void chooseBackup()} disabled={busy}>
          Inspect / restore backup
        </button>
        <button type="button" className="status__button" onClick={() => void runIntegrity()} disabled={busy}>
          Check current world
        </button>
      </div>

      <p className="backup-panel__footnote">
        Backups use SQLite’s online snapshot API, include a SHA-256 checksum and schema version, and do not include the source machine’s database path. The current 512 MiB size limit is enforced before restore.
      </p>

      {savedPath && <p className="backup-panel__path"><strong>Saved to:</strong> <code>{savedPath}</code></p>}
      {notice && <p className="backup-panel__notice" role="status" aria-live="polite">{notice}</p>}
      {error && <p className="backup-panel__error" role="alert">{error}</p>}

      {preview && (
        <div className="backup-preview" aria-labelledby="backup-preview-title">
          <h3 id="backup-preview-title">Verified backup preview</h3>
          <dl className="kv">
            <dt className="kv__label">Created</dt><dd className="kv__value">{preview.createdAt}</dd>
            <dt className="kv__label">App / schema</dt><dd className="kv__value">{preview.applicationVersion} / v{preview.schemaVersion}</dd>
            <dt className="kv__label">Archive size</dt><dd className="kv__value">{formatBytes(preview.databaseBytes)}</dd>
            <dt className="kv__label">SHA-256</dt><dd className="kv__value kv__value--mono">{preview.sha256}</dd>
            <dt className="kv__label">Compatibility</dt><dd className={`kv__value ${restoreSupported ? 'kv__value--ok' : 'kv__value--bad'}`}>
              {preview.compatibility === 'sameSchema' ? 'Supported — same schema' : preview.compatibility === 'olderSchema' ? 'Supported — will be upgraded in staging' : 'Unsupported — newer app required'}
            </dd>
            <dt className="kv__label">Players in backup</dt><dd className="kv__value">{preview.players.length ? preview.players.map((player) => player.name).join(', ') : 'No Players yet'}</dd>
              <dt className="kv__label">World match</dt><dd className="kv__value">{!currentIdentityKnown ? 'Current world identity unavailable' : sameWorld ? 'Matches this world' : 'Different world identity'}</dd>
            <dt className="kv__label">Integrity</dt><dd className={`kv__value ${preview.integrity.healthy ? 'kv__value--ok' : 'kv__value--bad'}`}>
              {preview.integrity.healthy ? 'Passed' : 'Failed'} · {preview.integrity.foreignKeyViolations} foreign-key issue(s)
            </dd>
          </dl>

          {restoreSupported && (
            <div className="backup-panel__actions">
              {!confirmationOpen ? (
                <button type="button" className="status__button status__button--danger" disabled={busy} onClick={() => setConfirmationOpen(true)}>
                  Review restore consequences
                </button>
              ) : (
                <div className="backup-confirm" role="group" aria-labelledby="backup-confirm-title">
                  <h4 id="backup-confirm-title">Replace the current world?</h4>
                  <p>The selected snapshot replaces all records in this local world. The app will first save and verify a safety backup of the current database. Older supported schemas are upgraded in a separate staging file before replacement. The selected backup itself is never changed.</p>
                  {needsTypedConfirmation && (
                    <label className="backup-confirm__field">
                      {currentIdentityKnown ? 'This is a different world.' : 'The current world identity could not be verified.'} Type <code>RESTORE</code> to confirm replacement.
                      <input value={typedConfirmation} onChange={(event) => setTypedConfirmation(event.target.value)} autoComplete="off" spellCheck={false} />
                    </label>
                  )}
                  <div className="backup-panel__actions">
                    <button type="button" className="status__button status__button--danger" disabled={!canConfirmRestore} onClick={() => void restore()}>
                      {busy ? 'Restoring…' : 'Confirm restore'}
                    </button>
                    <button type="button" className="status__button" disabled={busy} onClick={() => { setConfirmationOpen(false); setTypedConfirmation(''); }}>
                      Cancel
                    </button>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {integrity && (
        <div className={`backup-integrity ${integrity.healthy ? 'backup-integrity--ok' : 'backup-integrity--bad'}`} role="status" aria-live="polite">
          <h3>Read-only current-world check: {integrity.healthy ? 'passed' : 'issues found'}</h3>
          <p>Schema v{integrity.schemaVersion}; SQLite: {integrity.integrityCheckOk ? 'ok' : 'failed'}; foreign-key violations: {integrity.foreignKeyViolations}; malformed configuration rows: {integrity.malformedConfigurationRows}; cross-Player Tag links: {integrity.crossPlayerTagRelationships}.</p>
          {integrity.missingRequiredTables.length > 0 && <p>Missing required tables: {integrity.missingRequiredTables.join(', ')}.</p>}
          <p>This check did not repair, migrate, or write to the current world.</p>
        </div>
      )}
    </section>
  );
}
