import { afterEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { CoreClient } from '../domain/ipc';
import type { Concept, ConceptProgressTrack, Effect, Player, PlayerStat, Quest, QuestSession, Skill, SkillTree, Workspace, WorkspacePanel, WorldOverview } from '../domain/world';
import { PlayerCharacter } from '../features/world/PlayerCharacter';
import { WorldWorkspace } from '../features/world/WorldWorkspace';
import { buildWorldTimeline, formatSessionDuration } from '../features/world/worldTimeline';

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

const player: Player = { id: 'player-1', name: 'Rin', description: null, level: 3, levelName: 'Wayfinder', progressionLabel: null, currentXp: 90, isActive: true, metadataJson: '{}', createdAt: '2026-09-20T00:00:00Z', updatedAt: '2026-09-27T00:00:00Z' };
const quest: Quest = { id: 'quest-1', playerId: player.id, typeCode: 'main', parentQuestId: null, skillId: null, title: 'Prepare the garden', status: 'active', difficulty: null, progress: 20, xpReward: 0, dueAt: null, startedAt: '2026-09-27T08:00:00Z', completedAt: null, description: 'A recorded outcome' };
const tree: SkillTree = { id: 'tree-1', playerId: player.id, typeCode: 'life', name: 'Practice', description: null, isActive: true };
const skill: Skill = { id: 'skill-1', skillTreeId: tree.id, parentSkillId: null, typeCode: 'core', name: 'Observation', level: 2, levelName: null, progressionLabel: null, currentXp: 0, investedMinutes: 0, status: 'active' };
const concept: Concept = { id: 'concept-1', playerId: player.id, transferKey: 'concept-key', typeCode: 'subject', name: 'Garden', description: null, isActive: true, metadataJson: '{}', createdAt: '2026-09-20T00:00:00Z', updatedAt: '2026-09-27T09:00:00Z' };
const overview: WorldOverview = { player, quests: [quest], skillTrees: [tree], skills: [skill], effects: [], recentTransactions: [], narratives: [] };
const activeSession: QuestSession = { id: 'session-1', playerId: player.id, questId: quest.id, stageId: null, branchId: null, skillId: null, conceptId: null, startedAt: '2026-09-27T09:00:00Z', endedAt: null, status: 'in_progress', progressBefore: null, progressAfter: null, result: null, notes: null, isActive: true, metadataJson: '{}', createdAt: '2026-09-27T09:00:00Z', updatedAt: '2026-09-27T09:00:00Z' };
const workspace: Workspace = { id: 'workspace-1', playerId: player.id, name: 'Overview', template: 'overview', sortOrder: 0, isDefault: true, createdAt: '', updatedAt: '' };
const panel: WorkspacePanel = { id: 'panel-1', workspaceId: workspace.id, panelType: 'quests', title: 'Objectives', variant: 'rows', density: 'cozy', filterStatus: null, filterActive: null, filterTypeCode: null, filterConceptId: null, filterRecentDays: null, sortBy: 'name_asc', itemLimit: 6, sortOrder: 0, gridSpan: 1, isVisible: true, isPinned: false, isCollapsed: false, createdAt: '', updatedAt: '' };
function client(overrides: Record<string, unknown> = {}) { return { listStatDefinitions: vi.fn(async () => []), listPlayerSnapshots: vi.fn(async () => []), listPresentationPreferences: vi.fn(async () => []), listConceptProgress: vi.fn(async () => []), ...overrides } as unknown as CoreClient; }

const common = { client: client(), player, overview, concepts: [concept], stats: [] as PlayerStat[], sessions: [activeSession], tracks: {} as Record<string, ConceptProgressTrack[]>, onRefresh: async () => {}, onNavigate: vi.fn(), onOpenEntity: vi.fn(), onQuickCapture: vi.fn() };

describe('Phase 6 Player Hub interaction loop', () => {
  it('shows Rust-emitted active Quest and in-progress Session values, and exposes the real end action', async () => {
    const finishQuestSession = vi.fn(async () => ({ ...activeSession, status: 'completed', endedAt: '2026-09-27T09:45:00Z' }));
    const finishClient = client({ finishQuestSession, listPlayerSnapshots: vi.fn(async () => []) });
    render(<PlayerCharacter {...common} client={finishClient} />);
    expect(await screen.findByText('Prepare the garden', { selector: 'strong' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Complete' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Active Sessions' })).toBeInTheDocument();
    expect(document.querySelector('.player-hub__session-copy strong')).toHaveTextContent('Quest · Prepare the garden');
    fireEvent.click(screen.getByRole('button', { name: 'End or record outcome' }));
    fireEvent.change(screen.getByLabelText('Result optional'), { target: { value: 'Outlined the first section' } });
    fireEvent.change(screen.getByLabelText('Notes optional'), { target: { value: 'Need to revisit the introduction.' } });
    fireEvent.click(screen.getByRole('button', { name: 'Record completion' }));
    await waitFor(() => expect(finishQuestSession).toHaveBeenCalledWith(activeSession.id, 'completed', 'Outlined the first section', 'Need to revisit the introduction.'));
  });

  it('uses completed-session timestamps for duration and rejects missing, invalid or negative duration', () => {
    expect(formatSessionDuration('2026-09-27T08:00:00Z', '2026-09-27T09:35:00Z')).toBe('1 hr 35 min');
    expect(formatSessionDuration('2026-09-27T08:00:00Z', null)).toBeNull();
    expect(formatSessionDuration('not a timestamp', '2026-09-27T09:35:00Z')).toBeNull();
    expect(formatSessionDuration('2026-09-27T10:00:00Z', '2026-09-27T09:35:00Z')).toBeNull();
  });

  it('builds history only from persisted timestamps and keeps stable newest-first order', () => {
    const completed: QuestSession = { ...activeSession, status: 'completed', endedAt: '2026-09-27T09:45:00Z' };
    const records = buildWorldTimeline({ player, overview, concepts: [concept], stats: [], sessions: [completed], tracks: {} });
    expect(records.map(item => item.title)).toEqual(['Session ended', 'Session started', 'Quest started']);
    expect(records[0]?.detail).toContain('45 min');
    expect(records.map(item => item.at)).not.toContain('');
  });

  it('opens a persisted Quest directly from a populated configurable panel row', async () => {
    const onOpenEntity = vi.fn();
    await act(async () => {
      render(<WorldWorkspace route="player" client={client()} player={player} overview={overview} concepts={[concept]} stats={[]} sessions={[activeSession]} workspace={workspace} workspaces={[workspace]} panels={[panel]} onPanelsChange={() => {}} onCreateWorkspace={async () => {}} onRenameWorkspace={async () => {}} onDefaultWorkspace={async () => {}} onDeleteWorkspace={async () => {}} onDuplicateWorkspace={async () => {}} onImportWorkspace={async () => {}} onRefresh={async () => {}} onCreatePlayer={async () => {}} onNavigate={() => {}} onOpenEntity={onOpenEntity} onQuickCapture={() => {}} />);
      await new Promise(resolve => setTimeout(resolve, 0));
    });
    fireEvent.click(screen.getAllByRole('button', { name: /Prepare the garden/ }).at(-1)!);
    expect(onOpenEntity).toHaveBeenCalledWith('quest', quest.id);
    expect(screen.getByRole('button', { name: 'Customize workspace' })).toBeInTheDocument();
  });
});


describe('Phase 6 contextual navigation', () => {
  it('opens the exact requested record in the active world and returns to the Player Hub', async () => {
    const hit = { kind: 'quest', id: quest.id, playerId: player.id, conceptId: null, typeCode: quest.typeCode, status: 'active', active: true, lifecycle: 'active' as const, visible: true, occurredAt: quest.startedAt, capturedAt: null, name: quest.title, snippet: quest.description ?? '', progression: quest.progress, relevance: null };
    const searchWorld = vi.fn(async () => [hit]);
    const onReturnTo = vi.fn();
    const explorerClient = client({
      searchWorld,
      getEntityLifecycle: vi.fn(async () => ({ state: 'active' as const })),
      listEntityRevisions: vi.fn(async () => []),
      listAttachedContent: vi.fn(async () => []),
      listComments: vi.fn(async () => []),
    });
    const { WorldExplorer } = await import('../features/world/WorldExplorer');
    render(<WorldExplorer client={explorerClient} player={player} concepts={[concept]} overview={overview} onRefresh={async () => {}} initialTarget={{ kind: 'quest', id: quest.id }} onReturnTo={onReturnTo} />);
    expect(await screen.findByRole('heading', { name: quest.title })).toBeInTheDocument();
    expect(searchWorld).toHaveBeenCalledWith(expect.objectContaining({ kind: 'quest', playerId: player.id, includeTrashed: true }));
    fireEvent.click(screen.getByRole('button', { name: '← Return to previous view' }));
    expect(onReturnTo).toHaveBeenCalledOnce();
  });
});


describe('Phase 6 current state and contextual actions', () => {
  it('shows truthful empty states and does not infer missing Quest activity', async () => {
    const emptyOverview = { ...overview, quests: [], skills: [], effects: [], recentTransactions: [], narratives: [] };
    render(<PlayerCharacter {...common} overview={emptyOverview} concepts={[]} sessions={[]} />);
    expect(await screen.findByText('No open Quests')).toBeInTheDocument();
    expect(screen.getByText('No active Sessions')).toBeInTheDocument();
    expect(screen.getByText('No recorded activity yet')).toBeInTheDocument();
  });

  it('surfaces hidden records without changing them and provides Explorer navigation', async () => {
    const hiddenQuest = { playerId: player.id, entityKind: 'quest', entityId: quest.id, context: 'dashboard', isVisible: false, sortOrder: 0, isPinned: false, isCollapsed: null, variant: null, density: null, metadataJson: '{}', createdAt: '', updatedAt: '' };
    const onNavigate = vi.fn();
    render(<PlayerCharacter {...common} client={client({ listPresentationPreferences: vi.fn(async () => [hiddenQuest]) })} onNavigate={onNavigate} />);
    expect(await screen.findByText(/1 item hidden in this view/)).toBeInTheDocument();
    expect(screen.queryByText('Prepare the garden', { selector: 'strong' })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Review in Explorer →' }));
    expect(onNavigate).toHaveBeenCalledWith('explorer');
    expect(quest.status).toBe('active');
  });

  it('shows an actually active Effect, distinguishes a past expiry without changing it, and supports explicit deactivation', async () => {
    const effect: Effect = { id: 'effect-1', playerId: player.id, targetKind: 'player', targetConceptId: null, typeCode: 'condition', name: 'Rested', description: null, startedAt: '2001-01-01T10:00:00Z', expiresAt: '2001-01-01T11:00:00Z', deactivatedAt: null, intensity: 1 };
    const deactivateEffect = vi.fn(async () => ({ ...effect, deactivatedAt: '2026-09-27T12:00:00Z' }));
    render(<PlayerCharacter {...common} overview={{ ...overview, effects: [effect] }} client={client({ deactivateEffect })} />);
    expect(await screen.findByText('Rested', { selector: 'strong' })).toBeInTheDocument();
    expect(screen.getByText('Past recorded expiry · no automatic mutation')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Deactivate' }));
    await waitFor(() => expect(deactivateEffect).toHaveBeenCalledWith(effect.id));
  });

  it('starts an open Quest explicitly before offering a contextual Session action', async () => {
    const openQuest: Quest = { ...quest, status: 'open', startedAt: null };
    const startQuest = vi.fn(async () => ({ ...openQuest, status: 'active' as const }));
    render(<PlayerCharacter {...common} overview={{ ...overview, quests: [openQuest] }} sessions={[]} client={client({ startQuest })} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Start Quest' }));
    await waitFor(() => expect(startQuest).toHaveBeenCalledWith(openQuest.id));
  });

  it('starts a contextual Session from the Player Hub without creating one from navigation', async () => {
    const startQuestSession = vi.fn(async () => activeSession);
    render(<PlayerCharacter {...common} sessions={[]} client={client({ startQuestSession })} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Start Session' }));
    await waitFor(() => expect(startQuestSession).toHaveBeenCalledWith(player.id, { questId: quest.id }));
  });
});


describe('Phase 6 Quest workflow', () => {
  it('starts an open Quest from its own workflow route using the native Quest operation', async () => {
    const openQuest: Quest = { ...quest, status: 'open', startedAt: null };
    const startQuest = vi.fn(async () => ({ ...openQuest, status: 'active' as const }));
    const clientWithStart = client({ startQuest, listQuestStages: vi.fn(async () => []), listPresentationPreferences: vi.fn(async () => []) });
    render(<WorldWorkspace route="quests" client={clientWithStart} player={player} overview={{ ...overview, quests: [openQuest] }} concepts={[concept]} stats={[]} sessions={[]} workspace={null} workspaces={[]} panels={[]} onPanelsChange={() => {}} onCreateWorkspace={async () => {}} onRenameWorkspace={async () => {}} onDefaultWorkspace={async () => {}} onDeleteWorkspace={async () => {}} onDuplicateWorkspace={async () => {}} onImportWorkspace={async () => {}} onRefresh={async () => {}} onCreatePlayer={async () => {}} onNavigate={() => {}} onOpenEntity={() => {}} onQuickCapture={() => {}} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Start quest' }));
    await waitFor(() => expect(startQuest).toHaveBeenCalledWith(openQuest.id));
    expect(screen.queryByRole('button', { name: 'Complete quest' })).not.toBeInTheDocument();
  });
});


describe('Phase 6 Session context navigation', () => {
  it('opens the exact Concept referenced by a Session without issuing a world mutation', async () => {
    const linkedSession: QuestSession = { ...activeSession, conceptId: concept.id };
    const sessionHit = { kind: 'quest_session', id: linkedSession.id, playerId: player.id, conceptId: concept.id, typeCode: 'session', status: 'in_progress', active: true, lifecycle: 'active' as const, visible: true, occurredAt: linkedSession.startedAt, capturedAt: null, name: 'Quest Session', snippet: 'Recorded activity', progression: null, relevance: null };
    const conceptHit = { kind: 'concept', id: concept.id, playerId: player.id, conceptId: null, typeCode: concept.typeCode, status: null, active: true, lifecycle: 'active' as const, visible: true, occurredAt: concept.createdAt, capturedAt: null, name: concept.name, snippet: concept.description ?? '', progression: null, relevance: null };
    const searchWorld = vi.fn(async (query: { kind?: string | null }) => query.kind === 'quest_session' ? [sessionHit] : query.kind === 'concept' ? [conceptHit] : []);
    const setEntityLifecycle = vi.fn();
    const associateConcept = vi.fn();
    const explorerClient = client({ searchWorld, getEntityLifecycle: vi.fn(async () => ({ state: 'active' as const })), listEntityRevisions: vi.fn(async () => []), listPresentationPreferences: vi.fn(async () => []), listConceptAssociations: vi.fn(async () => []), listConceptRelationships: vi.fn(async () => []), listAttachedContent: vi.fn(async () => []), listComments: vi.fn(async () => []), listProgressSuggestions: vi.fn(async () => []), setEntityLifecycle, associateConcept });
    const { WorldExplorer } = await import('../features/world/WorldExplorer');
    render(<WorldExplorer client={explorerClient} player={player} concepts={[concept]} overview={overview} sessions={[linkedSession]} onRefresh={async () => {}} initialTarget={{ kind: 'quest_session', id: linkedSession.id }} />);
    expect(await screen.findByRole('button', { name: 'Concept · Garden' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Concept · Garden' }));
    expect(await screen.findByRole('heading', { name: concept.name })).toBeInTheDocument();
    expect(searchWorld).toHaveBeenLastCalledWith(expect.objectContaining({ kind: 'concept', playerId: player.id, includeHidden: true }));
    expect(setEntityLifecycle).not.toHaveBeenCalled();
    expect(associateConcept).not.toHaveBeenCalled();
  });
});
