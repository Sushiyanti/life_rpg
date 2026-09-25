/**
 * `useHealthReport` — the use-case hook behind the status screen.
 *
 * This is the frontend's equivalent of a command handler: it owns loading/error
 * state, calls exactly one client method, and hands plain data to the present-
 * ational component. The screen never talks to `invoke`.
 *
 * `client` is injectable so tests can pass a stub without module mocking.
 */

import { useCallback, useEffect, useState } from 'react';

import type { HealthReport } from '../../domain/health';
import { coreClient, type CoreClient } from '../../domain/ipc';

export interface HealthReportState {
  report: HealthReport | null;
  error: { code: string; message: string } | null;
  isLoading: boolean;
  /** Re-run the check (used by the Refresh button). */
  refresh: () => void;
}

export function useHealthReport(client: CoreClient = coreClient): HealthReportState {
  const [report, setReport] = useState<HealthReport | null>(null);
  const [error, setError] = useState<{ code: string; message: string } | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [nonce, setNonce] = useState(0);

  const refresh = useCallback(() => {
    setNonce((n) => n + 1);
  }, []);

  useEffect(() => {
    // Guard against setting state after unmount: the check opens SQLite and can
    // outlive a fast dev-mode hot reload.
    let cancelled = false;
    setIsLoading(true);

    client
      .getStatus()
      .then((result) => {
        if (cancelled) return;
        setReport(result);
        setError(null);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        const code =
          typeof err === 'object' && err !== null && 'code' in err
            ? String((err as { code: unknown }).code)
            : 'unknown_error';
        const message = err instanceof Error ? err.message : String(err);
        setError({ code, message });
        setReport(null);
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [client, nonce]);

  return { report, error, isLoading, refresh };
}
