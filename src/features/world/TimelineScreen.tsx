import { useEffect, useState } from 'react';
import type { CoreClient } from '../../domain/ipc';
import type { Concept, Player, TimelineCategory, TimelineEntityKind, TimelineItem, TimelineQuery } from '../../domain/world';
import './TimelineScreen.css';

const PAGE_SIZE = 50;
const MAX_OFFSET = 100_000;
const categories: { value: TimelineCategory; label: string }[] = [
  { value: 'session', label: 'Sessions' },
  { value: 'transaction', label: 'Transactions' },
  { value: 'effect_history', label: 'Effects' },
  { value: 'content', label: 'Content' },
  { value: 'comment', label: 'Comments' },
  { value: 'concept_progress', label: 'Progress' },
  { value: 'revision', label: 'Revisions' },
  { value: 'snapshot', label: 'Snapshots' },
  { value: 'record_change', label: 'Quest, Skill & Player changes' },
  { value: 'lifecycle', label: 'Lifecycle history' },
  { value: 'relationship_history', label: 'Content relationship history' },
];
const entityKinds: { value: TimelineEntityKind; label: string }[] = [
  { value: 'player', label: 'Player' },
  { value: 'concept', label: 'Concept' },
  { value: 'quest', label: 'Quest' },
  { value: 'quest_stage', label: 'Quest Stage' },
  { value: 'quest_branch', label: 'Quest Branch' },
  { value: 'quest_session', label: 'Session' },
  { value: 'skill_tree', label: 'Skill Tree' },
  { value: 'skill', label: 'Skill' },
  { value: 'effect', label: 'Effect' },
  { value: 'transaction', label: 'Transaction' },
  { value: 'narrative_entry', label: 'Content' },
  { value: 'concept_progress', label: 'Concept progress' },
];
const categoryLabels = Object.fromEntries(categories.map(({ value, label }) => [value, label])) as Record<TimelineCategory, string>;
const readable = (value: string) => value.replaceAll('_', ' ');
const timestampLabel = (item: TimelineItem, secondary = false) => item.category === 'session'
  ? (secondary ? 'Ended' : 'Started')
  : readable(secondary ? item.secondaryTimestampKind ?? 'recorded' : item.timestampKind);
const formatDateTime = (value: string) => new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(value));
const formatGroupDate = (value: string) => new Intl.DateTimeFormat(undefined, { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' }).format(new Date(value));
const localDateBoundary = (value: string, end: boolean): string | null => {
  if (!value) return null;
  const date = new Date(`${value}T${end ? '23:59:59.999' : '00:00:00.000'}`);
  return Number.isNaN(date.valueOf()) ? null : date.toISOString();
};

type Props = { client: CoreClient; player: Pick<Player, 'id' | 'name'>; concepts: Concept[]; onOpenEntity: (kind: string, id: string) => void };

export function TimelineScreen({ client, player, concepts, onOpenEntity }: Props) {
  const [category, setCategory] = useState<TimelineCategory | ''>('');
  const [entityKind, setEntityKind] = useState<TimelineEntityKind | ''>('');
  const [conceptId, setConceptId] = useState('');
  const [fromDate, setFromDate] = useState('');
  const [throughDate, setThroughDate] = useState('');
  const [sort, setSort] = useState<'newest' | 'oldest'>('newest');
  const [items, setItems] = useState<TimelineItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [hasMore, setHasMore] = useState(false);
  const [nextOffset, setNextOffset] = useState(0);
  const [error, setError] = useState('');

  useEffect(() => {
    let live = true;
    setLoading(true);
    setError('');
    setItems([]);
    setNextOffset(0);
    setHasMore(false);
    const query: TimelineQuery = {
      playerId: player.id,
      category: category || null,
      entityKind: entityKind || null,
      conceptId: conceptId || null,
      from: localDateBoundary(fromDate, false),
      through: localDateBoundary(throughDate, true),
      sort,
      limit: PAGE_SIZE,
      offset: 0,
    };
    void client.queryTimeline(query).then(rows => {
      if (!live) return;
      setItems(rows);
      setNextOffset(rows.length);
      setHasMore(rows.length === PAGE_SIZE && rows.length <= MAX_OFFSET);
    }).catch(reason => {
      if (!live) return;
      setError(reason instanceof Error ? reason.message : 'The Timeline could not be loaded.');
    }).finally(() => {
      if (live) setLoading(false);
    });
    return () => { live = false; };
  }, [client, player.id, category, entityKind, conceptId, fromDate, throughDate, sort]);

  const queryForPage = (offset: number): TimelineQuery => ({
    playerId: player.id,
    category: category || null,
    entityKind: entityKind || null,
    conceptId: conceptId || null,
    from: localDateBoundary(fromDate, false),
    through: localDateBoundary(throughDate, true),
    sort,
    limit: PAGE_SIZE,
    offset,
  });
  const loadMore = async () => {
    if (loadingMore || !hasMore) return;
    setLoadingMore(true);
    setError('');
    try {
      const rows = await client.queryTimeline(queryForPage(nextOffset));
      setItems(current => [...current, ...rows]);
      setNextOffset(current => current + rows.length);
      setHasMore(rows.length === PAGE_SIZE && nextOffset + rows.length <= MAX_OFFSET);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'More Timeline records could not be loaded.');
    } finally {
      setLoadingMore(false);
    }
  };
  const clearFilters = () => {
    setCategory('');
    setEntityKind('');
    setConceptId('');
    setFromDate('');
    setThroughDate('');
    setSort('newest');
  };

  const groups: { key: string; label: string; items: TimelineItem[] }[] = [];
  for (const item of items) {
    const key = new Date(item.timestamp).toLocaleDateString();
    const last = groups.at(-1);
    if (!last || last.key !== key) groups.push({ key, label: formatGroupDate(item.timestamp), items: [item] });
    else last.items.push(item);
  }

  return <div className="timeline-page">
    <header className="timeline-heading">
      <div><p className="eyebrow">PERSISTED WORLD & HISTORY RECORDS</p><h2>Timeline</h2><p>Browse what this world has actually recorded, with each source’s original time meaning intact.</p></div>
      <div className="timeline-heading__scope"><span>Active Player</span><strong>{player.name}</strong></div>
    </header>

    <section className="timeline-filters" aria-label="Timeline filters">
      <label>Category<select aria-label="Timeline category" value={category} onChange={event => setCategory(event.target.value as TimelineCategory | '')}><option value="">All recorded sources</option>{categories.map(item => <option key={item.value} value={item.value}>{item.label}</option>)}</select></label>
      <label>Entity kind<select aria-label="Entity kind" value={entityKind} onChange={event => setEntityKind(event.target.value as TimelineEntityKind | '')}><option value="">All entity kinds</option>{entityKinds.map(item => <option key={item.value} value={item.value}>{item.label}</option>)}</select></label>
      <label>Concept<select aria-label="Timeline Concept" value={conceptId} onChange={event => setConceptId(event.target.value)}><option value="">All Concepts</option>{concepts.map(item => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label>
      <label>From<input aria-label="Timeline from date" type="date" value={fromDate} onChange={event => setFromDate(event.target.value)} /></label>
      <label>Through<input aria-label="Timeline through date" type="date" value={throughDate} onChange={event => setThroughDate(event.target.value)} /></label>
      <label>Order<select aria-label="Timeline order" value={sort} onChange={event => setSort(event.target.value as 'newest' | 'oldest')}><option value="newest">Newest first</option><option value="oldest">Oldest first</option></select></label>
      <button className="button button--quiet timeline-filters__clear" type="button" onClick={clearFilters}>Clear filters</button>
    </section>
    <p className="timeline-note">Events, mutable-record revisions, state snapshots, and lifecycle history remain labeled as distinct records. Missing dates and unrecorded activity stay missing.</p>

    {error && <div className="error-banner" role="alert">{error}</div>}
    {loading && <div className="timeline-state" role="status"><span className="loading-dot" />Loading recorded history…</div>}
    {!loading && !error && items.length === 0 && <div className="timeline-empty"><span aria-hidden="true">◷</span><strong>No recorded events in this range.</strong><p>No history rows match these filters. This does not imply that nothing happened.</p></div>}
    {!loading && items.length > 0 && <div className="timeline-stream" aria-label="Chronological world timeline">
      {groups.map(group => <section className="timeline-day" key={group.key} aria-labelledby={`timeline-day-${group.key}`}>
        <h3 id={`timeline-day-${group.key}`}>{group.label}</h3>
        <ol>{group.items.map(item => <li key={item.sourceId}>
          <article className="timeline-item">
            <div className="timeline-item__when"><time dateTime={item.timestamp}>{formatDateTime(item.timestamp)}</time><span>{timestampLabel(item)}</span>{item.secondaryTimestamp && <small>{timestampLabel(item, true)} · {formatDateTime(item.secondaryTimestamp)}</small>}</div>
            <div className="timeline-item__body">
              <div className="timeline-item__badges"><span className={`timeline-badge timeline-badge--${item.category}`}>{categoryLabels[item.category]}</span><span className="timeline-entity-kind">{readable(item.entityKind)}</span></div>
              <button className="timeline-item__open" type="button" title={`Open ${readable(item.entityKind)}: ${item.title}`} onClick={() => onOpenEntity(item.entityKind, item.entityId)}><strong>{item.title}</strong><span className="timeline-item__identity">{readable(item.entityKind)} · {item.entityId}</span></button>
              <p className="timeline-item__summary" title={item.summary}>{item.summary || 'No additional details were recorded.'}</p>
              {item.summary.length > 200 && <details className="timeline-item__full"><summary>Read full recorded summary</summary><p>{item.summary}</p></details>}
              {(item.typeCode || item.state) && <div className="timeline-item__metadata">{item.typeCode && <span>Type: {readable(item.typeCode)}</span>}{item.state && <span>State: {readable(item.state)}</span>}</div>}
              {item.conceptId && <small className="timeline-item__concept">Concept context · {concepts.find(concept => concept.id === item.conceptId)?.name ?? item.conceptId}</small>}
            </div>
          </article>
        </li>)}</ol>
      </section>)}
    </div>}
    {!loading && hasMore && <div className="timeline-more"><button className="button button--quiet" type="button" disabled={loadingMore} onClick={() => void loadMore()}>{loadingMore ? 'Loading more…' : 'Load more recorded items'}</button></div>}
    {!loading && items.length > 0 && !hasMore && <p className="timeline-end">End of the matching recorded history.</p>}
  </div>;
}
