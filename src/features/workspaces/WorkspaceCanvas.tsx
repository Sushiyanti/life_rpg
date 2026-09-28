import { useState, type ReactNode } from 'react';
import type { CoreClient } from '../../domain/ipc';
import type {
  Concept,
  ConceptProgressTrack,
  Player,
  PlayerStat,
  QuestSession,
  Workspace,
  WorkspacePanel,
  WorkspaceTemplate,
  WorldOverview,
} from '../../domain/world';
import type { AppRoute } from '../../app/AppShell';
import { WorkspaceBuilder } from './WorkspaceBuilder';
import { WorkspacePanelView } from './WorkspacePanels';

type Props = {
  client: CoreClient;
  player: Player;
  overview: WorldOverview;
  concepts: Concept[];
  stats: PlayerStat[];
  sessions: QuestSession[];
  tracks: Record<string, ConceptProgressTrack[]>;
  workspace: Workspace | null;
  workspaces: Workspace[];
  panels: WorkspacePanel[];
  panelAllowedIds: Record<string, Set<string>>;
  onPanelsChange: (panels: WorkspacePanel[]) => void;
  onCreate: (name: string, template: WorkspaceTemplate) => Promise<void>;
  onRename: (name: string) => Promise<void>;
  onDefault: () => Promise<void>;
  onDelete: () => Promise<void>;
  onDuplicate: (name: string) => Promise<void>;
  onImport: (value: import('./workspaceTransfer').ResolvedWorkspaceImport) => Promise<void>;
  isVisible: (kind: string, id: string) => boolean;
  onVisibility: (kind: string, id: string) => Promise<void>;
  onNavigate: (route: AppRoute) => void;
  onOpenEntity: (kind: string, id: string) => void;
};

export function WorkspaceCanvas(props: Props) {
  const [customizing, setCustomizing] = useState(false);
  const [error, setError] = useState('');
  const { client, player, overview, concepts, stats, sessions, tracks, workspace, workspaces, panels,
    panelAllowedIds, onPanelsChange, onCreate, onRename, onDefault, onDelete, onDuplicate, onImport,
    isVisible, onVisibility, onNavigate, onOpenEntity } = props;

  const updateCollapse = async (panel: WorkspacePanel) => {
    if (!workspace) return;
    setError('');
    try {
      const saved = await client.saveWorkspacePanel({
        playerId: player.id, workspaceId: workspace.id, panelId: panel.id,
        panelType: panel.panelType, title: panel.title, variant: panel.variant, density: panel.density,
        filterStatus: panel.filterStatus, filterActive: panel.filterActive, filterTypeCode: panel.filterTypeCode,
        filterConceptId: panel.filterConceptId, filterTagIds: panel.filterTagIds, filterTagMatch: panel.filterTagMatch, filterRecentDays: panel.filterRecentDays,
        filterTimelineCategory: panel.filterTimelineCategory, filterTimelineEntityKind: panel.filterTimelineEntityKind,
        filterTimelineEntityId: panel.filterTimelineEntityId, filterTimelineFrom: panel.filterTimelineFrom,
        filterTimelineThrough: panel.filterTimelineThrough, sortBy: panel.sortBy,
        itemLimit: panel.itemLimit, sortOrder: panel.sortOrder, gridSpan: panel.gridSpan,
        isVisible: panel.isVisible, isPinned: panel.isPinned, isCollapsed: !panel.isCollapsed,
      });
      onPanelsChange(panels.map(item => item.id === saved.id ? saved : item));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Panel state could not be saved.');
    }
  };

  return <section className="player-workspace" aria-label="Your configurable workspace">
    <div className="dashboard-toolbar">
      <div><p className="eyebrow">YOUR WORKSPACE · {workspace?.template.toUpperCase() ?? 'PERSISTENT'}</p><h3>{workspace?.name ?? 'Today at a glance'}</h3></div>
      {workspace && <button className="button button--quiet" aria-expanded={customizing} onClick={() => setCustomizing(value => !value)}>{customizing ? 'Close builder' : 'Customize workspace'} <span aria-hidden="true">⌘</span></button>}
    </div>
    {customizing && workspace && <WorkspaceBuilder client={client} playerId={player.id} workspace={workspace} workspaces={workspaces} panels={panels} concepts={concepts} onPanelsChange={onPanelsChange} onCreate={onCreate} onRename={onRename} onDefault={onDefault} onDelete={onDelete} onDuplicate={onDuplicate} onImport={onImport} />}
    {panels.some(panel => panel.isVisible) ? <div className="widget-grid">{panels.filter(panel => panel.isVisible).slice().sort((a, b) => Number(b.isPinned) - Number(a.isPinned) || a.sortOrder - b.sortOrder).map(panel => <WidgetCard key={panel.id} panel={panel} onCollapse={() => void updateCollapse(panel)}>
      <WorkspacePanelView client={client} panel={panel} data={{ player, overview, concepts, stats, sessions, tracks }} allowedIds={panelAllowedIds[panel.id]} isVisible={isVisible} onVisibility={onVisibility} onNavigate={onNavigate} onOpenEntity={onOpenEntity} />
    </WidgetCard>)}</div> : <div className="empty-card"><span className="empty-card__mark">◈</span><strong>No visible panels</strong><p>Choose Customize workspace to add a panel or reveal one you've hidden.</p><button className="button button--small" onClick={() => setCustomizing(true)}>Customize workspace</button></div>}
    {error && <p className="inline-feedback" role="alert">{error}</p>}
  </section>;
}

function WidgetCard({ panel, children, onCollapse }: { panel: WorkspacePanel; children: ReactNode; onCollapse: () => void }) {
  return <section className={`widget-card widget-card--${panel.variant} widget-card--${panel.density} ${panel.isPinned ? 'is-pinned' : ''} ${panel.gridSpan === 2 ? 'widget-card--wide' : ''}`} style={{ gridColumn: `span ${panel.gridSpan}` }}>
    <header className="widget-card__head"><div><span className="widget-card__source">{panel.isPinned ? 'PINNED PANEL' : 'WORLD PANEL'} · {panel.panelType}</span><h3>{panel.title || panel.panelType}</h3></div><button className="button button--icon button--quiet" aria-label={panel.isCollapsed ? 'Expand panel' : 'Collapse panel'} onClick={onCollapse}>{panel.isCollapsed ? '＋' : '−'}</button></header>
    {!panel.isCollapsed && <div className="widget-card__body">{children}</div>}
  </section>;
}
