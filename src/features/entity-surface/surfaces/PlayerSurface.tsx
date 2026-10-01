import { useEffect, useState } from 'react';
import type { Player } from '../../../domain/world';
import { EntitySection, Field, SurfaceActions } from '../SurfacePrimitives';
import type { SurfaceRenderProps } from '../surface-types';

export function PlayerSurface({ entity: player, client, editing, setEditing, onSaved }: SurfaceRenderProps<Player>) {
  const [level, setLevel] = useState(String(player.level));
  const [levelName, setLevelName] = useState(player.levelName ?? '');
  const [progressionLabel, setProgressionLabel] = useState(player.progressionLabel ?? '');

  useEffect(() => {
    setLevel(String(player.level));
    setLevelName(player.levelName ?? '');
    setProgressionLabel(player.progressionLabel ?? '');
  }, [player]);

  async function save() {
    await client.setPlayerProgression(player.id, Number(level), levelName || undefined, progressionLabel || undefined);
    await onSaved('Player details saved.');
    setEditing(false);
  }

  return <>
    <EntitySection eyebrow="Identity" title="Player record">
      <div className="entity-grid"><Field label="Name" value={player.name} /><Field label="Level" value={String(player.level)} /><Field label="Experience" value={player.currentXp.toLocaleString()} /><Field label="Record ID" value={player.id} mono /></div>
      <p className="entity-copy">{player.description || 'Your Player record is the authored centre of this world.'}</p>
    </EntitySection>
    {editing && <EntitySection eyebrow="Edit mode" title="Player-authored progression">
      <div className="entity-form"><label>Level<input type="number" min="1" value={level} onChange={(event) => setLevel(event.target.value)} /></label><label>Level title<input value={levelName} onChange={(event) => setLevelName(event.target.value)} /></label><label>Progression note<textarea rows={3} value={progressionLabel} onChange={(event) => setProgressionLabel(event.target.value)} /></label><SurfaceActions><button className="button button--primary" onClick={() => void save()}>Save</button><button className="button button--quiet" onClick={() => setEditing(false)}>Cancel</button></SurfaceActions></div>
    </EntitySection>}
  </>;
}
