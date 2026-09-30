import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { EntitySurfaceProvider, useEntitySurface } from '../features/entity-surface/EntitySurface';
import type { Concept, Player, Quest, WorldOverview } from '../domain/world';
import type { CoreClient } from '../domain/ipc';

afterEach(() => { document.body.style.overflow = ''; });

const player: Player = { id: 'player-1', name: 'Rin', description: 'A test world', level: 3, levelName: 'Wayfinder', progressionLabel: 'Mapping the unknown', currentXp: 920, isActive: true, metadataJson: '{}', createdAt: '', updatedAt: '' };
const quest: Quest = { id: 'quest-1', playerId: player.id, typeCode: 'main', parentQuestId: null, skillId: null, title: 'Prepare the garden', status: 'in_progress', difficulty: null, progress: 25, xpReward: 0, dueAt: null, startedAt: null, completedAt: null, description: 'A small objective' };
const concept: Concept = { id: 'concept-1', playerId: player.id, typeCode: 'project', name: 'Garden', description: 'A place to tend', isActive: true, metadataJson: '{}', createdAt: '', updatedAt: '' };
const overview: WorldOverview = { player, quests: [quest], skillTrees: [], skills: [], effects: [], recentTransactions: [], narratives: [] };
const client = { setPlayerProgression: vi.fn(async () => player) } as unknown as CoreClient;
function Launcher() { const { openSurface } = useEntitySurface(); return <button onClick={() => openSurface('quest', quest.id)}>Open quest</button>; }
function renderSurface() { return render(<EntitySurfaceProvider client={client} player={player} overview={overview} concepts={[concept]} onRefresh={async () => {}}><Launcher /></EntitySurfaceProvider>); }

describe('EntitySurface', () => {
  it('opens the correct entity without replacing the launcher context', async () => {
    renderSurface();
    fireEvent.click(screen.getByRole('button', { name: 'Open quest' }));
    expect(await screen.findByRole('heading', { name: 'Prepare the garden' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open quest' })).toBeInTheDocument();
    expect(document.body.style.overflow).toBe('hidden');
  });

  it('supports nested surfaces and returns to the parent', async () => {
    renderSurface();
    fireEvent.click(screen.getByRole('button', { name: 'Open quest' }));
    fireEvent.click(await screen.findByRole('button', { name: /Garden/ }));
    expect(await screen.findByRole('heading', { name: 'Garden' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Back to Prepare the garden/ }));
    expect(await screen.findByRole('heading', { name: 'Prepare the garden' })).toBeInTheDocument();
  });

  it('closes the topmost surface on Escape and restores the triggering focus', async () => {
    renderSurface();
    const trigger = screen.getByRole('button', { name: 'Open quest' });
    trigger.focus();
    fireEvent.click(trigger);
    await screen.findByRole('heading', { name: 'Prepare the garden' });
    fireEvent.keyDown(document, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByTestId('entity-surface-layer')).not.toBeInTheDocument());
    expect(document.activeElement).toBe(trigger);
  });

  it('saves Player edits in the same surface', async () => {
    render(<EntitySurfaceProvider client={client} player={player} overview={overview} concepts={[concept]} onRefresh={async () => {}}><button onClick={() => {}}>Context</button><OpenPlayer /></EntitySurfaceProvider>);
    fireEvent.click(screen.getByRole('button', { name: 'Open player' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Edit' }));
    fireEvent.change(screen.getByLabelText('Level'), { target: { value: '4' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(client.setPlayerProgression).toHaveBeenCalledWith(player.id, 4, 'Wayfinder', 'Mapping the unknown'));
    expect(await screen.findByRole('status')).toHaveTextContent('Player details saved');
  });
});
function OpenPlayer() { const { openSurface } = useEntitySurface(); return <button onClick={() => openSurface('player', player.id)}>Open player</button>; }
