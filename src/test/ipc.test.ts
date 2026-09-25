/**
 * Tests for the typed IPC layer.
 *
 * The point of these tests is the *boundary*, not the components: command names
 * must be exactly what the Rust `generate_handler!` list registers, and every
 * failure must be normalized into one shape. A typo'd command name or a lost
 * error code is the kind of bug that only shows up at runtime in a packaged app,
 * so it is pinned here.
 */

import { describe, expect, it, vi } from 'vitest';

import { COMMANDS, CoreClient, type InvokeTransport } from '../domain/ipc';
import { toCommandError } from '../domain/health';
import { failedReport, healthyReport } from './fixtures';

describe('COMMANDS', () => {
  it('matches the Rust command registrations exactly', () => {
    // Keep this list in sync with `invoke_handler(tauri::generate_handler![...])`
    // in src-tauri/src/lib.rs. snake_case on the wire, camelCase in TS.
    expect(COMMANDS.getStatus).toBe('get_status');
    expect(COMMANDS.getWorldLocation).toBe('get_world_location');
    expect(COMMANDS.ping).toBe('ping');
  });
});

describe('CoreClient', () => {
  it('passes the command name through and returns the payload', async () => {
    const transport = vi.fn(async () => healthyReport()) as unknown as InvokeTransport;
    const client = new CoreClient(transport);

    const report = await client.getStatus();

    expect(report.status).toBe('ok');
    expect(transport).toHaveBeenCalledWith('get_status', undefined);
  });

  it('returns null for an in-memory world location', async () => {
    const transport = vi.fn(async () => null) as unknown as InvokeTransport;
    const client = new CoreClient(transport);
    await expect(client.getWorldLocation()).resolves.toBeNull();
  });

  it('normalizes structured CommandError payloads', async () => {
    const transport = vi.fn(async () => {
      throw { code: 'storage_unreachable', message: 'store is not reachable: disk gone' };
    }) as unknown as InvokeTransport;
    const client = new CoreClient(transport);

    await expect(client.getStatus()).rejects.toMatchObject({
      code: 'storage_unreachable',
      message: 'store is not reachable: disk gone',
    });
  });

  it('normalizes plain Error values into the same shape', async () => {
    const transport = vi.fn(async () => {
      throw new Error('the core is not listening');
    }) as unknown as InvokeTransport;
    const client = new CoreClient(transport);

    await expect(client.getStatus()).rejects.toMatchObject({
      code: 'unknown_error',
      message: 'the core is not listening',
    });
  });

  it('preserves the original thrown value on the wrapped error', async () => {
    const original = { code: 'x', message: 'y' };
    const transport = vi.fn(async () => {
      throw original;
    }) as unknown as InvokeTransport;
    const client = new CoreClient(transport);

    try {
      await client.ping();
      throw new Error('should have thrown');
    } catch (err) {
      expect((err as { cause: unknown }).cause).toEqual(original);
    }
  });
});

describe('toCommandError', () => {
  it('accepts a well-formed payload', () => {
    expect(toCommandError({ code: 'a', message: 'b' })).toEqual({ code: 'a', message: 'b' });
  });

  it('falls back for primitives, null and malformed objects', () => {
    expect(toCommandError('boom').code).toBe('unknown_error');
    expect(toCommandError(undefined).code).toBe('unknown_error');
    expect(toCommandError({ code: 7 }).code).toBe('unknown_error');
  });
});

describe('failed report fixture', () => {
  it('represents an unreachable store without a database or round trip', () => {
    const report = failedReport();
    expect(report.status).toBe('failed');
    expect(report.database).toBeNull();
    expect(report.roundTrip).toBeNull();
    expect(report.problems).toHaveLength(3);
  });
});
