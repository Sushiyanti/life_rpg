/**
 * Component tests for the status screen.
 *
 * These assert behaviour the user actually sees: the verdict, the round-trip
 * evidence, the migration ledger, and — most importantly — that a failing core
 * produces a visible error rather than a blank screen.
 */

import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { CoreClient, type InvokeTransport } from '../domain/ipc';
import { StatusScreen } from '../features/status/StatusScreen';
import { failedReport, healthyReport } from './fixtures';

function clientReturning(report: unknown, times = Number.POSITIVE_INFINITY): CoreClient {
  const calls = { count: 0 };
  const transport: InvokeTransport = async () => {
    calls.count += 1;
    if (calls.count > times) throw new Error('exhausted');
    return report as never;
  };
  return new CoreClient(transport);
}

describe('StatusScreen', () => {
  it('shows a loading state before the core answers', () => {
    const transport: InvokeTransport = () => new Promise(() => {});
    render(<StatusScreen client={new CoreClient(transport)} />);
    expect(screen.getByText(/contacting the world core/i)).toBeInTheDocument();
  });

  it('renders the verdict, schema state and round-trip proof for a healthy world', async () => {
    render(<StatusScreen client={clientReturning(healthyReport())} />);

    expect(await screen.findByText(/operational/i)).toBeInTheDocument();
    expect(screen.getByText(/all systems nominal/i)).toBeInTheDocument();

    // Persistence card
    expect(screen.getByText('sqlite')).toBeInTheDocument();
    expect(screen.getByText('wal')).toBeInTheDocument();
    expect(screen.getByText('v3 of v3')).toBeInTheDocument();
    expect(screen.getByText('enforced')).toBeInTheDocument();

    // Round-trip card — the actual proof of a database write + read
    expect(screen.getByText('committed')).toBeInTheDocument();
    expect(screen.getAllByText('probe-deadbeef-0').length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText(/identical — persistence verified/i)).toBeInTheDocument();
    expect(screen.getByText(/probe rows in store/i)).toBeInTheDocument();
    // The probe row count is rendered next to its label (rowId 1, probeRows 1).
    const probeRowsLabel = screen.getByText(/probe rows in store/i);
    expect(probeRowsLabel.nextElementSibling?.textContent).toBe('1');
  });

  it('renders the migration ledger, including a pending row', async () => {
    const report = healthyReport();
    report.database!.migrations.push({
      version: 4,
      name: '0004_future_step',
      applied: false,
      appliedAt: null,
      state: 'pending',
    });
    report.database!.expectedSchemaVersion = 4;

    render(<StatusScreen client={clientReturning(report)} />);

    expect(await screen.findByText('0001_core_ledger')).toBeInTheDocument();
    expect(screen.getByText('0004_future_step')).toBeInTheDocument();
    expect(screen.getByText('pending')).toBeInTheDocument();
    expect(screen.getAllByText('applied').length).toBe(3);
  });

  it('surfaces problems and a failed verdict instead of a reassuring summary', async () => {
    render(<StatusScreen client={clientReturning(failedReport())} />);

    expect(await screen.findByText(/^failed$/i)).toBeInTheDocument();
    expect(screen.getByText(/world storage is unavailable/i)).toBeInTheDocument();
    expect(screen.getByText(/^problems$/i)).toBeInTheDocument();
    // All three probe operations fail, so the raw cause appears once per failure.
    expect(screen.getAllByText(/disk gone/i).length).toBe(3);
    expect(
      screen.getByText(/the store could not be inspected/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/no round trip was performed/i),
    ).toBeInTheDocument();
  });

  it('shows a visible error with the code when the core does not respond', async () => {
    const transport: InvokeTransport = async () => {
      throw { code: 'storage_unreachable', message: 'the core is not listening' };
    };
    render(<StatusScreen client={new CoreClient(transport)} />);

    expect(await screen.findByRole('alert')).toBeInTheDocument();
    expect(screen.getByText(/the core did not respond/i)).toBeInTheDocument();
    expect(screen.getByText(/storage_unreachable/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();
  });

  it('re-runs the check when the user clicks the button', async () => {
    const user = userEvent.setup();
    const getStatus = vi.fn(async () => healthyReport());
    const client = new CoreClient(getStatus as unknown as InvokeTransport);

    render(<StatusScreen client={client} />);
    await screen.findByText(/operational/i);

    await user.click(screen.getByRole('button', { name: /re-run check/i }));

    await waitFor(() => expect(getStatus).toHaveBeenCalledTimes(2));
  });

  it('labels an in-memory store honestly rather than implying persistence', async () => {
    const report = healthyReport();
    report.database!.locationHint = null;
    render(<StatusScreen client={clientReturning(report)} />);

    expect(
      await screen.findByText(/in-memory \(not persisted to disk\)/i),
    ).toBeInTheDocument();
  });
});
