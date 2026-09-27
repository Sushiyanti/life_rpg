import { useEffect, useMemo, useRef, useState, type FormEvent } from 'react';
import type { CoreClient } from '../../domain/ipc';
import type { Concept, Player, Quest, QuestSession, Skill } from '../../domain/world';
import './QuickCapture.css';

export type CaptureContext = { kind: string; id: string; label: string };

type Props = {
  client: CoreClient;
  player: Player;
  quests: Quest[];
  skills: Skill[];
  concepts: Concept[];
  sessions: QuestSession[];
  onClose: () => void;
  onSaved: () => Promise<void>;
  initialKind?: string;
};

const quickKinds = ['note', 'journal', 'reflection', 'reminder', 'todo'] as const;
const roleByKind: Record<string, string> = {
  note: 'notes', journal: 'reflection', reflection: 'reflection', reminder: 'reminder', todo: 'todo',
};

export function QuickCapture({ client, player, quests, skills, concepts, sessions, onClose, onSaved, initialKind = 'note' }: Props) {
  const [kind, setKind] = useState<string>(quickKinds.includes(initialKind as typeof quickKinds[number]) ? initialKind : 'note');
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [contextKey, setContextKey] = useState('');
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  const dialogRef = useRef<HTMLElement>(null);
  const contexts = useMemo<CaptureContext[]>(() => [
    ...quests.filter(item => item.status === 'open' || item.status === 'active').map(item => ({ kind: 'quest', id: item.id, label: `Quest · ${item.title}` })),
    ...skills.filter(item => item.status === 'active').map(item => ({ kind: 'skill', id: item.id, label: `Skill · ${item.name}` })),
    ...concepts.filter(item => item.isActive).map(item => ({ kind: 'concept', id: item.id, label: `Concept · ${item.name}` })),
    ...sessions.filter(item => item.status === 'in_progress').map(item => ({ kind: 'quest_session', id: item.id, label: `Active Session · ${contextLabel(item, quests, skills, concepts)}` })),
  ], [quests, skills, concepts, sessions]);

  useEffect(() => {
    function keepDialogKeyboardAccessible(event: KeyboardEvent) {
      if (event.key === 'Escape' && !busy) {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key !== 'Tab') return;
      const focusable = dialogRef.current?.querySelectorAll<HTMLElement>('button:not([disabled]),input:not([disabled]),select:not([disabled]),textarea:not([disabled]),[tabindex]:not([tabindex="-1"])');
      if (!focusable?.length) return;
      const first = focusable[0]!;
      const last = focusable[focusable.length - 1]!;
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
    }
    document.addEventListener('keydown', keepDialogKeyboardAccessible);
    return () => document.removeEventListener('keydown', keepDialogKeyboardAccessible);
  }, [busy, onClose]);

  async function submit(event: FormEvent) {
    event.preventDefault();
    const body = content.trim();
    if (!body) return;
    const firstLine = body.split(/\r?\n/, 1)[0]?.trim() ?? '';
    const entryTitle = title.trim() || firstLine.slice(0, 100) || `${kind[0]!.toUpperCase()}${kind.slice(1)}`;
    const context = contexts.find(item => `${item.kind}|${item.id}` === contextKey);
    setBusy(true);
    setError('');
    setMessage('');
    try {
      const entry = await client.writeNarrative(player.id, kind, entryTitle, body);
      setTitle('');
      setContent('');
      let attachmentError = '';
      if (context) {
        try {
          await client.attachContent(player.id, entry.id, context.kind, context.id, roleByKind[kind] ?? 'notes');
        } catch (reason) {
          attachmentError = reason instanceof Error ? reason.message : 'The attachment did not save. The content itself is safe.';
        }
      }
      let refreshError = '';
      await onSaved().catch(reason => { refreshError = reason instanceof Error ? reason.message : 'The view could not be refreshed.'; });
      if (attachmentError) {
        setMessage(`Your ${kind} was saved, but the optional link to ${context!.label} could not be added.`);
        setError(`${attachmentError}${refreshError ? ` The entry is saved, but the view could not refresh: ${refreshError}` : ''}`);
      } else if (refreshError) {
        setMessage(`${kind[0]!.toUpperCase()}${kind.slice(1)} was saved to your Chronicle.`);
        setError(`The entry is safely saved, but this view could not refresh: ${refreshError}`);
      } else {
        setMessage(context ? `Saved and attached to ${context.label}.` : `${kind[0]!.toUpperCase()}${kind.slice(1)} saved to your Chronicle.`);
      }
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Your content could not be saved. Your draft is still here.');
    } finally {
      setBusy(false);
    }
  }

  return <div className="capture-backdrop" role="presentation" onMouseDown={event => { if (!busy && event.target === event.currentTarget) onClose(); }}>
    <section ref={dialogRef} className="capture-dialog" role="dialog" aria-modal="true" aria-labelledby="capture-title" onMouseDown={event => event.stopPropagation()}>
      <header className="capture-dialog__head"><div><p className="eyebrow">QUICK CAPTURE · PLAYER-AUTHORED</p><h2 id="capture-title">Keep this in your world</h2><p>Only what you write is recorded. No context is linked unless you choose it.</p></div><button type="button" className="button button--icon button--quiet" aria-label="Close quick capture" disabled={busy} onClick={onClose}>×</button></header>
      <form className="capture-form" onSubmit={event => void submit(event)}>
        <label>Entry kind<select value={kind} onChange={event => setKind(event.target.value)}>{quickKinds.map(value => <option value={value} key={value}>{value[0]!.toUpperCase()}{value.slice(1)}</option>)}</select></label>
        <label>Title <span className="muted">optional</span><input value={title} onChange={event => setTitle(event.target.value)} maxLength={240} placeholder="A title will come from your first line" /></label>
        <label>What do you want to keep?<textarea autoFocus required rows={6} maxLength={20000} value={content} onChange={event => setContent(event.target.value)} placeholder="Write the observation, thought, reminder, or next step…" /></label>
        <label>Attach to context <span className="muted">optional</span><select value={contextKey} onChange={event => setContextKey(event.target.value)}><option value="">No attachment · keep this entry standalone</option>{contexts.map(item => <option key={`${item.kind}|${item.id}`} value={`${item.kind}|${item.id}`}>{item.label}</option>)}</select></label>
        <div className="capture-form__foot"><span className="muted">Saved locally with its actual creation time.</span><div><button type="button" className="button button--quiet" disabled={busy} onClick={onClose}>Cancel</button><button className="button button--primary" disabled={busy || !content.trim()}>{busy ? 'Saving…' : 'Save entry'}</button></div></div>
      </form>
      {message && <p className="capture-message" role="status">{message}</p>}
      {error && <p className="capture-error" role="alert">{error}</p>}
    </section>
  </div>;
}

function contextLabel(session: QuestSession, quests: Quest[], skills: Skill[], concepts: Concept[]) {
  if (session.questId) return quests.find(item => item.id === session.questId)?.title ?? 'Quest';
  if (session.skillId) return skills.find(item => item.id === session.skillId)?.name ?? 'Skill';
  if (session.conceptId) return concepts.find(item => item.id === session.conceptId)?.name ?? 'Concept';
  return 'Recorded activity';
}
