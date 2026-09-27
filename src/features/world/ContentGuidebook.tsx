import { useEffect, useMemo, useState, type FormEvent } from 'react';
import type { CoreClient } from '../../domain/ipc';
import type { Concept, ContentAttachment, NarrativeEntry, Player, WorldOverview } from '../../domain/world';
import './ContentGuidebook.css';

const CONTENT_KINDS = ['note','introduction','briefing','story','guide','guidance','instruction','reading','todo','reference','summary','reminder','reflection','journal'] as const;
const label = (value: string) => value.replaceAll('_', ' ').replace(/\b\w/g, char => char.toUpperCase());
const date = (value: string) => new Date(value).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });

type Props = {
  client: CoreClient;
  player: Player;
  overview: WorldOverview;
  concepts: Concept[];
  onRefresh: (message?: string) => Promise<void>;
  onOpenEntity: (kind: string, id: string) => void;
};

type Draft = { kind: string; title: string; content: string; author: string; sourceKind: string; sourceId: string };
const blankDraft = (): Draft => ({ kind: 'note', title: '', content: '', author: '', sourceKind: '', sourceId: '' });

export function ContentGuidebook({ client, player, overview, concepts, onRefresh, onOpenEntity }: Props) {
  const [query, setQuery] = useState('');
  const [kindFilter, setKindFilter] = useState('all');
  const [selected, setSelected] = useState<NarrativeEntry | null>(null);
  const [draft, setDraft] = useState<Draft>(blankDraft);
  const [editing, setEditing] = useState(false);
  const [roles, setRoles] = useState<string[]>(['guidance', 'notes', 'reference']);
  const [relationships, setRelationships] = useState<ContentAttachment[]>([]);
  const [targetKind, setTargetKind] = useState('player');
  const [targetId, setTargetId] = useState(player.id);
  const [role, setRole] = useState('guidance');
  const [message, setMessage] = useState('');
  const [busy, setBusy] = useState(false);

  useEffect(() => { void client.listContentAttachmentRoles().then(rows => { if (rows.length) { setRoles(rows); setRole(current => rows.includes(current) ? current : rows[0]!); } }).catch(() => {}); }, [client]);
  useEffect(() => { let live = true; if (!selected) { setRelationships([]); return; } void client.listContentRelationships(player.id, selected.id, true).then(rows => { if (live) setRelationships(rows); }).catch(() => { if (live) setRelationships([]); }); return () => { live = false; }; }, [client, player.id, selected?.id]);
  useEffect(() => { if (targetKind === 'player') setTargetId(player.id); }, [player.id, targetKind]);

  const targets = useMemo(() => {
    const base = [{ kind: 'player', id: player.id, label: `Player · ${player.name}` }];
    return [...base,
      ...overview.quests.map(value => ({ kind: 'quest', id: value.id, label: `Quest · ${value.title}` })),
      ...overview.skillTrees.map(value => ({ kind: 'skill_tree', id: value.id, label: `Skill tree · ${value.name}` })),
      ...overview.skills.map(value => ({ kind: 'skill', id: value.id, label: `Skill · ${value.name}` })),
      ...concepts.map(value => ({ kind: 'concept', id: value.id, label: `Concept · ${value.name}` })),
      ...overview.effects.map(value => ({ kind: 'effect', id: value.id, label: `Effect · ${value.name}` })),
    ];
  }, [player, overview, concepts]);
  const filtered = useMemo(() => overview.narratives.filter(entry => {
    const haystack = `${entry.title} ${entry.content} ${entry.author ?? ''} ${entry.sourceKind ?? ''}`.toLowerCase();
    return (kindFilter === 'all' || entry.kind === kindFilter) && (!query.trim() || haystack.includes(query.trim().toLowerCase()));
  }), [overview.narratives, kindFilter, query]);

  const choose = (entry: NarrativeEntry) => { setSelected(entry); setDraft({ kind: entry.kind, title: entry.title, content: entry.content, author: entry.author ?? '', sourceKind: entry.sourceKind ?? '', sourceId: entry.sourceId ?? '' }); setEditing(false); setMessage(''); };
  const create = () => { setSelected(null); setDraft(blankDraft()); setEditing(true); setMessage(''); };
  const save = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true); setMessage('');
    try {
      const value = { kind: draft.kind, title: draft.title, content: draft.content, author: draft.author || null, sourceKind: draft.sourceKind || null, sourceId: draft.sourceId || null };
      const saved = selected
        ? await client.updateNarrative(player.id, selected.id, value)
        : await client.createNarrative(player.id, value);
      await onRefresh(selected ? 'Content updated. A recoverable prior version was recorded.' : 'Content saved to your guidebook.');
      setSelected(saved); setDraft({ kind: saved.kind, title: saved.title, content: saved.content, author: saved.author ?? '', sourceKind: saved.sourceKind ?? '', sourceId: saved.sourceId ?? '' }); setEditing(false);
    } catch (error) { setMessage(error instanceof Error ? error.message : 'Content could not be saved.'); }
    finally { setBusy(false); }
  };
  const attach = async () => {
    if (!selected || !targetId) return;
    setBusy(true); setMessage('');
    try { const link = await client.attachContent(player.id, selected.id, targetKind, targetId, role); setRelationships(rows => [...rows, link]); await onRefresh('Content relationship recorded.'); setMessage('Attached. Removing this link later will preserve the attachment history.'); }
    catch (error) { setMessage(error instanceof Error ? error.message : 'The content could not be attached.'); }
    finally { setBusy(false); }
  };
  const detach = async (relationshipId: string) => {
    if (!selected) return;
    setBusy(true); setMessage('');
    try {
      await client.removeContentAttachment(player.id, relationshipId);
      setRelationships(await client.listContentRelationships(player.id, selected.id, true));
      await onRefresh('Content relationship removed.');
      setMessage('The relationship was removed as a timestamped historical fact. The content and target remain unchanged.');
    } catch (error) { setMessage(error instanceof Error ? error.message : 'The content relationship could not be removed.'); }
    finally { setBusy(false); }
  };
  const targetOptions = targets.filter(target => target.kind === targetKind);

  return <div className="guidebook-page">
    <header className="page-heading guidebook-heading"><div><p className="eyebrow">REUSABLE CONTENT · AUTHORED GUIDANCE</p><h2>Content Guidebook</h2><p>Create reusable notes, instructions, references, and reflections. Content stays separate from comments, progression, and world state.</p></div><button className="button button--primary" onClick={create}>＋ New content</button></header>
    <section className="guidebook-toolbar surface-card"><label className="filter-search">Find content<input value={query} onChange={event => setQuery(event.target.value)} placeholder="Search title, body, author, or source…" /></label><label>Kind<select value={kindFilter} onChange={event => setKindFilter(event.target.value)}><option value="all">All kinds</option>{CONTENT_KINDS.map(kind => <option key={kind} value={kind}>{label(kind)}</option>)}</select></label><span>{filtered.length} shown · {overview.narratives.length} total</span></section>
    <div className="guidebook-layout"><aside className="guidebook-list" aria-label="Guidebook content">{filtered.length === 0 ? <div className="empty-card"><strong>No matching content</strong><p>Write the first reusable entry or adjust the filters.</p></div> : filtered.map(entry => <button className={`guidebook-card ${selected?.id === entry.id ? 'is-selected' : ''}`} key={entry.id} onClick={() => choose(entry)}><span className="type-pill">{label(entry.kind)}</span><strong>{entry.title}</strong><p>{entry.content.slice(0, 160)}{entry.content.length > 160 ? '…' : ''}</p><small>Updated {date(entry.updatedAt)}</small></button>)}</aside>
      <main className="guidebook-detail">{editing ? <form className="surface-card guidebook-editor" onSubmit={event => void save(event)}><div className="surface-card__heading"><div><p className="eyebrow">{selected ? 'EDIT CONTENT' : 'NEW CONTENT'}</p><h3>{selected ? 'Revise this content' : 'Add reusable content'}</h3></div>{selected && <button type="button" className="button button--small button--quiet" onClick={() => { setEditing(false); choose(selected); }}>Cancel</button>}</div><label>Content kind<select value={draft.kind} onChange={event => setDraft({ ...draft, kind: event.target.value })}>{CONTENT_KINDS.map(kind => <option key={kind} value={kind}>{label(kind)}</option>)}</select></label><label>Title<input required maxLength={512} value={draft.title} onChange={event => setDraft({ ...draft, title: event.target.value })} placeholder="A concise title" /></label><label>Body<textarea required rows={11} value={draft.content} onChange={event => setDraft({ ...draft, content: event.target.value })} placeholder="Write the actual instruction, note, or reflection…" /></label><div className="form-grid form-grid--three"><label>Author <span className="muted">optional</span><input value={draft.author} onChange={event => setDraft({ ...draft, author: event.target.value })} placeholder="Player-authored by default" /></label><label>Source kind <span className="muted">optional</span><input value={draft.sourceKind} onChange={event => setDraft({ ...draft, sourceKind: event.target.value })} placeholder="e.g. book, conversation" /></label><label>Source ID <span className="muted">optional</span><input value={draft.sourceId} onChange={event => setDraft({ ...draft, sourceId: event.target.value })} placeholder="Paired with source kind" /></label></div><button className="button button--primary" disabled={busy}>{busy ? 'Saving…' : selected ? 'Save revision' : 'Save content'}</button></form> : selected ? <>
        <article className="surface-card guidebook-reading"><div className="surface-card__heading"><div><span className="type-pill">{label(selected.kind)}</span><h3>{selected.title}</h3><p className="muted">Created {date(selected.createdAt)} · Updated {date(selected.updatedAt)}</p></div><div className="guidebook-reading__actions"><button className="button button--small" onClick={() => setEditing(true)}>Edit</button><button className="button button--small button--quiet" onClick={() => onOpenEntity('narrative_entry', selected.id)}>Lifecycle & history</button></div></div><p className="guidebook-body">{selected.content}</p><dl className="guidebook-meta"><div><dt>Author</dt><dd>{selected.author ?? 'Not specified'}</dd></div><div><dt>Source</dt><dd>{selected.sourceKind && selected.sourceId ? `${selected.sourceKind} · ${selected.sourceId}` : 'No source recorded'}</dd></div><div><dt>Status</dt><dd>{selected.isActive ? 'Active content record' : 'Inactive content record'}</dd></div></dl></article>
        <section className="surface-card guidebook-attach"><div className="surface-card__heading"><div><p className="eyebrow">EXPLICIT RELATIONSHIP</p><h3>Attach to a world record</h3></div><span>{relationships.filter(item => item.isActive).length} active</span></div><p className="muted">The target remains its own entity. This only records how this content is used there.</p><div className="form-grid form-grid--three"><label>Target type<select value={targetKind} onChange={event => { setTargetKind(event.target.value); const first = targets.find(target => target.kind === event.target.value); setTargetId(first?.id ?? ''); }}>{['player','quest','skill_tree','skill','concept','effect'].map(kind => <option key={kind} value={kind}>{label(kind)}</option>)}</select></label><label>Target<select value={targetId} onChange={event => setTargetId(event.target.value)}>{targetOptions.map(target => <option key={target.id} value={target.id}>{target.label}</option>)}</select></label><label>Role<select value={role} onChange={event => setRole(event.target.value)}>{roles.map(item => <option key={item} value={item}>{label(item)}</option>)}</select></label></div><button className="button button--cyan" disabled={busy || !targetId} onClick={() => void attach()}>Attach existing content</button>{relationships.length>0&&<div className="guidebook-relationship-list">{relationships.map(item=><div key={item.id}><strong>{label(item.targetKind)} · {item.targetId}</strong><span>{label(item.roleCode)} · {item.isActive?'active':`removed ${item.removedAt ? date(item.removedAt) : ''}`}</span>{item.isActive&&<button className="button button--small button--quiet" disabled={busy} onClick={() => void detach(item.id)}>Remove link</button>}</div>)}</div>}<p className="guidebook-attach__hint">For stages, branches, and Sessions, use the target’s detail view in World Explorer. Relationship removal is a timestamped history fact and never deletes the content or target.</p></section>
      </> : <div className="empty-card guidebook-empty"><strong>Select content to read or edit it</strong><p>Use the list to browse existing content, or start a new authored entry.</p><button className="button button--primary" onClick={create}>Create content</button></div>}</main></div>{message && <p className="inline-feedback" role="status">{message}</p>}</div>;
}
