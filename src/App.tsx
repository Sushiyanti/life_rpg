import { useCallback, useEffect, useState } from 'react';
import { AppShell, type AppRoute } from './app/AppShell';
import { coreClient } from './domain/ipc';
import type { Concept, Player, PlayerStat, QuestSession, SearchHit, WorldOverview, Workspace, WorkspacePanel, WorkspaceTemplate } from './domain/world';
import { WORKSPACE_TEMPLATES } from './features/workspaces/templates';
import { PANEL_REGISTRY } from './features/workspaces/panelRegistry';
import type { ResolvedWorkspaceImport } from './features/workspaces/workspaceTransfer';
import { RulePanel } from './features/world/RulePanel';
import { StatusScreen } from './features/status/StatusScreen';
import { WorldWorkspace } from './features/world/WorldWorkspace';
import { QuickCapture } from './features/world/QuickCapture';

const ACTIVE_WORLD_KEY = 'life-rpg.active-world.v1';
const ACTIVE_ROUTE_KEY = 'life-rpg.active-route.v1';
const localGet = (key: string): string | null => { try { return localStorage.getItem(key); } catch { return null; } };
const localSet = (key: string, value: string) => { try { localStorage.setItem(key, value); } catch { /* Storage is an optional presentation convenience. */ } };
const localRemove = (key: string) => { try { localStorage.removeItem(key); } catch { /* Storage is an optional presentation convenience. */ } };
const activeWorkspaceKey = (playerId: string) => `life-rpg.active-workspace.v1.${playerId}`;
const knownRoutes: AppRoute[] = ['dashboard', 'player', 'quests', 'skills', 'skillTrees', 'concepts', 'effects', 'journal', 'explorer', 'history', 'rules', 'status'];
const emptyQuery = { text: null, kind: 'player', playerId: null, conceptId: null, typeCode: null, status: null, active: null, from: null, through: null, context: null, includeHidden: true, includeArchived: true, includeTrashed: true, sort: 'name' as const, limit: 200, offset: 0 };
type EntityTarget = { kind: string; id: string; returnRoute: AppRoute };

export function App() {
  const [route, setRoute] = useState<AppRoute>(() => {
    const saved = localGet(ACTIVE_ROUTE_KEY) as AppRoute | null;
    return saved && knownRoutes.includes(saved) ? saved : 'player';
  });
  const [players, setPlayers] = useState<{ id: string; name: string }[]>([]);
  const [player, setPlayer] = useState<Player | null>(null);
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [workspace, setWorkspace] = useState<Workspace | null>(null);
  const [workspacePanels, setWorkspacePanels] = useState<WorkspacePanel[]>([]);
  const [overview, setOverview] = useState<WorldOverview | null>(null);
  const [concepts, setConcepts] = useState<Concept[]>([]);
  const [stats, setStats] = useState<PlayerStat[]>([]);
  const [sessions, setSessions] = useState<QuestSession[]>([]);
  const [healthReady, setHealthReady] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [quickCreate, setQuickCreate] = useState(false);
  const [quickCapture, setQuickCapture] = useState(false);
  const [entityTarget, setEntityTarget] = useState<EntityTarget | null>(null);

  const refreshWorld = useCallback(async (id = player?.id) => {
    if (!id) return;
    try {
      const [world, conceptRows, statRows, sessionRows] = await Promise.all([
        coreClient.getWorldOverview(id), coreClient.listConcepts(id), coreClient.listPlayerStats(id), coreClient.listQuestSessions(id),
      ]);
      setPlayer(world.player);
      setOverview(world);
      setConcepts(conceptRows);
      setStats(statRows);
      setSessions(sessionRows);
      setError('');
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'The local world could not be loaded.');
    }
  }, [player?.id]);

  const seedWorkspace = useCallback(async (playerId: string, name: string, template: WorkspaceTemplate, isDefault = false) => {
    const definition = WORKSPACE_TEMPLATES.find(item => item.id === template);
    const created = await coreClient.createWorkspace(playerId, name, template, isDefault);
    const panels: WorkspacePanel[] = [];
    for (const [sortOrder, panel] of (definition?.panels ?? []).entries()) {
      panels.push(await coreClient.saveWorkspacePanel({
        playerId, workspaceId: created.id, panelType: panel.panelType, title: panel.title,
        variant: panel.variant, density: panel.density, filterStatus: panel.filterStatus ?? null,
        filterActive: panel.filterActive ?? null, filterTypeCode: panel.filterTypeCode ?? null,
        filterConceptId: null, filterRecentDays: panel.filterRecentDays ?? null, sortBy: panel.sortBy,
        itemLimit: panel.itemLimit, sortOrder, gridSpan: panel.gridSpan ?? 1, isVisible: true,
        isPinned: panel.isPinned, isCollapsed: false,
      }));
    }
    return { created, panels };
  }, []);

  const loadWorkspace = useCallback(async (playerId: string) => {
    let rows = await coreClient.listWorkspaces(playerId);
    if (rows.length === 0) {
      let legacy: unknown[] = [];
      try {
        const raw = localGet('life-rpg.dashboard-panels.v1');
        const value = raw ? JSON.parse(raw) : [];
        if (Array.isArray(value)) legacy = value;
      } catch { /* Invalid legacy layout is ignored; the world remains untouched. */ }
      const allowed = ['quests', 'skills', 'concepts', 'effects', 'activity', 'transactions', 'journal'] as const;
      const usable = legacy.filter((item): item is Record<string, unknown> => !!item && typeof item === 'object'
        && allowed.includes((item as Record<string, unknown>).id as typeof allowed[number])
        && (item as Record<string, unknown>).visible !== false).slice(0, 50);
      if (usable.length) {
        const created = await coreClient.createWorkspace(playerId, 'My Dashboard', 'custom', true);
        for (const [order, item] of usable.entries()) {
          const panelType = item.id as typeof allowed[number];
          const definition = PANEL_REGISTRY[panelType];
          const variant = definition.variants.includes(item.variant as WorkspacePanel['variant']) ? item.variant as WorkspacePanel['variant'] : definition.defaultVariant;
          const density = item.density === 'compact' ? 'compact' : 'cozy';
          const title = typeof item.title === 'string' ? item.title.trim().slice(0, 120) || undefined : undefined;
          const limit = typeof item.limit === 'number' && Number.isFinite(item.limit) ? Math.max(1, Math.min(50, Math.trunc(item.limit))) : 6;
          await coreClient.saveWorkspacePanel({
            playerId, workspaceId: created.id, panelType, title: title ?? null, variant, density,
            filterStatus: null, filterActive: null, filterTypeCode: null, filterConceptId: null, filterRecentDays: null,
            sortBy: definition.defaultSort, itemLimit: limit, sortOrder: order, gridSpan: 1, isVisible: true,
            isPinned: item.pinned === true, isCollapsed: item.collapsed === true,
          });
        }
        localRemove('life-rpg.dashboard-panels.v1');
      } else {
        await seedWorkspace(playerId, 'Overview', 'overview', true);
      }
      rows = await coreClient.listWorkspaces(playerId);
    }
    const preferred = localGet(activeWorkspaceKey(playerId));
    const selected = rows.find(item => item.id === preferred) ?? rows.find(item => item.isDefault) ?? rows[0] ?? null;
    setWorkspaces(rows);
    setWorkspace(selected);
    if (selected) {
      localSet(activeWorkspaceKey(playerId), selected.id);
      setWorkspacePanels(await coreClient.listWorkspacePanels(playerId, selected.id));
    } else {
      setWorkspacePanels([]);
    }
  }, [seedWorkspace]);

  const createWorkspace = useCallback(async (name: string, template: WorkspaceTemplate) => {
    if (!player) return;
    const result = await seedWorkspace(player.id, name, template, false);
    const rows = await coreClient.listWorkspaces(player.id);
    setWorkspaces(rows);
    setWorkspace(result.created);
    setWorkspacePanels(result.panels);
    localSet(activeWorkspaceKey(player.id), result.created.id);
    setNotice(`${name} workspace created.`);
    setError('');
  }, [player, seedWorkspace]);

  const chooseWorkspace = useCallback(async (id: string) => {
    const next = workspaces.find(item => item.id === id);
    if (!next || !player) return;
    try {
      const panels = await coreClient.listWorkspacePanels(player.id, id);
      setWorkspace(next);
      localSet(activeWorkspaceKey(player.id), id);
      setWorkspacePanels(panels);
      setError('');
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Workspace could not be opened.');
    }
  }, [player, workspaces]);

  const renameWorkspace = useCallback(async (name: string) => {
    if (!player || !workspace) return;
    await coreClient.renameWorkspace(player.id, workspace.id, name);
    const rows = await coreClient.listWorkspaces(player.id);
    setWorkspaces(rows);
    setWorkspace(rows.find(item => item.id === workspace.id) ?? workspace);
    setNotice('Workspace renamed.');
  }, [player, workspace]);

  const makeDefault = useCallback(async () => {
    if (!player || !workspace) return;
    await coreClient.setDefaultWorkspace(player.id, workspace.id);
    const rows = await coreClient.listWorkspaces(player.id);
    setWorkspaces(rows);
    setWorkspace(rows.find(item => item.id === workspace.id) ?? workspace);
    setNotice('Default workspace updated. Active view stays open.');
  }, [player, workspace]);

  const deleteWorkspace = useCallback(async () => {
    if (!player || !workspace) return;
    await coreClient.deleteWorkspace(player.id, workspace.id);
    const rows = await coreClient.listWorkspaces(player.id);
    if (rows.length === 0) throw new Error('A world must retain one workspace.');
    const next = rows.find(item => item.isDefault) ?? rows[0]!;
    const panels = await coreClient.listWorkspacePanels(player.id, next.id);
    setWorkspaces(rows);
    setWorkspace(next);
    setWorkspacePanels(panels);
    localSet(activeWorkspaceKey(player.id), next.id);
    setNotice('Workspace removed; underlying world records were not changed.');
  }, [player, workspace]);

  const copyPanels = async (source: WorkspacePanel[], playerId: string, target: Workspace) => {
    const result: WorkspacePanel[] = [];
    for (const panel of source) result.push(await coreClient.saveWorkspacePanel({
      playerId, workspaceId: target.id, panelType: panel.panelType, title: panel.title,
      variant: panel.variant, density: panel.density, filterStatus: panel.filterStatus,
      filterActive: panel.filterActive, filterTypeCode: panel.filterTypeCode, filterConceptId: panel.filterConceptId,
      filterRecentDays: panel.filterRecentDays, sortBy: panel.sortBy, itemLimit: panel.itemLimit,
      sortOrder: panel.sortOrder, gridSpan: panel.gridSpan, isVisible: panel.isVisible,
      isPinned: panel.isPinned, isCollapsed: panel.isCollapsed,
    }));
    return result;
  };

  const duplicateWorkspace = useCallback(async (name: string) => {
    if (!player || !workspace) return;
    const created = await coreClient.createWorkspace(player.id, name, 'custom', false);
    const panels = await copyPanels(workspacePanels, player.id, created);
    const rows = await coreClient.listWorkspaces(player.id);
    setWorkspaces(rows);
    setWorkspace(created);
    setWorkspacePanels(panels);
    localSet(activeWorkspaceKey(player.id), created.id);
    setNotice('Workspace duplicated with independent panel configuration.');
  }, [player, workspace, workspacePanels]);

  const importWorkspace = useCallback(async (value: ResolvedWorkspaceImport) => {
    if (!player) return;
    const result = await coreClient.importWorkspace(player.id, value.workspace.name, value.workspace.template, value.panels);
    const rows = await coreClient.listWorkspaces(player.id);
    setWorkspaces(rows);
    setWorkspace(result.workspace);
    setWorkspacePanels(result.panels);
    localSet(activeWorkspaceKey(player.id), result.workspace.id);
    setNotice(`Workspace imported. ${value.resolvedConcepts} Concept filter(s) resolved; ${value.unresolvedConcepts} left unfiltered.`);
    setError('');
  }, [player]);

  useEffect(() => {
    let live = true;
    async function boot() {
      setLoading(true);
      try {
        const [hits] = await Promise.all([coreClient.searchWorld(emptyQuery), coreClient.ping()]);
        if (!live) return;
        const choices = hits.filter((hit: SearchHit) => hit.kind === 'player').map(hit => ({ id: hit.id, name: hit.name }));
        setPlayers(choices);
        setHealthReady(true);
        const saved = localGet(ACTIVE_WORLD_KEY);
        const selected = choices.find(item => item.id === saved) ?? choices[0];
        if (selected) {
          const loaded = await coreClient.getPlayer(selected.id);
          if (live && loaded) {
            localSet(ACTIVE_WORLD_KEY, selected.id);
            setPlayer(loaded);
            const [world, conceptRows, statRows, sessionRows] = await Promise.all([
              coreClient.getWorldOverview(selected.id), coreClient.listConcepts(selected.id),
              coreClient.listPlayerStats(selected.id), coreClient.listQuestSessions(selected.id),
            ]);
            if (live) {
              setPlayer(world.player);
              setOverview(world);
              setConcepts(conceptRows);
              setStats(statRows);
              setSessions(sessionRows);
              await loadWorkspace(selected.id);
            }
          }
        }
      } catch (reason) {
        if (live) {
          setHealthReady(false);
          setError(reason instanceof Error ? reason.message : 'The local world could not be reached.');
        }
      } finally {
        if (live) setLoading(false);
      }
    }
    void boot();
    return () => { live = false; };
  }, [loadWorkspace]);

  const selectPlayer = useCallback(async (id: string) => {
    if (!id) return;
    localSet(ACTIVE_WORLD_KEY, id);
    setLoading(true);
    setError('');
    try {
      const [world, conceptRows, statRows, sessionRows] = await Promise.all([
        coreClient.getWorldOverview(id), coreClient.listConcepts(id), coreClient.listPlayerStats(id), coreClient.listQuestSessions(id),
      ]);
      setPlayer(world.player);
      setOverview(world);
      setConcepts(conceptRows);
      setStats(statRows);
      setSessions(sessionRows);
      await loadWorkspace(id);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Could not switch world.');
    } finally {
      setLoading(false);
    }
  }, [loadWorkspace]);

  const createPlayer = useCallback(async (name: string, description?: string) => {
    const created = await coreClient.createPlayer(name, description);
    localSet(ACTIVE_WORLD_KEY, created.id);
    setPlayers(rows => [...rows, { id: created.id, name: created.name }]);
    setPlayer(created);
    await refreshWorld(created.id);
    await loadWorkspace(created.id);
    setNotice(`World created for ${created.name}.`);
  }, [refreshWorld, loadWorkspace]);

  const afterMutation = useCallback(async (message?: string) => {
    await refreshWorld();
    if (message) setNotice(message);
  }, [refreshWorld]);
  const navigate = useCallback((next: AppRoute) => {
    setEntityTarget(null);
    setQuickCreate(false);
    setQuickCapture(false);
    setRoute(next);
  }, []);
  const openEntity = useCallback((kind: string, id: string) => {
    if (!id) { setRoute('explorer'); return; }
    setEntityTarget({ kind, id, returnRoute: route });
    setRoute('explorer');
  }, [route]);
  const returnToOrigin = useCallback(() => {
    if (!entityTarget) return;
    const origin = entityTarget.returnRoute;
    setEntityTarget(null);
    setRoute(origin);
  }, [entityTarget]);

  useEffect(() => { localSet(ACTIVE_ROUTE_KEY, route); }, [route]);

  return <AppShell route={route} onNavigate={navigate} player={player} players={players} onPlayerChange={id => void selectPlayer(id)}
    workspaces={workspaces} workspace={workspace} onWorkspaceChange={id => void chooseWorkspace(id)} onCreate={() => setQuickCreate(true)}
    onQuickCapture={() => setQuickCapture(true)} ready={healthReady}>
    {notice && <div className="notice-bar" role="status"><span>{notice}</span><button className="text-link" onClick={() => setNotice('')}>Dismiss</button></div>}
    {error && <div className="error-banner" role="alert"><strong>World unavailable</strong><span>{error}</span><button className="button button--small" onClick={() => void refreshWorld()}>Retry</button></div>}
    {loading && <div className="loading-state" role="status"><span className="loading-dot" />Loading your world…</div>}
    {!loading && (!player || !overview)
      ? <WorldWorkspace route={route} client={coreClient} player={null} overview={null} concepts={[]} stats={[]} sessions={[]}
        workspace={null} workspaces={[]} panels={[]} onPanelsChange={setWorkspacePanels} onCreateWorkspace={createWorkspace}
        onRenameWorkspace={renameWorkspace} onDefaultWorkspace={makeDefault} onDeleteWorkspace={deleteWorkspace}
        onDuplicateWorkspace={duplicateWorkspace} onImportWorkspace={importWorkspace} onRefresh={afterMutation}
        onCreatePlayer={createPlayer} onNavigate={navigate} onOpenEntity={openEntity} onQuickCapture={() => setQuickCapture(true)} />
      : !loading && player && overview && <WorldWorkspace route={route} client={coreClient} player={player} overview={overview} concepts={concepts}
        stats={stats} sessions={sessions} workspace={workspace} workspaces={workspaces} panels={workspacePanels} onPanelsChange={setWorkspacePanels}
        onCreateWorkspace={createWorkspace} onRenameWorkspace={renameWorkspace} onDefaultWorkspace={makeDefault}
        onDeleteWorkspace={deleteWorkspace} onDuplicateWorkspace={duplicateWorkspace} onImportWorkspace={importWorkspace}
        onRefresh={afterMutation} onCreatePlayer={createPlayer} onNavigate={navigate} onOpenEntity={openEntity}
        initialTarget={route === 'explorer' && entityTarget ? { kind: entityTarget.kind, id: entityTarget.id } : undefined}
        onReturnTo={entityTarget ? returnToOrigin : undefined} onQuickCapture={() => setQuickCapture(true)} />}
    {route === 'rules' && player && <RulePanel playerId={player.id} client={coreClient} />}
    {route === 'status' && <StatusScreen />}
    {quickCreate && <div className="modal-backdrop" role="presentation" onClick={() => setQuickCreate(false)}><section className="quick-create" role="dialog" aria-modal="true" aria-labelledby="quick-create-title" onClick={event => event.stopPropagation()}>
      <button className="quick-create__close" aria-label="Close" onClick={() => setQuickCreate(false)}>×</button><p className="eyebrow">START WITH A RECORD</p><h2 id="quick-create-title">Create in this world</h2><p className="muted">Choose what you want to add. Your entry stays local and editable.</p>
      <div className="quick-create__grid">
        <button className="quick-create__choice" onClick={() => { setQuickCreate(false); setQuickCapture(true); }}><span>✎</span>Quick note<small>Capture without leaving your current view</small></button>
        {([{ label: 'Quest', route: 'quests' }, { label: 'Skill tree', route: 'skills' }, { label: 'Concept', route: 'concepts' }, { label: 'Journal entry', route: 'journal' }] as const).map(item => <button className="quick-create__choice" key={item.route} onClick={() => navigate(item.route)}><span>＋</span>{item.label}<small>Open creation form</small></button>)}
      </div>
    </section></div>}
    {quickCapture && player && overview && <QuickCapture client={coreClient} player={player} quests={overview.quests} skills={overview.skills} concepts={concepts} sessions={sessions}
      onClose={() => setQuickCapture(false)} onSaved={async () => { await afterMutation('Your Chronicle entry is ready in this world.'); }} />}
  </AppShell>;
}
