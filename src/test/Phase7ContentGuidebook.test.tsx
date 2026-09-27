import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { CoreClient } from '../domain/ipc';
import type { Concept, NarrativeEntry, Player, TypeDefinition, WorldOverview } from '../domain/world';
import { ContentGuidebook } from '../features/world/ContentGuidebook';

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

const player: Player = { id: 'player-7', name: 'Ada', description: null, level: 1, levelName: null, progressionLabel: null, currentXp: 0, isActive: true, metadataJson: '{}', createdAt: '2026-09-27T00:00:00Z', updatedAt: '2026-09-27T00:00:00Z' };
const concept: Concept = { id: 'concept-7', playerId: player.id, transferKey: 'concept-7-key', typeCode: 'subject', name: 'Python', description: null, isActive: true, metadataJson: '{}', createdAt: '2026-09-27T00:00:00Z', updatedAt: '2026-09-27T00:00:00Z' };
const guide: NarrativeEntry = { id: 'content-7', playerId: player.id, kind: 'guide', title: 'Python Learning Path', content: 'Read the first chapter before practice.', author: 'Ada', sourceKind: 'book', sourceId: 'isbn-7', isActive: true, metadataJson: '{}', createdAt: '2026-09-27T00:00:00Z', updatedAt: '2026-09-27T01:00:00Z' };
const overview: WorldOverview = { player, quests: [], skillTrees: [], skills: [], effects: [], recentTransactions: [], narratives: [guide] };
const contentTypes: TypeDefinition[] = [
  { code: 'note', namespace: 'narrative_entry', label: 'Note', description: null, sortOrder: 10, isActive: true, isSystem: true, metadataJson: '{}' },
  { code: 'guide', namespace: 'narrative_entry', label: 'Guide', description: null, sortOrder: 20, isActive: true, isSystem: true, metadataJson: '{}' },
  { code: 'todo', namespace: 'narrative_entry', label: 'To Do', description: null, sortOrder: 30, isActive: true, isSystem: false, metadataJson: '{}' },
];
function client(overrides: Record<string, unknown> = {}) {
  return {
    listTypeDefinitions: vi.fn(async () => contentTypes),
    listContentAttachmentRoles: vi.fn(async () => ['guidance', 'reference']),
    listContentRelationships: vi.fn(async () => []),
    createNarrative: vi.fn(async (_player: string, value: Omit<NarrativeEntry, 'id'|'playerId'|'isActive'|'metadataJson'|'createdAt'|'updatedAt'>) => ({ ...guide, ...value, id: 'created-7', author: value.author ?? null, sourceKind: value.sourceKind ?? null, sourceId: value.sourceId ?? null })),
    updateNarrative: vi.fn(async (_player: string, id: string, value: Partial<NarrativeEntry>) => ({ ...guide, ...value, id })),
    attachContent: vi.fn(async (_player: string, contentId: string, targetKind: string, targetId: string, roleCode: string) => ({ id: 'rel-7', contentId, playerId: player.id, targetKind, targetId, roleCode, sortOrder: 0, isActive: true, createdAt: '2026-09-27T02:00:00Z', updatedAt: '2026-09-27T02:00:00Z', removedAt: null })),
    ...overrides,
  } as unknown as CoreClient;
}

const props = { player, overview, concepts: [concept], onRefresh: vi.fn(async () => {}), onOpenEntity: vi.fn() };

describe('Phase 7 Content Guidebook', () => {
  it('browses, filters, inspects, and exposes lifecycle history for typed content', async () => {
    const onOpenEntity = vi.fn();
    const registryClient = client();
    render(<ContentGuidebook {...props} client={registryClient} onOpenEntity={onOpenEntity} />);
    await waitFor(() => expect(registryClient.listTypeDefinitions).toHaveBeenCalledWith('narrative_entry'));
    expect(screen.getByText('Python Learning Path')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Python Learning Path'));
    expect((await screen.findAllByText('Read the first chapter before practice.')).at(-1)).toHaveClass('guidebook-body');
    expect(screen.getByText('book · isbn-7')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: 'todo' } });
    expect(screen.getByText('No matching content')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Kind'), { target: { value: 'guide' } });
    fireEvent.click(screen.getByRole('button', { name: 'Lifecycle & history' }));
    expect(onOpenEntity).toHaveBeenCalledWith('narrative_entry', guide.id);
  });

  it('creates plain-text content and attaches the canonical record without mutating a target', async () => {
    const createNarrative = vi.fn(async (_player: string, value: { kind: string; title: string; content: string; author?: string|null; sourceKind?: string|null; sourceId?: string|null }) => ({ ...guide, ...value, id: 'created-7', author: value.author ?? null, sourceKind: value.sourceKind ?? null, sourceId: value.sourceId ?? null }));
    const attachContent = vi.fn(async (_player: string, contentId: string, targetKind: string, targetId: string, roleCode: string) => ({ id: 'rel-7', contentId, playerId: player.id, targetKind, targetId, roleCode, sortOrder: 0, isActive: true, createdAt: '2026-09-27T02:00:00Z', updatedAt: '2026-09-27T02:00:00Z', removedAt: null }));
    const removeContentAttachment = vi.fn(async () => {});
    render(<ContentGuidebook {...props} client={client({ createNarrative, attachContent, removeContentAttachment })} />);
    fireEvent.click(screen.getByRole('button', { name: /New content/ }));
    fireEvent.change(screen.getByLabelText('Title'), { target: { value: 'Session checklist' } });
    fireEvent.change(screen.getByLabelText('Body'), { target: { value: 'Read, practise, reflect.' } });
    fireEvent.change(screen.getByLabelText('Author optional'), { target: { value: 'Ada' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save content' }));
    await waitFor(() => expect(createNarrative).toHaveBeenCalledWith(player.id, expect.objectContaining({ kind: 'note', title: 'Session checklist', content: 'Read, practise, reflect.', author: 'Ada' })));
    fireEvent.change(screen.getByLabelText('Target type'), { target: { value: 'concept' } });
    fireEvent.change(screen.getByLabelText('Target'), { target: { value: concept.id } });
    fireEvent.change(screen.getByLabelText('Role'), { target: { value: 'reference' } });
    fireEvent.click(screen.getByRole('button', { name: 'Attach existing content' }));
    await waitFor(() => expect(attachContent).toHaveBeenCalledWith(player.id, 'created-7', 'concept', concept.id, 'reference'));
    expect(screen.getByText(/Concept · concept-7/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Remove link' }));
    await waitFor(() => expect(removeContentAttachment).toHaveBeenCalledWith(player.id, 'rel-7'));
  });

  it('uses a dynamically registered Content type for filtering, ordering, and creation', async () => {
    const practice: TypeDefinition = { code: 'practice', namespace: 'narrative_entry', label: 'Practice', description: null, sortOrder: 15, isActive: true, isSystem: false, metadataJson: '{}' };
    const registry = [contentTypes[0]!, practice, contentTypes[1]!, { ...contentTypes[0]!, namespace: 'quest', code: 'main', label: 'Quest' }];
    const createNarrative = vi.fn(async (_player: string, value: { kind: string; title: string; content: string }) => ({ ...guide, ...value, id: 'created-practice' }));
    render(<ContentGuidebook {...props} client={client({ listTypeDefinitions: vi.fn(async () => registry), createNarrative })} />);
    await waitFor(() => expect(screen.getByRole('option', { name: 'Practice' })).toBeInTheDocument());
    const kind = screen.getByLabelText('Kind');
    expect(Array.from(kind.querySelectorAll('option')).map(option => option.textContent)).toEqual(['All kinds', 'Note', 'Practice', 'Guide']);
    fireEvent.change(kind, { target: { value: 'practice' } });
    expect(screen.getByText('No matching content')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /New content/ }));
    expect(screen.getAllByRole('option', { name: 'Practice' })).toHaveLength(2);
    fireEvent.change(screen.getByLabelText('Content kind'), { target: { value: 'practice' } });
    fireEvent.change(screen.getByLabelText('Title'), { target: { value: 'Practice plan' } });
    fireEvent.change(screen.getByLabelText('Body'), { target: { value: 'Repeat the exercise.' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save content' }));
    await waitFor(() => expect(createNarrative).toHaveBeenCalledWith(player.id, expect.objectContaining({ kind: 'practice' })));
  });

  it('keeps existing Content with an inactive kind readable and preserves it during editing', async () => {
    const legacy: NarrativeEntry = { ...guide, kind: 'legacy', title: 'Legacy entry' };
    const inactive: TypeDefinition = { code: 'legacy', namespace: 'narrative_entry', label: 'Legacy', description: null, sortOrder: 99, isActive: false, isSystem: false, metadataJson: '{}' };
    const updateNarrative = vi.fn(async (_player: string, id: string, value: Partial<NarrativeEntry>) => ({ ...legacy, ...value, id }));
    render(<ContentGuidebook {...props} overview={{ ...overview, narratives: [legacy] }} client={client({ listTypeDefinitions: vi.fn(async () => [...contentTypes, inactive]), updateNarrative })} />);
    fireEvent.click(await screen.findByText('Legacy entry'));
    expect(screen.getAllByText('Legacy · inactive')).toHaveLength(2);
    fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
    expect(screen.getByRole('option', { name: 'Legacy (inactive)' })).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Title'), { target: { value: 'Updated legacy entry' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save revision' }));
    await waitFor(() => expect(updateNarrative).toHaveBeenCalledWith(player.id, legacy.id, expect.objectContaining({ kind: 'legacy' })));
  });

  it('renders loading, error, and empty registry states without crashing', async () => {
    let resolve: ((value: TypeDefinition[]) => void) | undefined;
    const pending = new Promise<TypeDefinition[]>(done => { resolve = done; });
    const { unmount } = render(<ContentGuidebook {...props} client={client({ listTypeDefinitions: vi.fn(() => pending) })} />);
    expect(screen.getByText('Loading content types…')).toBeInTheDocument();
    resolve!([]);
    await waitFor(() => expect(screen.getByText('No active content types are registered.')).toBeInTheDocument());
    unmount();
    render(<ContentGuidebook {...props} client={client({ listTypeDefinitions: vi.fn(async () => { throw new Error('registry offline'); }) })} />);
    expect(await screen.findByText(/Content types unavailable: registry offline/)).toBeInTheDocument();
  });
});
