import { useEffect, useState } from 'react';
import type { CoreClient } from '../../domain/ipc';
import type { TimelineItem, TimelineQuery, WorkspacePanel } from '../../domain/world';
import './TimelineWorkspacePanel.css';

type Props = { client: CoreClient; panel: WorkspacePanel; playerId: string; onOpenEntity: (kind: string, id: string) => void };
const boundary = (date: string | null, end: boolean): string | null => {
  if (!date) return null;
  const value = new Date(`${date}T${end ? '23:59:59.999' : '00:00:00.000'}`);
  return Number.isNaN(value.valueOf()) ? null : value.toISOString();
};
const readable = (value: string) => value.replaceAll('_', ' ');
const dateTime = (value: string) => new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(value));

export function TimelineWorkspacePanel({ client, panel, playerId, onOpenEntity }: Props) {
  const [items, setItems] = useState<TimelineItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  useEffect(() => {
    let live = true;
    setLoading(true);
    setError('');
    const query: TimelineQuery = {
      playerId,
      category: panel.filterTimelineCategory,
      entityKind: panel.filterTimelineEntityKind,
      entityId: panel.filterTimelineEntityId,
      conceptId: panel.filterConceptId,
      from: boundary(panel.filterTimelineFrom, false),
      through: boundary(panel.filterTimelineThrough, true),
      sort: panel.sortBy === 'timeline_oldest' ? 'oldest' : 'newest',
      limit: panel.itemLimit,
      offset: 0,
    };
    void client.queryTimeline(query).then(rows => { if (live) setItems(rows); }).catch(reason => {
      if (live) setError(reason instanceof Error ? reason.message : 'Timeline records could not be loaded.');
    }).finally(() => { if (live) setLoading(false); });
    return () => { live = false; };
  }, [client, playerId, panel.id, panel.filterTimelineCategory, panel.filterTimelineEntityKind, panel.filterTimelineEntityId, panel.filterConceptId, panel.filterTimelineFrom, panel.filterTimelineThrough, panel.sortBy, panel.itemLimit]);

  if (loading) return <p className="timeline-panel-state" role="status">Loading recorded history…</p>;
  if (error) return <p className="timeline-panel-state" role="alert">{error}</p>;
  if (!items.length) return <p className="timeline-panel-state">No recorded history matches this panel’s filters.</p>;
  return <ol className="timeline-panel-list" aria-label="Filtered recorded Timeline">
    {items.map(item => <li key={item.sourceId}>
      <article className="timeline-panel-item">
        <div className="timeline-panel-item__when"><time dateTime={item.timestamp}>{dateTime(item.timestamp)}</time><span>{readable(item.timestampKind)}</span></div>
        <button className="timeline-panel-item__open" type="button" onClick={() => onOpenEntity(item.entityKind, item.entityId)} title={`Open exact ${readable(item.entityKind)} record`}><strong>{item.title}</strong><small>{readable(item.category)} · {readable(item.entityKind)} · {item.entityId}</small></button>
        {item.summary && <p>{item.summary}</p>}
        {item.relationshipContext && <details className="timeline-panel-item__relationship"><summary>Relationship · {item.relationshipContext.contentTitle} · {readable(item.relationshipContext.roleCode)}</summary><p>{item.relationshipContext.removedAt ? `Removed ${dateTime(item.relationshipContext.removedAt)}` : 'Currently active'} · attached {dateTime(item.relationshipContext.createdAt)}</p><div><button className="text-link" type="button" onClick={() => onOpenEntity('narrative_entry', item.relationshipContext!.contentId)}>Open Content</button><button className="text-link" type="button" onClick={() => onOpenEntity(item.entityKind, item.entityId)}>Open target</button></div></details>}
      </article>
    </li>)}
  </ol>;
}
