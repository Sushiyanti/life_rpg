/**
 * Status verdict badge.
 *
 * Presentation-only: it maps a verdict to a token-driven appearance. It is also
 * the first component written against the Phase 4 presentation vocabulary — it
 * takes an `emphasis`-like verdict and resolves it to a token rather than
 * branching on colors inline.
 */

import type { HealthStatus } from '../../domain/health';

const LABEL: Record<HealthStatus, string> = {
  ok: 'Operational',
  degraded: 'Degraded',
  failed: 'Failed',
};

const SIGIL: Record<HealthStatus, string> = {
  ok: '●',
  degraded: '▲',
  failed: '✕',
};

interface StatusPillProps {
  status: HealthStatus;
}

export function StatusPill({ status }: StatusPillProps) {
  return (
    <span className={`pill pill--${status}`} data-status={status}>
      <span className="pill__sigil" aria-hidden="true">
        {SIGIL[status]}
      </span>
      <span className="pill__label">{LABEL[status]}</span>
    </span>
  );
}
