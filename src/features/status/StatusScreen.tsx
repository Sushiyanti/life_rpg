/**
 * `StatusScreen` — the Phase 1 proof-of-life screen.
 *
 * It exists to answer four questions visibly and honestly:
 *
 *   1. Did the application launch?
 *   2. Is SQLite reachable?
 *   3. Did a database read/write actually work?
 *   4. Is the schema at the version this build expects?
 *
 * Every value shown comes from the report. Nothing here is optimistic: if the
 * core reports `failed`, this renders red and prints the raw problem strings
 * rather than a reassuring summary.
 *
 * `client` is an optional injection point (used by tests). Application code
 * renders `<StatusScreen />` and gets the real IPC client.
 */

import type { HealthReport } from '../../domain/health';
import { migrationProgress } from '../../domain/health';
import type { CoreClient } from '../../domain/ipc';
import { useHealthReport } from './useHealthReport';
import { StatusPill } from './StatusPill';
import { InfoCard } from './InfoCard';
import { MigrationLedger } from './MigrationLedger';
import './StatusScreen.css';

interface StatusScreenProps {
  client?: CoreClient;
}

export function StatusScreen({ client }: StatusScreenProps = {}) {
  const { report, error, isLoading, refresh } = useHealthReport(client);

  if (isLoading && !report) {
    return (
      <div className="status status--centered" role="status" aria-live="polite">
        <div className="status__spinner" aria-hidden="true" />
        <p className="status__loading-text">Contacting the world core…</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="status status--centered" role="alert">
        <h2 className="status__error-title">The core did not respond</h2>
        <p className="status__error-code">code: {error.code}</p>
        <p className="status__error-message">{error.message}</p>
        <button type="button" className="status__button" onClick={refresh}>
          Retry
        </button>
      </div>
    );
  }

  if (!report) {
    return null;
  }

  const db = report.database;

  return (
    <div className="status">
      <section className={`status__hero status__hero--${report.status}`}>
        <div className="status__hero-main">
          <StatusPill status={report.status} />
          <h2 className="status__headline">{report.headline}</h2>
          <p className="status__checked">
            Checked at <code>{report.checkedAt}</code>
          </p>
        </div>
        <button
          type="button"
          className="status__button"
          onClick={refresh}
          disabled={isLoading}
        >
          {isLoading ? 'Checking…' : 'Re-run check'}
        </button>
      </section>

      {report.problems.length > 0 && (
        <section className="status__problems" role="alert">
          <h3 className="status__section-title">Problems</h3>
          <ul className="status__problem-list">
            {report.problems.map((problem) => (
              <li key={problem}>{problem}</li>
            ))}
          </ul>
        </section>
      )}

      <section className="status__grid">
        <InfoCard title="Application" accent="cyan">
          <dl className="kv">
            <Row label="Name" value={report.application.name} />
            <Row label="Version" value={report.application.version} />
            <Row label="Build" value={report.application.phase} />
            <Row label="Runtime" value={report.application.runtime} />
            <Row label="UI ⇄ core" value={report.application.ipcTransport} />
            <Row
              label="Offline-first"
              value={report.application.offlineFirst ? 'yes — no network used' : 'no'}
            />
          </dl>
        </InfoCard>

        <InfoCard title="Persistence" accent="gold">
          {db ? (
            <>
              <dl className="kv">
                <Row label="Backend" value={db.backend} />
                <Row
                  label="Store"
                  value={db.locationHint ?? 'in-memory (not persisted to disk)'}
                  mono={Boolean(db.locationHint)}
                />
                <Row label="Journal mode" value={db.journalMode} />
                <Row
                  label="Foreign keys"
                  value={db.foreignKeys ? 'enforced' : 'not enforced'}
                  tone={db.foreignKeys ? 'ok' : 'warn'}
                />
                <Row
                  label="Schema"
                  value={`v${db.schemaVersion} of v${db.expectedSchemaVersion}`}
                  tone={db.schemaCurrent ? 'ok' : 'warn'}
                />
              </dl>
              <div
                className="meter"
                role="progressbar"
                aria-valuenow={db.schemaVersion}
                aria-valuemin={0}
                aria-valuemax={db.expectedSchemaVersion}
                aria-label="Migration progress"
              >
                <div
                  className="meter__fill"
                  style={{ width: `${migrationProgress(report) * 100}%` }}
                />
              </div>
            </>
          ) : (
            <p className="status__absent">
              The store could not be inspected. See the problems above.
            </p>
          )}
        </InfoCard>

        <InfoCard title="Write / read round trip" accent="purple">
          {report.roundTrip ? (
            <RoundTripTable report={report} />
          ) : (
            <p className="status__absent">
              No round trip was performed — storage was not reachable.
            </p>
          )}
        </InfoCard>
      </section>

      {db && <MigrationLedger migrations={db.migrations} />}
    </div>
  );
}

function RoundTripTable({ report }: { report: HealthReport }) {
  const rt = report.roundTrip;
  if (!rt) return null;

  return (
    <>
      <dl className="kv">
        <Row label="Transaction" value="committed" tone={rt.matches ? 'ok' : 'bad'} />
        <Row label="Wrote" value={rt.token} mono />
        <Row label="Read back" value={rt.readBackToken} mono />
        <Row
          label="Match"
          value={rt.matches ? 'identical — persistence verified' : 'MISMATCH'}
          tone={rt.matches ? 'ok' : 'bad'}
        />
        <Row label="Row id" value={String(rt.rowId)} />
        <Row label="Probe rows in store" value={String(rt.probeRows)} />
        <Row label="Recorded at" value={rt.writtenAt} mono />
      </dl>
      <p className="status__footnote">
        This write was executed against the real database and read back inside a
        single transaction. Re-running the check adds another row, so the count
        should climb — that is the evidence the store is genuinely writable.
      </p>
    </>
  );
}

interface RowProps {
  label: string;
  value: string;
  mono?: boolean;
  tone?: 'ok' | 'warn' | 'bad';
}

function Row({ label, value, mono = false, tone }: RowProps) {
  const valueClass = [
    'kv__value',
    mono ? 'kv__value--mono' : '',
    tone ? `kv__value--${tone}` : '',
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <>
      <dt className="kv__label">{label}</dt>
      <dd className={valueClass}>{value}</dd>
    </>
  );
}
