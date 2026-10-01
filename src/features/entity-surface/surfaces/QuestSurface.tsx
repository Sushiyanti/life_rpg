import type { Quest } from '../../../domain/world';
import { EntitySection, Field, RelationButton } from '../SurfacePrimitives';
import type { SurfaceRenderProps } from '../surface-types';

export function QuestSurface({ entity: quest, attachedConcepts, openSurface }: SurfaceRenderProps<Quest>) {
  return <>
    <EntitySection eyebrow="Identity" title="Quest details"><div className="entity-grid"><Field label="Status" value={quest.status.replaceAll('_', ' ')} /><Field label="Progress" value={`${quest.progress}%`} /><Field label="Type" value={quest.typeCode.replaceAll('_', ' ')} /><Field label="Record ID" value={quest.id} mono /></div><p className="entity-copy">{quest.description || 'No description has been recorded for this Quest.'}</p></EntitySection>
    <EntitySection eyebrow="Actual relationships" title="Attached Concepts">{attachedConcepts.length ? attachedConcepts.map((concept) => <RelationButton key={concept.id} label={`Concept · ${concept.typeCode.replaceAll('_', ' ')}`} name={concept.name} onClick={() => openSurface('concept', concept.id, { label: concept.name })} />) : <p className="muted">No active Concepts are attached to this Quest.</p>}</EntitySection>
  </>;
}
