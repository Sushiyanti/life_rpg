import { useEffect, useMemo, useState, type FormEvent } from 'react';
import type { CoreClient } from '../../domain/ipc';
import type { AppRoute } from '../../app/AppShell';
import type {
  Concept,
  ConceptProgressTrack,
  Player,
  PlayerSnapshot,
  PlayerStat,
  QuestSession,
  StatDefinition,
  WorldOverview,
} from '../../domain/world';
import { buildWorldTimeline } from './worldTimeline';
import { effectLifecycleAt } from './effectLifecycle';
import './PlayerCharacter.css';

type Props = {
  client: CoreClient;
  player: Player;
  overview: WorldOverview;
  concepts: Concept[];
  stats: PlayerStat[];
  sessions: QuestSession[];
  tracks: Record<string, ConceptProgressTrack[]>;
  onRefresh: () => Promise<void>;
  onNavigate: (route: AppRoute) => void;
  onOpenEntity: (kind: string, id: string) => void;
  onQuickCapture: () => void;
};

const formatDate = (value: string) => new Date(value).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
const pretty = (value: string) => value.replaceAll('_', ' ');

export function PlayerCharacter({
  client, player, overview, concepts, stats, sessions, tracks, onRefresh, onNavigate, onOpenEntity, onQuickCapture,
}: Props) {
  const [definitions, setDefinitions] = useState<StatDefinition[]>([]);
  const [snapshots, setSnapshots] = useState<PlayerSnapshot[]>([]);
  const [editing, setEditing] = useState(false);
  const [level, setLevel] = useState(String(player.level));
  const [levelName, setLevelName] = useState(player.levelName ?? '');
  const [label, setLabel] = useState(player.progressionLabel ?? '');
  const [statCode, setStatCode] = useState('focus');
  const [statName, setStatName] = useState('Focus');
  const [statValue, setStatValue] = useState('');
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  const [hiddenCount, setHiddenCount] = useState(0);
  const [hiddenRecords, setHiddenRecords] = useState<Set<string>>(new Set());
  const [showAllActivity, setShowAllActivity] = useState(false);
  const [finishingSessionId, setFinishingSessionId] = useState<string | null>(null);
  const [sessionDrafts, setSessionDrafts] = useState<Record<string, { result: string; notes: string }>>({});
  const [sessionAnchorNames, setSessionAnchorNames] = useState<Record<string, string>>({});

  useEffect(() => {
    let live = true;
    const stageIds = [...new Set(sessions.flatMap(session => session.stageId ? [session.stageId] : []))];
    const branchIds = [...new Set(sessions.flatMap(session => session.branchId ? [session.branchId] : []))];
    void Promise.all([
      ...stageIds.map(async id => [id, await client.getQuestStage(id).catch(() => null)] as const),
      ...branchIds.map(async id => [id, await client.getQuestBranch(id).catch(() => null)] as const),
    ]).then(rows => {
      if (!live) return;
      setSessionAnchorNames(Object.fromEntries(rows.flatMap(([id, value]) => value ? [[id, value.title]] : [])));
    });
    return () => { live = false; };
  }, [client, sessions]);

  useEffect(() => {
    let live = true;
    void Promise.all([
      client.listStatDefinitions(),
      client.listPlayerSnapshots(player.id),
      client.listPresentationPreferences(player.id, 'dashboard'),
    ]).then(([definitionsRows, snapshotRows, preferences]) => {
      if (!live) return;
      setDefinitions(definitionsRows);
      setSnapshots(snapshotRows);
      if (definitionsRows[0]) {
        setStatCode(definitionsRows[0].code);
        setStatName(definitionsRows[0].name);
      }
      const hidden = new Set(preferences.filter(item => !item.isVisible).map(item => `${item.entityKind}:${item.entityId}`));
      setHiddenRecords(hidden);
      setHiddenCount(
        overview.quests.filter(item => hidden.has(`quest:${item.id}`)).length
        + overview.skillTrees.filter(item => hidden.has(`skill_tree:${item.id}`)).length
        + overview.skills.filter(item => hidden.has(`skill:${item.id}`)).length
        + concepts.filter(item => hidden.has(`concept:${item.id}`)).length
        + overview.effects.filter(item => hidden.has(`effect:${item.id}`)).length
        + sessions.filter(item => hidden.has(`quest_session:${item.id}`)).length
        + overview.narratives.filter(item => hidden.has(`narrative_entry:${item.id}`)).length,
      );
    }).catch(reason => {
      if (live) setError(reason instanceof Error ? reason.message : 'Player state details could not be loaded.');
    });
    return () => { live = false; };
  }, [client, player.id, overview.quests, overview.skills, overview.effects, overview.narratives, concepts, sessions]);

  useEffect(() => {
    setLevel(String(player.level));
    setLevelName(player.levelName ?? '');
    setLabel(player.progressionLabel ?? '');
  }, [player.id, player.level, player.levelName, player.progressionLabel]);

  const events = useMemo(() => buildWorldTimeline({ player, overview, concepts, stats, sessions, tracks })
    .filter(event => !hiddenRecords.has(`${event.recordKind}:${event.recordId}`)), [player, overview, concepts, stats, sessions, tracks, hiddenRecords]);
  const currentQuests = overview.quests.filter(quest => (quest.status === 'open' || quest.status === 'active') && !hiddenRecords.has(`quest:${quest.id}`));
  const activeSessions = sessions.filter(session => session.status === 'in_progress' && !hiddenRecords.has(`quest_session:${session.id}`));
  const visibleEffects = overview.effects.filter(effect => !hiddenRecords.has(`effect:${effect.id}`));
  const activeConceptProgress = concepts.flatMap(concept => (tracks[concept.id] ?? [])
    .filter(track => track.isActive && !hiddenRecords.has(`concept:${concept.id}`))
    .map(track => ({ concept, track })));

  async function refreshRecords() {
    await onRefresh();
    setSnapshots(await client.listPlayerSnapshots(player.id));
  }

  async function perform(action: () => Promise<unknown>, success: string) {
    setSaving(true);
    setMessage('');
    setError('');
    try {
      await action();
      await refreshRecords();
      setMessage(success);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'The requested action could not be completed.');
    } finally {
      setSaving(false);
    }
  }

  async function save(e: FormEvent, action: () => Promise<void>, success: string) {
    e.preventDefault();
    await perform(action, success);
  }

  async function finishSession(sessionId: string, status: 'completed' | 'interrupted') {
    const draft = sessionDrafts[sessionId] ?? { result: '', notes: '' };
    setSaving(true);
    setMessage('');
    setError('');
    try {
      await client.finishQuestSession(sessionId, status, draft.result.trim() || undefined, draft.notes.trim() || undefined);
      await refreshRecords();
      setFinishingSessionId(null);
      setSessionDrafts(current => { const next = { ...current }; delete next[sessionId]; return next; });
      setMessage(status === 'completed' ? 'Session ended. Duration comes from its actual recorded timestamps.' : 'Session marked interrupted.');
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'The Session could not be updated. Your result and notes remain in this form.');
    } finally {
      setSaving(false);
    }
  }

  const contextName = (session: QuestSession) => [
    session.questId && `Quest · ${overview.quests.find(item => item.id === session.questId)?.title ?? session.questId}`,
    session.stageId && `Stage · ${sessionAnchorNames[session.stageId] ?? session.stageId}`,
    session.branchId && `Branch · ${sessionAnchorNames[session.branchId] ?? session.branchId}`,
    session.skillId && `Skill · ${overview.skills.find(item => item.id === session.skillId)?.name ?? session.skillId}`,
    session.conceptId && `Concept · ${concepts.find(item => item.id === session.conceptId)?.name ?? session.conceptId}`,
  ].filter((value): value is string => Boolean(value)).join(' · ') || 'Recorded activity';

  return <div className="workspace-page player-hub">
    <header className="player-hub__heading">
      <div>
        <p className="eyebrow">PLAYER HUB · STATE AS CURRENTLY RECORDED</p>
        <h2>Your world, {player.name}</h2>
        <p>What follows is what this local world knows now. Life between records is not inferred.</p>
      </div>
      <div className="player-hub__heading-actions">
        <button className="button button--quiet" onClick={() => onNavigate('dashboard')}>Open workspace</button>
        <button className="button button--primary" onClick={onQuickCapture}>Quick capture</button>
      </div>
    </header>

    {hiddenCount > 0 && <div className="hidden-awareness" role="status">
      <span aria-hidden="true">◌</span>
      <p><strong>{hiddenCount} item{hiddenCount === 1 ? '' : 's'} hidden in this view.</strong> Hidden items remain in your world; nothing is deleted.</p>
      <button className="text-link" onClick={() => onNavigate('explorer')}>Review in Explorer →</button>
    </div>}

    <section className="player-hub__state" aria-label="Current Player state">
      <div className="player-hub__identity">
        <span className="player-hub__avatar" aria-hidden="true">{player.name.slice(0, 1).toUpperCase()}</span>
        <div><p className="eyebrow">PLAYER STATE</p><h3>{player.name}</h3><small>World created {formatDate(player.createdAt)}</small></div>
      </div>
      <div className="player-hub__level">
        <span>PLAYER-AUTHORED LEVEL</span><strong>{player.level}</strong>
        <small>{player.levelName || player.progressionLabel || 'No progression label set'}</small>
        <button className="text-link" onClick={() => setEditing(value => !value)}>{editing ? 'Close editor' : 'Edit progression'}</button>
      </div>
      <div className="player-hub__xp">
        <span>RECORDED XP</span><strong>{player.currentXp.toLocaleString()}</strong>
        <small>Informational only · does not calculate level</small>
      </div>
    </section>

    {editing && <form className="inline-editor" onSubmit={e => void save(e, async () => {
      await client.setPlayerProgression(player.id, Number(level), levelName || undefined, label || undefined);
      setEditing(false);
    }, 'Your authored Player progression was saved. XP remains independent.')}>
      <strong>Set your Player progression</strong>
      <p className="muted">The Player remains the authority over their level; no formula or elapsed time changes it.</p>
      <div className="form-grid form-grid--four">
        <label>Level<input aria-label="Level" type="number" required min="1" step="1" value={level} onChange={e => setLevel(e.target.value)} /></label>
        <label>Level title<input value={levelName} onChange={e => setLevelName(e.target.value)} /></label>
        <label>Progression note<input value={label} onChange={e => setLabel(e.target.value)} /></label>
        <button className="button button--primary" disabled={saving}>Save authored level</button>
      </div>
    </form>}

    <section className="player-hub__grid" aria-label="Current world context">
      <section className="surface-card player-hub__card">
        <div className="surface-card__heading"><div><p className="eyebrow">CURRENT OBJECTIVES</p><h3>Quests</h3></div><button className="text-link" onClick={() => onNavigate('quests')}>Quest log →</button></div>
        {currentQuests.length === 0 ? <div className="player-hub__empty"><strong>No open Quests</strong><p>Completed records remain in History. Start something only when you choose to.</p><button className="button button--small" onClick={() => onNavigate('quests')}>Explore Quests</button></div>
          : <div className="player-hub__list">{currentQuests.slice(0, 4).map(quest => <article className="player-hub__row" key={quest.id}>
            <button className="player-hub__open" onClick={() => onOpenEntity('quest', quest.id)}><span className={`type-pill type-pill--${quest.status}`}>{pretty(quest.status)}</span><strong>{quest.title}</strong><small>{quest.progress}% recorded progress{quest.description ? ` · ${quest.description}` : ''}</small></button>
            <div className="player-hub__row-actions">
              {quest.status === 'open' && <button className="button button--small button--primary" disabled={saving} onClick={() => void perform(() => client.startQuest(quest.id), 'Quest started. Begin a Session when real activity begins.')}>Start Quest</button>}
              {quest.status === 'active' && <><button className="button button--small" disabled={saving} onClick={() => void perform(() => client.startQuestSession(player.id, { questId: quest.id }), 'A Quest Session started at the recorded current time.')}>Start Session</button><button className="button button--small button--quiet" disabled={saving} onClick={() => void perform(() => client.completeQuest(quest.id), 'Quest completed explicitly.')}>Complete</button></>}
            </div>
          </article>)}</div>}
      </section>

      <section className="surface-card player-hub__card">
        <div className="surface-card__heading"><div><p className="eyebrow">REAL ACTIVITY</p><h3>Active Sessions</h3></div><span className="live-count">{activeSessions.length} active</span></div>
        {activeSessions.length === 0 ? <div className="player-hub__empty"><strong>No active Sessions</strong><p>Sessions exist only when you choose to record real activity.</p><button className="text-link" onClick={() => onNavigate('quests')}>Choose a Quest or Skill →</button></div>
          : <div className="player-hub__list">{activeSessions.map(session => { const draft = sessionDrafts[session.id] ?? { result: '', notes: '' }; return <article className="player-hub__session" key={session.id}>
            <span className="live-dot" aria-hidden="true" />
            <div className="player-hub__session-copy"><strong>{contextName(session)}</strong><small>Started {formatDate(session.startedAt)} · active as last recorded</small></div>
            <div className="player-hub__row-actions"><button className="button button--small" disabled={saving} onClick={() => setFinishingSessionId(finishingSessionId === session.id ? null : session.id)}>End or record outcome</button></div>
            {finishingSessionId === session.id && <form className="session-outcome-form" onSubmit={event => void save(event, async () => { await client.finishQuestSession(session.id, 'completed', draft.result.trim() || undefined, draft.notes.trim() || undefined); setFinishingSessionId(null); setSessionDrafts(current => { const next = { ...current }; delete next[session.id]; return next; }); }, 'Session completed with your outcome and notes.') }><label>Result <span className="muted">optional</span><input maxLength={512} value={draft.result} onChange={event => setSessionDrafts(current => ({ ...current, [session.id]: { ...draft, result: event.target.value } }))} placeholder="What came out of this Session?" /></label><label>Notes <span className="muted">optional</span><textarea rows={2} maxLength={4000} value={draft.notes} onChange={event => setSessionDrafts(current => ({ ...current, [session.id]: { ...draft, notes: event.target.value } }))} placeholder="Keep only details you choose to record." /></label><div><button className="button button--small button--primary" disabled={saving}>Record completion</button><button className="button button--small button--quiet" type="button" disabled={saving} onClick={() => void finishSession(session.id, 'interrupted')}>Mark interrupted</button><button className="text-link" type="button" disabled={saving} onClick={() => setFinishingSessionId(null)}>Cancel</button></div></form>}
          </article>; })}</div>}
      </section>

      <section className="surface-card player-hub__card">
        <div className="surface-card__heading"><div><p className="eyebrow">DERIVED FROM RECORDED TIMESTAMPS</p><h3>Effects</h3></div><button className="text-link" onClick={() => onNavigate('effects')}>Manage all →</button></div>
        {visibleEffects.length === 0 ? <div className="player-hub__empty"><strong>No visible Effects recorded</strong><p>Expiry is displayed from its recorded timestamp. Time passing never writes a deactivation.</p></div>
          : <div className="player-hub__list">{visibleEffects.slice().sort((a,b)=>Date.parse(b.startedAt)-Date.parse(a.startedAt)).slice(0,6).map(effect => {
            const lifecycle = effectLifecycleAt(effect);
            return <article className="player-hub__effect" key={effect.id}><span className={`effect-indicator ${lifecycle==='expired'?'is-expired':''}`} aria-hidden="true">✦</span><button className="player-hub__open" onClick={() => onOpenEntity('effect', effect.id)}><span className={`type-pill effect-state effect-state--${lifecycle}`}>{pretty(lifecycle)}</span><strong>{effect.name}</strong><small>{effect.targetKind === 'concept' ? `Concept · ${concepts.find(item => item.id === effect.targetConceptId)?.name ?? 'linked'}` : 'Player'} · intensity {effect.intensity}</small><small>{effect.expiresAt ? `Recorded expiry ${formatDate(effect.expiresAt)}` : 'No expiry recorded · indefinite/manual'}</small>{effect.deactivatedAt&&<small>Manually deactivated {formatDate(effect.deactivatedAt)}</small>}</button>{!effect.deactivatedAt&&lifecycle==='active'&&<button className="button button--small button--quiet" disabled={saving} onClick={() => void perform(() => client.deactivateEffect(player.id,effect.id), 'Effect manually deactivated; the historical event was recorded.')}>Deactivate</button>}</article>;
          })}</div>}
      </section>

      <section className="surface-card player-hub__card">
        <div className="surface-card__heading"><div><p className="eyebrow">AUTHORED ATTRIBUTES</p><h3>Player stats</h3></div><button className="text-link" onClick={() => setEditing(value => !value)}>Progression ↗</button></div>
        {stats.length === 0 ? <div className="player-hub__empty"><strong>No stats recorded yet</strong><p>Only the attributes you choose to define appear here.</p></div>
          : <div className="player-hub__stats">{stats.map(stat => {
            const definition = definitions.find(item => item.code === stat.statCode);
            return <div className="player-hub__stat" key={stat.statCode}><span>{definition?.name ?? pretty(stat.statCode)}</span><strong>{stat.currentValue}{definition?.unit ? ` ${definition.unit}` : ''}</strong></div>;
          })}</div>}
        <form className="player-hub__stat-form" onSubmit={e => void save(e, async () => {
          if (!statCode.trim() || !Number.isFinite(Number(statValue))) return;
          if (!definitions.some(item => item.code === statCode)) {
            await client.defineStat(statCode, statName);
            setDefinitions(await client.listStatDefinitions());
          }
          await client.setPlayerStat(player.id, statCode, Number(statValue));
          setStatValue('');
        }, 'Player stat updated.') }>
          <label>Stat<select value={statCode} onChange={e => {
            const definition = definitions.find(item => item.code === e.target.value);
            setStatCode(e.target.value);
            if (definition) setStatName(definition.name);
          }}>
            {definitions.map(definition => <option key={definition.code} value={definition.code}>{definition.name}</option>)}
            {definitions.length === 0 && <option value="focus">Focus</option>}
          </select></label>
          {definitions.length === 0 && <label>Display name<input value={statName} onChange={e => setStatName(e.target.value)} required /></label>}
          <label>New value<input type="number" step="any" required value={statValue} onChange={e => setStatValue(e.target.value)} placeholder="Set a value" /></label>
          <button className="button button--small button--primary" disabled={saving || !statValue}>Update</button>
        </form>
      </section>

      <section className="surface-card player-hub__card">
        <div className="surface-card__heading"><div><p className="eyebrow">CONCEPTS IN FOCUS</p><h3>Progress tracks</h3></div><button className="text-link" onClick={() => onNavigate('concepts')}>All Concepts →</button></div>
        {activeConceptProgress.length === 0 ? <div className="player-hub__empty"><strong>No active progress tracks</strong><p>Progress appears here when a Concept track has been explicitly recorded.</p></div>
          : <div className="player-hub__list">{activeConceptProgress.slice(0, 6).map(({ concept, track }) => <button className="player-hub__concept" key={`${concept.id}:${track.trackCode}`} onClick={() => onOpenEntity('concept', concept.id)}><span><strong>{concept.name}</strong><small>{pretty(track.trackCode)} · {track.control === 'manual' ? 'player-authored' : 'rule-controlled'}</small></span><strong>{track.currentValue}{['confidence', 'progress'].includes(track.trackCode) ? '%' : ''}{track.level === null ? '' : ` · L${track.level}`}</strong><i aria-hidden="true"><b style={{ width: `${Math.max(0, Math.min(100, track.currentValue))}%` }} /></i></button>)}</div>}
      </section>

      <section className="surface-card player-hub__card">
        <div className="surface-card__heading"><div><p className="eyebrow">PLAYER-AUTHORED DEVELOPMENT</p><h3>Active Skills</h3></div><button className="text-link" onClick={() => onNavigate('skills')}>Skill trees →</button></div>
        {overview.skills.filter(skill => skill.status === 'active' && !hiddenRecords.has(`skill:${skill.id}`)).length === 0 ? <div className="player-hub__empty"><strong>No active Skills recorded</strong><p>Levels stay player-authored; practice time is recorded activity, not an automatic level formula.</p><button className="button button--small" onClick={() => onNavigate('skills')}>Open Skills</button></div>
          : <div className="player-hub__list">{overview.skills.filter(skill => skill.status === 'active' && !hiddenRecords.has(`skill:${skill.id}`)).slice(0, 5).map(skill => <button className="player-hub__skill" key={skill.id} onClick={() => onOpenEntity('skill', skill.id)}><span><strong>{skill.name}</strong><small>{overview.skillTrees.find(tree => tree.id === skill.skillTreeId)?.name ?? 'Skill Tree'} · {skill.investedMinutes} minutes recorded</small></span><strong>Level {skill.level}{skill.levelName ? ` · ${skill.levelName}` : ''}</strong></button>)}</div>}
      </section>

      <section className="surface-card player-hub__card">
        <div className="surface-card__heading"><div><p className="eyebrow">RECENT CONTENT</p><h3>Notes & guidance</h3></div><button className="text-link" onClick={() => onNavigate('journal')}>Chronicle →</button></div>
        {overview.narratives.filter(note => !hiddenRecords.has(`narrative_entry:${note.id}`)).length === 0 ? <div className="player-hub__empty"><strong>No visible content captured yet</strong><p>Keep a note in your own words; hidden entries remain in your world and nothing is generated for you.</p><button className="button button--small" onClick={onQuickCapture}>Capture a note</button></div>
          : <div className="player-hub__list">{overview.narratives.filter(note => !hiddenRecords.has(`narrative_entry:${note.id}`)).slice().sort((a, b) => Date.parse(b.createdAt) - Date.parse(a.createdAt)).slice(0, 3).map(note => <button className="player-hub__note" key={note.id} onClick={() => onOpenEntity('narrative_entry', note.id)}><span className="type-pill">{pretty(note.kind)}</span><strong>{note.title}</strong><small>{formatDate(note.createdAt)}</small><p>{note.content}</p></button>)}</div>}
      </section>
    </section>

    <section className="surface-card player-hub__timeline">
      <div className="surface-card__heading"><div><p className="eyebrow">WHAT HAPPENED · PERSISTED TIMESTAMPS</p><h3>Recent activity</h3></div><div className="player-hub__timeline-actions"><span>{events.length} recorded items</span><button className="text-link" onClick={() => onNavigate('history')}>History & recovery →</button></div></div>
      <p className="muted">This is a history of records, not an estimate of time spent or days that were not recorded.</p>
      {events.length === 0 ? <div className="player-hub__empty"><strong>No recorded activity yet</strong><p>Start with a Quest, a practice session, or a note when it reflects something you actually did.</p></div>
        : <div className="player-hub__timeline-list">{events.slice(0, showAllActivity ? 18 : 7).map(event => <article className="player-hub__timeline-row" key={event.id}><span className="player-hub__timeline-mark" aria-hidden="true" /><button className="player-hub__timeline-open" onClick={() => onOpenEntity(event.recordKind, event.recordId)}><strong>{event.title}</strong><small>{event.detail}</small></button><time dateTime={event.at}>{formatDate(event.at)}</time></article>)}</div>}
      {events.length > 7 && <button className="button button--small button--quiet" onClick={() => setShowAllActivity(value => !value)}>{showAllActivity ? 'Show recent only' : 'Show more activity'}</button>}
    </section>

    <section className="surface-card player-hub__history">
      <div className="surface-card__heading"><div><p className="eyebrow">PLAYER-AUTHORED HISTORY</p><h3>Snapshots</h3></div><button className="button button--small" disabled={saving} onClick={() => void perform(async () => {
        await client.capturePlayerSnapshot(player.id);
        setSnapshots(await client.listPlayerSnapshots(player.id));
      }, 'An explicit Player snapshot was captured.')}>Capture snapshot</button></div>
      <p className="muted">Captures are explicit. Missing-day snapshots are never synthesized.</p>
      {snapshots.length === 0 ? <div className="player-hub__empty"><strong>No snapshots yet</strong><p>Capture a snapshot when you want a point-in-time record of your state.</p></div>
        : snapshots.slice().reverse().slice(0, 5).map(snapshot => <div className="snapshot-row" key={snapshot.id}><span>{snapshot.snapshotDate}</span><strong>Level {snapshot.level}</strong><small>{snapshot.currentXp.toLocaleString()} XP · captured {formatDate(snapshot.createdAt)}</small></div>)}
    </section>

    {message && <p className="inline-feedback" role="status">{message}</p>}
    {error && <p className="error-banner" role="alert">{error}</p>}
  </div>;
}
