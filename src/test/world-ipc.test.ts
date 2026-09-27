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

  it('preserves requested and applied XP while exposing stat and snapshot commands', async () => {
    const transport = vi.fn(async (): Promise<unknown> => []);
    const client = new CoreClient(transport as unknown as InvokeTransport);
    await client.defineStat('focus', 'Focus', { unit: 'points', minimum: 0, maximum: 10 });
    expect(transport).toHaveBeenCalledWith('define_stat', { code: 'focus', name: 'Focus', description: null, unit: 'points', minimum: 0, maximum: 10 });
    await client.setPlayerStat('p1', 'focus', 7.5);
    expect(transport).toHaveBeenLastCalledWith('set_player_stat', { playerId: 'p1', statCode: 'focus', value: 7.5 });
    await client.captureSkillSnapshot('s1');
    expect(transport).toHaveBeenLastCalledWith('capture_skill_snapshot', { skillId: 's1' });
    await client.listPlayerSnapshots('p1');
    expect(transport).toHaveBeenLastCalledWith('list_player_snapshots', { playerId: 'p1' });
  });
});
