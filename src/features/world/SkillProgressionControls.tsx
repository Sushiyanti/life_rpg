import { useEffect, useState } from 'react';
import type { CoreClient } from '../../domain/ipc';
import type { Comment, ContentAttachment, NarrativeEntry, Skill, SkillAvailabilityControl, SkillSnapshot, TimelineItem } from '../../domain/world';
import './SkillProgressionControls.css';

type Props = {
  skill: Skill;
  playerId: string;
  parentName?: string;
  narratives: NarrativeEntry[];
  client: CoreClient;
  busy: boolean;
  onDo: (fn: () => Promise<unknown>, success: string) => Promise<void>;
};

export function SkillProgressionControls({ skill, playerId, parentName, narratives, client, busy, onDo }: Props) {
  const [snapshots, setSnapshots] = useState<SkillSnapshot[]>([]);
  const [timeline, setTimeline] = useState<TimelineItem[]>([]);
  const [comments, setComments] = useState<Comment[]>([]);
  const [attachments, setAttachments] = useState<ContentAttachment[]>([]);
  const [loadError, setLoadError] = useState('');

  useEffect(() => {
    let live = true;
    setLoadError('');
    const query = { playerId, category: null, entityKind: 'skill' as const, entityId: skill.id, conceptId: null, from: null, through: null, sort: 'newest' as const, limit: 12, offset: 0 };
    void Promise.allSettled([
      Promise.resolve().then(() => client.listSkillSnapshots(skill.id)),
      Promise.resolve().then(() => client.queryTimeline(query)),
      Promise.resolve().then(() => client.listComments('skill', skill.id)),
      Promise.resolve().then(() => client.listAttachedContent(playerId, 'skill', skill.id)),
    ]).then(results => {
      if (!live) return;
      const [snapshotResult, timelineResult, commentsResult, attachmentsResult] = results;
      if (snapshotResult.status === 'fulfilled') setSnapshots(snapshotResult.value); else setSnapshots([]);
      if (timelineResult.status === 'fulfilled') setTimeline(timelineResult.value); else setTimeline([]);
      if (commentsResult.status === 'fulfilled') setComments(commentsResult.value); else setComments([]);
      if (attachmentsResult.status === 'fulfilled') setAttachments(attachmentsResult.value); else setAttachments([]);
      if (results.some(result => result.status === 'rejected')) setLoadError('Some Skill history, snapshot, comment, or Content data could not be loaded.');
    });
    return () => { live = false; };
  }, [client, playerId, skill.id, skill.currentXp, skill.availability, skill.availabilityControl]);

  const displayedAttachments = attachments.filter(link => link.isActive);
  const contentName = (link: ContentAttachment) => narratives.find(entry => entry.id === link.contentId)?.title ?? link.contentId;
  return <div className="skill-progression" aria-label={`${skill.name} progression and history`}>
    <div className="skill-progression__status">
      <span className={`skill-availability skill-availability--${skill.availability}`}>{skill.availability === 'available' ? 'Unlocked' : 'Locked'}</span>
      <span className="skill-lifecycle">Lifecycle · {skill.status}</span>
      <span>Level {skill.level}{skill.levelName ? ` · ${skill.levelName}` : ''}</span>
      <span>{skill.currentXp} Skill XP</span>
      <span>{skill.investedMinutes} minutes invested</span>
      {parentName && <span>Parent Skill · {parentName}</span>}
    </div>
    {skill.progressionLabel && <p className="skill-progression__label">{skill.progressionLabel}</p>}
    {(skill.story || skill.instructions) && <details className="skill-progression__details"><summary>Story &amp; instructions</summary>{skill.story && <p><strong>Story</strong><br />{skill.story}</p>}{skill.instructions && <p><strong>Instructions</strong><br />{skill.instructions}</p>}</details>}
    <div className="skill-progression__actions">
      <button className="button button--small" disabled={busy} onClick={() => void onDo(() => client.adjustSkillXp(skill.id, 10, 'manual_skill_xp'), 'Added 10 Skill XP. Skill level remains unchanged.')}>+10 XP</button>
      <button className="button button--small button--quiet" disabled={busy || skill.currentXp === 0} onClick={() => void onDo(() => client.adjustSkillXp(skill.id, -10, 'manual_skill_xp'), 'Requested a 10 XP reduction; applied XP is clamped at zero.')}>−10 XP</button>
      <button className="button button--small button--quiet" disabled={busy} onClick={() => void onDo(() => client.setSkillAvailability(skill.id, skill.availability !== 'available'), skill.availability === 'available' ? 'Skill manually locked. Rules cannot unlock it until you explicitly delegate authority.' : 'Skill manually unlocked.')}>{skill.availability === 'available' ? 'Lock manually' : 'Unlock manually'}</button>
      <label className="skill-progression__policy">Unlock authority<select aria-label={`Rule authority for ${skill.name}`} disabled={busy} value={skill.availabilityControl} onChange={event => void onDo(() => client.setSkillAvailabilityControl(skill.id, event.target.value as SkillAvailabilityControl), event.target.value === 'rule_controlled' ? 'Rule authority enabled for this Skill.' : 'Skill unlock authority returned to manual control.')}><option value="manual">Player only</option><option value="rule_controlled">Rules may unlock</option></select></label>
    </div>
    <details className="skill-progression__history"><summary>Related records · {timeline.length} history items, {snapshots.length} snapshots, {comments.length} comments, {displayedAttachments.length} Content links</summary>
      {loadError && <p className="skill-progression__error" role="status">{loadError}</p>}
      {displayedAttachments.length > 0 && <div><strong>Attached Content</strong><ul>{displayedAttachments.map(link => <li key={link.id}>{contentName(link)} · {link.roleCode}</li>)}</ul></div>}
      {comments.length > 0 && <div><strong>Comments</strong><ul>{comments.slice(0, 5).map(comment => <li key={String(comment.id)}>{comment.body} <small>{comment.createdAt}</small></li>)}</ul></div>}
      {timeline.length > 0 && <div><strong>Persisted Skill history</strong><ul>{timeline.map(item => <li key={item.sourceId}><time>{item.timestamp}</time> · <b>{item.title}</b>{item.summary && <span> — {item.summary}</span>}</li>)}</ul></div>}
      {snapshots.length > 0 && <div><strong>Explicit snapshots</strong><ul>{snapshots.slice(0, 5).map(snapshot => <li key={`${snapshot.id ?? ''}:${snapshot.snapshotDate}`}>{snapshot.snapshotDate} · Level {snapshot.level} · {snapshot.currentXp} XP · {snapshot.status}</li>)}</ul></div>}
      {!loadError && timeline.length === 0 && snapshots.length === 0 && comments.length === 0 && displayedAttachments.length === 0 && <p>No related history or Content is recorded yet.</p>}
    </details>
  </div>;
}
