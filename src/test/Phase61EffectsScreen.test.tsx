import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { CoreClient } from '../domain/ipc';
import type { Concept, Effect, EffectHistoryEntry, EffectType, Player, QuestSession } from '../domain/world';
import { EffectsScreen } from '../features/world/EffectsScreen';

afterEach(cleanup);

const player: Player = {
  id: 'p1', name: 'Test Player', description: null, level: 1, levelName: null,
  progressionLabel: null, currentXp: 0, isActive: true, metadataJson: '{}',
  createdAt: '2026-01-01T00:00:00Z', updatedAt: '2026-01-01T00:00:00Z',
};
const types: EffectType[] = [{ code: 'buff', label: 'Buff', description: 'An authored state', sortOrder: 0 }];
const effect: Effect = {
  id: 'e1', playerId: 'p1', targetKind: 'player', targetConceptId: null,
  typeCode: 'buff', name: 'Focus', description: 'A recorded state',
  startedAt: '2026-01-01T00:00:00Z', expiresAt: null, deactivatedAt: null, deactivationSource: null, intensity: 2,
};
const concepts: Concept[] = [];
const sessions: QuestSession[] = [];

function setup(effects: Effect[] = []) {
  const client = new CoreClient(async () => { throw new Error('unexpected command'); });
  vi.spyOn(client, 'listEffectTypes').mockResolvedValue(types);
  const create = vi.spyOn(client, 'createEffect').mockResolvedValue(effect);
  const update = vi.spyOn(client, 'updateEffect').mockResolvedValue(effect);
  const deactivate = vi.spyOn(client, 'deactivateEffect').mockResolvedValue(effect);
  const history: EffectHistoryEntry[] = [{
    id: 'h1', playerId: 'p1', effectId: 'e1', sessionId: null,
    eventKind: 'created', recordedAt: '2026-01-01T00:00:00Z',
    previousStateJson: null, currentStateJson: '{"name":"Focus"}',
  }];
  const listHistory = vi.spyOn(client, 'listEffectHistory').mockResolvedValue(history);
  const refresh = vi.fn(async () => {});
  render(<EffectsScreen
    client={client} player={player} effects={effects} concepts={concepts} sessions={sessions}
    isVisible={() => true} onVisibility={async () => {}} showHidden={false}
    onShowHidden={() => {}} onRefresh={refresh}
  />);
  return { client, create, update, deactivate, listHistory, refresh };
}

describe('Phase 6.1 Effects manager', () => {
  it('creates an indefinite Effect without inventing an expiry or Concept target', async () => {
    const { create } = setup();
    await screen.findByRole('option', { name: /Buff/ });
    expect(screen.getByText(/Never — no expiry recorded/i)).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Focus before a talk' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create Effect' }));
    await waitFor(() => expect(create).toHaveBeenCalledWith('p1', expect.objectContaining({
      name: 'Focus before a talk', expiresAt: null, targetConceptId: null, intensity: 1,
    }), undefined));
  });

  it('loads and shows append-only Effect history without mutating the Effect', async () => {
    const { listHistory } = setup([effect]);
    fireEvent.click(await screen.findByRole('button', { name: 'View history' }));
    expect(await screen.findByText('Effect history · 1 recorded event')).toBeInTheDocument();
    expect(await screen.findByText('created')).toBeInTheDocument();
    expect(listHistory).toHaveBeenCalledWith('p1', 'e1');
  });

  it('keeps future-expiring and indefinite Effects manageable but not scheduled Effects', async () => {
    const future: Effect = {
      ...effect, id: 'future', name: 'Tomorrow focus',
      expiresAt: '2099-01-01T00:00:00Z',
    };
    const scheduled: Effect = {
      ...effect, id: 'scheduled', name: 'Future focus',
      startedAt: '2099-01-01T00:00:00Z',
    };
    const { deactivate } = setup([effect, future, scheduled]);
    expect(await screen.findByText('scheduled')).toBeInTheDocument();
    expect(await screen.findAllByRole('button', { name: 'Manually deactivate' })).toHaveLength(2);
    fireEvent.click(screen.getAllByRole('button', { name: 'Manually deactivate' })[1]!);
    await waitFor(() => expect(deactivate).toHaveBeenCalledWith('p1', 'future', undefined));
  });

  it('does not offer deactivation for expired Effects but clearly documents explicit expiry extension', async () => {
    const expired: Effect = {
      ...effect, id: 'expired', name: 'Exam focus',
      startedAt: '2020-01-01T00:00:00Z', expiresAt: '2020-01-02T00:00:00Z',
    };
    const { update, deactivate } = setup([expired]);
    expect(await screen.findByText('expired')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Manually deactivate' })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
    expect(screen.getByRole('note')).toHaveTextContent(/descriptive edits keep it expired/i);
    expect(screen.getByRole('note')).toHaveTextContent(/deliberate lifecycle edit, recorded in history/i);
    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Exam focus — extended deliberately' } });
    fireEvent.change(screen.getByLabelText('Expires at'), { target: { value: '2099-01-01T00:00' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save Effect' }));
    await waitFor(() => expect(update).toHaveBeenCalledWith('p1', 'expired', expect.objectContaining({
      name: 'Exam focus — extended deliberately',
      expiresAt: new Date('2099-01-01T00:00').toISOString(),
    })));
    expect(deactivate).not.toHaveBeenCalled();
  });
});
