import type {
  Concept,
  ConceptProgressTrack,
  Effect,
  NarrativeEntry,
  Player,
  PlayerStat,
  Quest,
  QuestSession,
  Skill,
  Transaction,
  WorldOverview,
} from '../../domain/world';

export type WorldActivity = {
  id: string;
  recordKind: string;
  recordId: string;
  at: string;
  title: string;
  detail: string;
};

type TimelineInput = {
  player: Player;
  overview: WorldOverview;
  concepts: Concept[];
  stats: PlayerStat[];
  sessions: QuestSession[];
  tracks: Record<string, ConceptProgressTrack[]>;
};

const pretty = (value: string) => value.replaceAll('_', ' ');

/**
 * Builds a read-only activity timeline from timestamps already stored by the
 * domain. It never synthesizes app-open, elapsed-time, or missing-day events.
 */
export function buildWorldTimeline({ overview, concepts, sessions, tracks }: TimelineInput): WorldActivity[] {
  const events: WorldActivity[] = [];
  const add = (event: WorldActivity) => {
    if (Number.isFinite(Date.parse(event.at))) events.push(event);
  };

  for (const quest of overview.quests) {
    if (quest.startedAt) {
      add({
        id: `quest:${quest.id}:started`, recordKind: 'quest', recordId: quest.id,
        at: quest.startedAt, title: 'Quest started', detail: quest.title,
      });
    }
    if (quest.completedAt) {
      add({
        id: `quest:${quest.id}:completed`, recordKind: 'quest', recordId: quest.id,
        at: quest.completedAt, title: 'Quest completed', detail: quest.title,
      });
    }
  }

  const questById = new Map(overview.quests.map(item => [item.id, item]));
  const skillById = new Map(overview.skills.map(item => [item.id, item]));
  const conceptById = new Map(concepts.map(item => [item.id, item]));

  for (const session of sessions) {
    const context = session.questId
      ? `Quest · ${questById.get(session.questId)?.title ?? 'work'}`
      : session.skillId
        ? `Skill · ${skillById.get(session.skillId)?.name ?? 'practice'}`
        : session.conceptId
          ? `Concept · ${conceptById.get(session.conceptId)?.name ?? 'work'}`
          : 'Recorded activity';
    add({
      id: `session:${session.id}:started`, recordKind: 'quest_session', recordId: session.id,
      at: session.startedAt, title: 'Session started', detail: context,
    });
    if (session.endedAt) {
      add({
        id: `session:${session.id}:ended`, recordKind: 'quest_session', recordId: session.id,
        at: session.endedAt,
        title: session.status === 'interrupted' ? 'Session interrupted' : 'Session ended',
        detail: `${context}${formatSessionDuration(session.startedAt, session.endedAt) ? ` · ${formatSessionDuration(session.startedAt, session.endedAt)}` : ''}`,
      });
    }
  }

  for (const effect of overview.effects) {
    add({
      id: `effect:${effect.id}:started`, recordKind: 'effect', recordId: effect.id,
      at: effect.startedAt, title: 'Effect recorded',
      detail: `${effect.name}${effect.targetKind === 'concept' ? ` · ${conceptById.get(effect.targetConceptId ?? '')?.name ?? 'Concept'}` : ' · Player'}`,
    });
    if (effect.deactivatedAt) {
      add({
        id: `effect:${effect.id}:deactivated`, recordKind: 'effect', recordId: effect.id,
        at: effect.deactivatedAt, title: 'Effect deactivated', detail: effect.name,
      });
    }
  }

  for (const concept of concepts) {
    for (const track of tracks[concept.id] ?? []) {
      add({
        id: `concept_progress:${concept.id}:${track.trackCode}:${track.updatedAt}`,
        recordKind: 'concept', recordId: concept.id, at: track.updatedAt,
        title: 'Concept progress recorded',
        detail: `${concept.name} · ${pretty(track.trackCode)} · ${track.currentValue}${track.level === null ? '' : ` · level ${track.level}`}`,
      });
    }
  }

  for (const note of overview.narratives) {
    add({
      id: `narrative:${note.id}:created`, recordKind: 'narrative_entry', recordId: note.id,
      at: note.createdAt, title: 'Content captured', detail: `${pretty(note.kind)} · ${note.title}`,
    });
  }

  for (const transaction of overview.recentTransactions) {
    add({
      id: `transaction:${transaction.id ?? `${transaction.occurredAt}:${transaction.resource}`}`,
      recordKind: 'transaction', recordId: String(transaction.id ?? ''), at: transaction.occurredAt,
      title: 'Transaction recorded',
      detail: `${pretty(transaction.resource)} · ${transaction.amount > 0 ? '+' : ''}${transaction.amount}${transaction.reason ? ` · ${transaction.reason}` : ''}`,
    });
  }

  return events.sort((a, b) => Date.parse(b.at) - Date.parse(a.at) || a.id.localeCompare(b.id));
}

export function formatSessionDuration(startedAt: string, endedAt: string | null): string | null {
  if (!endedAt) return null;
  const start = Date.parse(startedAt);
  const end = Date.parse(endedAt);
  if (!Number.isFinite(start) || !Number.isFinite(end) || end < start) return null;
  const totalMinutes = Math.round((end - start) / 60_000);
  if (totalMinutes < 1) return 'under 1 min';
  if (totalMinutes < 60) return `${totalMinutes} min`;
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return minutes ? `${hours} hr ${minutes} min` : `${hours} hr`;
}


// Preserve narrow imports as module consumers grow without adding a broad DTO.
export type TimelineRecord = Quest | QuestSession | Skill | Effect | ConceptProgressTrack | NarrativeEntry | Transaction;
