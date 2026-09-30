import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import type { CoreClient } from '../../domain/ipc';
import type { Concept, Effect, Player, Quest, WorldOverview } from '../../domain/world';
import './EntitySurface.css';

export type SurfaceKind = 'player' | 'quest' | 'concept' | 'effect';
export type SurfaceMode = 'view' | 'edit';
export type EntitySurfaceRecord = {
  kind: SurfaceKind;
  id: string;
  mode: SurfaceMode;
  returnFocus?: HTMLElement | null;
};

type SurfaceContextValue = {
  openSurface: (kind: SurfaceKind, id: string, options?: { mode?: SurfaceMode }) => void;
  closeSurface: () => void;
  backSurface: () => void;
  stack: EntitySurfaceRecord[];
};

const SurfaceContext = createContext<SurfaceContextValue | null>(null);

export function useEntitySurface() {
  const context = useContext(SurfaceContext);
  if (!context) throw new Error('useEntitySurface must be used inside EntitySurfaceProvider');
  return context;
}

export function useOptionalEntitySurface(): SurfaceContextValue {
  return useContext(SurfaceContext) ?? { openSurface: () => {}, closeSurface: () => {}, backSurface: () => {}, stack: [] };
}

type ProviderProps = {
  children: ReactNode;
  client: CoreClient;
  player: Player | null;
  overview: WorldOverview | null;
  concepts: Concept[];
  onRefresh: () => Promise<void>;
};

export function EntitySurfaceProvider({ children, client, player, overview, concepts, onRefresh }: ProviderProps) {
  const [stack, setStack] = useState<EntitySurfaceRecord[]>([]);
  const lastTrigger = useRef<HTMLElement | null>(null);

  const openSurface = useCallback((kind: SurfaceKind, id: string, options: { mode?: SurfaceMode } = {}) => {
    if (!id) return;
    lastTrigger.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    setStack((current) => [...current, { kind, id, mode: options.mode ?? 'view', returnFocus: lastTrigger.current }]);
  }, []);

  const closeSurface = useCallback(() => {
    setStack((current) => {
      const closing = current[current.length - 1];
      window.setTimeout(() => (closing?.returnFocus ?? lastTrigger.current)?.focus(), 0);
      return current.slice(0, -1);
    });
  }, []);

  const backSurface = closeSurface;
  const value = useMemo(() => ({ openSurface, closeSurface, backSurface, stack }), [openSurface, closeSurface, backSurface, stack]);

  return <SurfaceContext.Provider value={value}>
    {children}
    {stack.length > 0 && <EntitySurfaceStack client={client} player={player} overview={overview} concepts={concepts} onRefresh={onRefresh} />}
  </SurfaceContext.Provider>;
}

function EntitySurfaceStack({ client, player, overview, concepts, onRefresh }: Omit<ProviderProps, 'children'>) {
  const { stack, closeSurface, backSurface, openSurface } = useEntitySurface();
  const surface = stack[stack.length - 1]!;
  const [editing, setEditing] = useState(surface.mode === 'edit');
  const [copyState, setCopyState] = useState('Copy');
  const headingRef = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    setEditing(surface.mode === 'edit');
    setCopyState('Copy');
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    headingRef.current?.focus();
    return () => { document.body.style.overflow = previousOverflow; };
  }, [surface.kind, surface.id, surface.mode]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') { event.preventDefault(); closeSurface(); }
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [closeSurface]);

  const record = findRecord(surface.kind, surface.id, player, overview, concepts);
  const title = recordTitle(surface.kind, record);
  const copy = async (value: string) => {
    try { await navigator.clipboard?.writeText(value); setCopyState('Copied'); window.setTimeout(() => setCopyState('Copy'), 1400); } catch { setCopyState('Copy unavailable'); }
  };

  return <div className="entity-surface-layer" data-testid="entity-surface-layer">
    <button className="entity-surface-backdrop" aria-label="Close entity surface" onClick={closeSurface} />
    <aside className="entity-surface" role="dialog" aria-modal="true" aria-labelledby="entity-surface-title">
      <header className="entity-surface__header">
        <div className="entity-surface__heading">
          {stack.length > 1 && <button className="text-link" onClick={backSurface}>← Back to {recordTitle(stack[stack.length - 2]!.kind, findRecord(stack[stack.length - 2]!.kind, stack[stack.length - 2]!.id, player, overview, concepts))}</button>}
          <span className="eyebrow">{surface.kind} · contextual surface</span>
          <h2 id="entity-surface-title" tabIndex={-1} ref={headingRef}>{title}</h2>
          <p>Inspect this record without leaving your current workspace.</p>
        </div>
        <button className="entity-surface__close" aria-label="Close entity surface" onClick={closeSurface}>×</button>
      </header>
      <div className="entity-surface__toolbar">
        <span className="type-pill">{surface.mode === 'edit' || editing ? 'Editing' : 'Details'}</span>
        <div>
          {surface.kind === 'player' && <button className="button button--small button--cyan" onClick={() => setEditing((value) => !value)}>{editing ? 'Cancel edit' : 'Edit'}</button>}
          {!editing && <button className="button button--small" onClick={() => void copy(JSON.stringify(record, null, 2))}>{copyState}</button>}
        </div>
      </div>
      <div className="entity-surface__body">
        {surface.kind === 'player' && player && <PlayerSurface player={player} client={client} editing={editing} setEditing={setEditing} onRefresh={onRefresh} />}
        {surface.kind === 'quest' && <QuestSurface quest={record as Quest | null} concepts={concepts} openSurface={openSurface} />}
        {surface.kind === 'concept' && <ConceptSurface concept={record as Concept | null} />}
        {surface.kind === 'effect' && <EffectSurface effect={record as Effect | null} />}
        {!record && <p className="muted">This record is no longer available in the current world.</p>}
      </div>
    </aside>
  </div>;
}

function findRecord(kind: SurfaceKind, id: string, player: Player | null, overview: WorldOverview | null, concepts: Concept[]) {
  if (kind === 'player') return player?.id === id ? player : null;
  if (kind === 'quest') return overview?.quests.find((item) => item.id === id) ?? null;
  if (kind === 'concept') return concepts.find((item) => item.id === id) ?? null;
  return overview?.effects.find((item) => item.id === id) ?? null;
}
function recordTitle(kind: SurfaceKind, record: unknown) {
  if (!record) return 'Unknown record';
  if (kind === 'player') return (record as Player).name;
  if (kind === 'quest') return (record as Quest).title;
  return (record as Concept | Effect).name;
}

function Field({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) { return <div className="entity-field"><span>{label}</span><strong className={mono ? 'selectable' : undefined}>{value || '—'}</strong></div>; }
function EntitySection({ eyebrow, title, children }: { eyebrow: string; title: string; children: ReactNode }) { return <section className="entity-section"><div><span className="eyebrow">{eyebrow}</span><h3>{title}</h3></div>{children}</section>; }

function PlayerSurface({ player, client, editing, setEditing, onRefresh }: { player: Player; client: CoreClient; editing: boolean; setEditing: (value: boolean) => void; onRefresh: () => Promise<void> }) {
  const [level, setLevel] = useState(String(player.level));
  const [levelName, setLevelName] = useState(player.levelName ?? '');
  const [progressionLabel, setProgressionLabel] = useState(player.progressionLabel ?? '');
  const [message, setMessage] = useState('');
  useEffect(() => { setLevel(String(player.level)); setLevelName(player.levelName ?? ''); setProgressionLabel(player.progressionLabel ?? ''); }, [player]);
  async function save() {
    await client.setPlayerProgression(player.id, Number(level), levelName || undefined, progressionLabel || undefined);
    await onRefresh(); setEditing(false); setMessage('Player details saved.');
  }
  return <>
    <EntitySection eyebrow="Identity" title="Player record"><div className="entity-grid"><Field label="Name" value={player.name} /><Field label="Level" value={String(player.level)} /><Field label="Experience" value={player.currentXp.toLocaleString()} /><Field label="Record ID" value={player.id} mono /></div><p className="entity-copy">{player.description || 'Your Player record is the authored centre of this world.'}</p></EntitySection>
    {editing && <EntitySection eyebrow="Edit mode" title="Player-authored progression"><div className="entity-form"><label>Level<input type="number" min="1" value={level} onChange={(event) => setLevel(event.target.value)} /></label><label>Level title<input value={levelName} onChange={(event) => setLevelName(event.target.value)} /></label><label>Progression note<textarea rows={3} value={progressionLabel} onChange={(event) => setProgressionLabel(event.target.value)} /></label><div><button className="button button--primary" onClick={() => void save()}>Save</button><button className="button button--quiet" onClick={() => setEditing(false)}>Cancel</button></div></div></EntitySection>}
    {message && <p className="inline-feedback" role="status">{message}</p>}
  </>;
}
function QuestSurface({ quest, concepts, openSurface }: { quest: Quest | null; concepts: Concept[]; openSurface: SurfaceContextValue['openSurface'] }) {
  if (!quest) return null;
  return <><EntitySection eyebrow="Identity" title="Quest details"><div className="entity-grid"><Field label="Status" value={quest.status.replaceAll('_', ' ')} /><Field label="Progress" value={`${quest.progress}%`} /><Field label="Type" value={quest.typeCode.replaceAll('_', ' ')} /><Field label="Record ID" value={quest.id} mono /></div><p className="entity-copy">{quest.description || 'No description has been recorded for this Quest.'}</p></EntitySection><EntitySection eyebrow="Related records" title="Stay in context">{concepts.length > 0 ? <button className="entity-relation" onClick={() => openSurface('concept', concepts[0]!.id)}><span>Concept</span><strong>{concepts[0]!.name}</strong><small>Open nested surface →</small></button> : <p className="muted">No related Concepts are available yet.</p>}</EntitySection></>;
}
function ConceptSurface({ concept }: { concept: Concept | null }) { return concept ? <><EntitySection eyebrow="Identity" title="Concept details"><div className="entity-grid"><Field label="Type" value={concept.typeCode.replaceAll('_', ' ')} /><Field label="Status" value={concept.isActive ? 'Active' : 'Inactive'} /><Field label="Record ID" value={concept.id} mono /></div><p className="entity-copy">{concept.description || 'This Concept is ready to collect meaning and relationships.'}</p></EntitySection><EntitySection eyebrow="Context" title="Player-authored world model"><p className="muted">Concepts remain inspectable here so you can return to the surface that opened them.</p></EntitySection></> : null; }
function EffectSurface({ effect }: { effect: Effect | null }) { return effect ? <EntitySection eyebrow="Current state" title="Effect details"><div className="entity-grid"><Field label="Status" value={effect.deactivatedAt ? 'Inactive' : 'Active'} /><Field label="Type" value={effect.typeCode.replaceAll('_', ' ')} /><Field label="Intensity" value={String(effect.intensity)} /><Field label="Record ID" value={effect.id} mono /></div><p className="entity-copy">{effect.description || 'No description has been recorded for this Effect.'}</p></EntitySection> : null; }
