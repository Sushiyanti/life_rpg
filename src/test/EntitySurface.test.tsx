import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { EntitySurfaceProvider, useEntitySurface } from '../features/entity-surface/EntitySurface';
import { registerSurface, surfaceRegistry } from '../features/entity-surface/surface-registry';
import type { CoreClient } from '../domain/ipc';
import type { Concept, ConceptAssociation, ConceptRelationship, Effect, Player, Quest } from '../domain/world';

afterEach(() => { document.body.style.overflow = ''; });

const player: Player = { id: 'player-1', name: 'Rin', description: 'A test world', level: 3, levelName: 'Wayfinder', progressionLabel: 'Mapping the unknown', currentXp: 920, isActive: true, metadataJson: '{}', createdAt: '', updatedAt: '' };
const questA: Quest = { id: 'quest-a', playerId: player.id, typeCode: 'main', parentQuestId: null, skillId: null, title: 'Prepare the garden', status: 'in_progress', difficulty: null, progress: 25, xpReward: 0, dueAt: null, startedAt: null, completedAt: null, description: 'Quest A' };
const questB: Quest = { ...questA, id: 'quest-b', title: 'Wrong quest' };
const conceptB: Concept = { id: 'concept-b', playerId: player.id, typeCode: 'project', name: 'Garden', description: 'Concept B', isActive: true, metadataJson: '{}', createdAt: '', updatedAt: '' };
const conceptC: Concept = { ...conceptB, id: 'concept-c', name: 'Harvest', description: 'Concept C' };
const conceptWrong: Concept = { ...conceptB, id: 'concept-wrong', name: 'Unrelated concept' };
const effectC: Effect = { id: 'effect-c', playerId: player.id, targetKind: 'player', targetConceptId: null, typeCode: 'buff', name: 'Focused', description: 'Effect C', startedAt: '', expiresAt: null, deactivatedAt: null, intensity: 2 };
const questAssociation: ConceptAssociation = { id: 'association-a', playerId: player.id, conceptId: conceptB.id, entityKind: 'quest', entityId: questA.id, associationCode: 'about', isActive: true, metadataJson: '{}', createdAt: '', updatedAt: '' };
const relation: ConceptRelationship = { id: 'relation-b-c', playerId: player.id, sourceConceptId: conceptB.id, targetConceptId: conceptC.id, relationshipCode: 'related_to', isActive: true, createdAt: '', updatedAt: '' };

const client = {
  getPlayer: vi.fn(async (id: string) => id === player.id ? player : null),
  getQuest: vi.fn(async (id: string) => [questA, questB].find((quest) => quest.id === id) ?? null),
  getConcept: vi.fn(async (id: string) => [conceptB, conceptC, conceptWrong].find((concept) => concept.id === id) ?? null),
  listEffects: vi.fn(async () => [effectC]),
  listConceptAssociations: vi.fn(async (filters: { conceptId?: string; entityKind?: string; entityId?: string }) => filters.entityKind === 'quest' && filters.entityId === questA.id ? [questAssociation] : filters.conceptId === conceptB.id ? [questAssociation] : []),
  listConceptRelationships: vi.fn(async (id: string) => id === conceptB.id ? [relation] : []),
  setPlayerProgression: vi.fn(async () => player),
  setConceptActive: vi.fn(async () => conceptB),
} as unknown as CoreClient;

function Launcher() {
  const { openSurface } = useEntitySurface();
  return <><button onClick={() => openSurface('quest', questA.id, { label: questA.title })}>Open Quest A</button><button onClick={() => openSurface('quest', questB.id, { label: questB.title })}>Open Quest B</button><button onClick={() => openSurface('effect', effectC.id, { label: effectC.name })}>Open Effect C</button></>;
}
function OpenConcept() { const { openSurface } = useEntitySurface(); return <button onClick={() => openSurface('concept', conceptB.id, { label: conceptB.name })}>Open Concept B</button>; }
function renderSurface() { return render(<EntitySurfaceProvider client={client} player={player} onRefresh={async () => undefined}><Launcher /></EntitySurfaceProvider>); }

describe('EntitySurface', () => {
  it('resolves Quest A by ID and preserves the workspace trigger', async () => {
    renderSurface(); fireEvent.click(screen.getByRole('button', { name: 'Open Quest A' }));
    expect(await screen.findByRole('heading', { name: 'Prepare the garden' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open Quest A' })).toBeInTheDocument();
    expect(client.getQuest).toHaveBeenCalledWith(questA.id);
  });

  it('resolves Effect C lazily and copies human-readable content', async () => {
    renderSurface(); fireEvent.click(screen.getByRole('button', { name: 'Open Effect C' }));
    expect(await screen.findByRole('heading', { name: 'Focused' })).toBeInTheDocument();
    const write = vi.fn().mockResolvedValue(undefined); Object.defineProperty(navigator, 'clipboard', { value: { writeText: write }, configurable: true });
    fireEvent.click(screen.getByRole('button', { name: 'Copy' })); await waitFor(() => expect(write).toHaveBeenCalledWith(expect.stringContaining('Effect: Focused')));
  });

  it('opens Quest A → actual Concept B → actual Concept C and backs through the stack', async () => {
    renderSurface(); fireEvent.click(screen.getByRole('button', { name: 'Open Quest A' })); fireEvent.click(await screen.findByRole('button', { name: /Garden/ }));
    expect(await screen.findByRole('heading', { name: 'Garden' })).toBeInTheDocument(); fireEvent.click(await screen.findByRole('button', { name: /Harvest/ }));
    expect(await screen.findByRole('heading', { name: 'Harvest' })).toBeInTheDocument(); fireEvent.click(screen.getByRole('button', { name: /Back to Garden/ }));
    expect(await screen.findByRole('heading', { name: 'Garden' })).toBeInTheDocument(); fireEvent.click(screen.getByRole('button', { name: /Back to Prepare the garden/ }));
    expect(await screen.findByRole('heading', { name: 'Prepare the garden' })).toBeInTheDocument();
    expect(client.listConceptAssociations).toHaveBeenCalledWith({ entityKind: 'quest', entityId: questA.id });
  });

  it('supports the reusable Concept editor contract with explicit Save and Cancel', async () => {
    render(<EntitySurfaceProvider client={client} player={player} onRefresh={async () => undefined}><OpenConcept /></EntitySurfaceProvider>); fireEvent.click(screen.getByRole('button', { name: 'Open Concept B' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Edit' })); fireEvent.click(screen.getByLabelText('Active in this world')); fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(client.setConceptActive).toHaveBeenCalledWith(conceptB.id, false)); expect(await screen.findByRole('status')).toHaveTextContent('Concept active state saved');
  });

  it('contains Tab focus, locks body scroll, and closes the entire stack with the close button', async () => {
    renderSurface(); const trigger = screen.getByRole('button', { name: 'Open Quest A' }); trigger.focus(); fireEvent.click(trigger); await screen.findByRole('heading', { name: 'Prepare the garden' });
    expect(document.body.style.overflow).toBe('hidden'); const close = screen.getByRole('dialog').querySelector<HTMLButtonElement>('.entity-surface__close')!; close.focus(); fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Tab' }); expect(document.activeElement).toBe(close);
    fireEvent.click(close); await waitFor(() => expect(screen.queryByTestId('entity-surface-layer')).not.toBeInTheDocument()); expect(document.activeElement).toBe(trigger);
  });

  it('uses registry descriptors for a new kind without changing stack mechanics', () => {
    const testDescriptor = registerSurface({ kind: 'test-note', load: async () => ({ entity: null, attachedConcepts: [], relatedConcepts: [] }), title: () => 'Test note', copy: () => 'Test note', render: () => null });
    expect(testDescriptor.kind).toBe('test-note');
    expect(surfaceRegistry['test-note']).toBe(testDescriptor);
  });
});
