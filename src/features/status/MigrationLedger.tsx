/**
 * `MigrationLedger` — renders the schema migration history.
 *
 * This is the visible payoff of keeping a ledger table instead of a single
 * `user_version` integer: the screen can show *which* migrations exist, whether
 * each is applied, and when it ran. History made visible, which is the whole
 * premise of the application.
 */

import type { MigrationRecord } from '../../domain/health';

interface MigrationLedgerProps {
  migrations: MigrationRecord[];
}

export function MigrationLedger({ migrations }: MigrationLedgerProps) {
  if (migrations.length === 0) {
    return null;
  }

  return (
    <section className="ledger">
      <h3 className="status__section-title">Schema migration ledger</h3>
      <table className="ledger__table">
        <thead>
          <tr>
            <th scope="col">v</th>
            <th scope="col">Name</th>
            <th scope="col">State</th>
            <th scope="col">Applied at</th>
          </tr>
        </thead>
        <tbody>
          {migrations.map((migration) => (
            <tr key={migration.version} data-state={migration.state}>
              <td className="ledger__version">{migration.version}</td>
              <td className="ledger__name">{migration.name}</td>
              <td>
                <span className={`ledger__state ledger__state--${migration.state}`}>
                  {migration.state}
                </span>
              </td>
              <td className="ledger__applied">{migration.appliedAt ?? '—'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}
