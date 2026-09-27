import { describe, expect, it, vi } from 'vitest';
import { CoreClient, type InvokeTransport } from '../domain/ipc';
import { xpProgress, type Player } from '../domain/world';

const player = (): Player => ({ id: 'p1', name: 'Ada', description: null, level: 1, currentXp: 250, isActive: true, metadataJson: '{}', createdAt: '2026-09-25T00:00:00+00:00', updatedAt: '2026-09-25T00:00:00+00:00' });

describe('Phase 2 world IPC client', () => {
  it('sends camelCase player and XP payloads to matching commands', async () => {
    const transport = vi.fn(async (): Promise<unknown> => player());
    const client = new CoreClient(transport as unknown as InvokeTransport);
    await client.createPlayer('Ada', 'Testing');
    expect(transport).toHaveBeenCalledWith('create_player', { name: 'Ada', description: 'Testing' });
    transport.mockResolvedValueOnce({ player: player(), transaction: {} });
    await client.awardXp('p1', 25, 'test');
    expect(transport).toHaveBeenLastCalledWith('award_xp', { playerId: 'p1', amount: 25, reason: 'test', description: null });
  });

  it('computes progress within the current level', () => {
    expect(xpProgress(player())).toBeCloseTo(0.25);
  });
});
