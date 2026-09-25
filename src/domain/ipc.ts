/**
 * The IPC boundary, wrapped and typed.
 *
 * Every call into the Rust core goes through this module. Nothing else in the
 * app imports `invoke` directly. Two reasons that matters:
 *
 * 1. **One place to type the boundary.** `invoke` returns `unknown`; this module
 *    is where that `unknown` becomes a `HealthReport`. If the contract changes,
 *    it changes here and the compiler tells us what else broke.
 * 2. **One place to test the boundary.** `src/test/ipc.test.ts` stubs
 *    `__TAURI_INTERNALS__` and exercises the real code paths, including the
 *    failure paths, without booting a webview.
 *
 * Command names are declared once in `COMMANDS` so a typo is a compile error
 * rather than a runtime 404-style rejection.
 */

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import type { CommandError, HealthReport } from '../domain/health';
import { toCommandError } from '../domain/health';

/** Command names exposed by `src-tauri/src/commands/`. */
export const COMMANDS = {
  getStatus: 'get_status',
  getWorldLocation: 'get_world_location',
  ping: 'ping',
} as const;

export type CommandName = (typeof COMMANDS)[keyof typeof COMMANDS];

/** Injectable transport so tests can drive the API without a webview. */
export type InvokeTransport = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

const defaultTransport: InvokeTransport = async <T,>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> => {
  // `invoke` is untyped at the boundary on purpose; this is the single place
  // where an `unknown` payload becomes a contract type.
  return (await tauriInvoke(command, args)) as T;
};

/**
 * The typed client for the Life RPG core.
 *
 * Construct one with the default transport in the app; construct one with a
 * fake in tests.
 */
export class CoreClient {
  private readonly transport: InvokeTransport;

  constructor(transport: InvokeTransport = defaultTransport) {
    this.transport = transport;
  }

  /** Liveness check that never touches storage. */
  async ping(): Promise<string> {
    return this.invoke<string>(COMMANDS.ping);
  }

  /** Full status, including a real persistence round trip. */
  async getStatus(): Promise<HealthReport> {
    return this.invoke<HealthReport>(COMMANDS.getStatus);
  }

  /** Path of the world database on disk, if file-backed. */
  async getWorldLocation(): Promise<string | null> {
    return this.invoke<string | null>(COMMANDS.getWorldLocation);
  }

  private async invoke<T>(command: CommandName, args?: Record<string, unknown>): Promise<T> {
    try {
      return await this.transport<T>(command, args);
    } catch (error) {
      // Normalize everything into a CommandError so callers have exactly one
      // failure shape to handle. The original value is preserved on `cause`
      // for the console.
      const normalized: CommandError = toCommandError(error);
      const wrapped = new Error(normalized.message) as Error & { code: string; cause: unknown };
      wrapped.code = normalized.code;
      wrapped.cause = error;
      throw wrapped;
    }
  }
}

/** Convenience singleton for app code. Tests should build their own client. */
export const coreClient = new CoreClient();
