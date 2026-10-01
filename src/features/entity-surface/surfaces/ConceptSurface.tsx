import { useEffect, useState } from 'react';
import type { Concept } from '../../../domain/world';
import { EntitySection, Field, RelationButton, SurfaceActions } from '../SurfacePrimitives';
import type { SurfaceRenderProps } from '../surface-types';

export function ConceptSurface({ entity: concept, relatedConcepts, client, editing, setEditing, onSaved, openSurface }: SurfaceRenderProps<Concept>) {
  const [active, setActive] = useState(concept.isActive);
  useEffect(() => setActive(concept.isActive), [concept]);

  async function save() {
    await client.setConceptActive(concept.id, active);
    await onSaved('Concept active state saved.');
    setEditing(false);
  }

  return <>
    <EntitySection eyebrow="Identity" title="Concept details"><div className="entity-grid"><Field label="Type" value={concept.typeCode.replaceAll('_', ' ')} /><Field label="Status" value={concept.isActive ? 'Active' : 'Inactive'} /><Field label="Record ID" value={concept.id} mono /></div><p className="entity-copy">{concept.description || 'This Concept is ready to collect meaning and relationships.'}</p></EntitySection>
    <EntitySection eyebrow="Actual relationships" title="Related Concepts">{relatedConcepts.length ? relatedConcepts.map((related) => <RelationButton key={related.id} label={`Concept · ${related.typeCode.replaceAll('_', ' ')}`} name={related.name} onClick={() => openSurface('concept', related.id, { label: related.name })} />) : <p className="muted">No active related Concepts are recorded.</p>}</EntitySection>
    {editing && <EntitySection eyebrow="Edit mode" title="Concept active state"><div className="entity-form"><label className="entity-check"><input type="checkbox" checked={active} onChange={(event) => setActive(event.target.checked)} /> Active in this world</label><SurfaceActions><button className="button button--primary" onClick={() => void save()}>Save</button><button className="button button--quiet" onClick={() => setEditing(false)}>Cancel</button></SurfaceActions></div></EntitySection>}
  </>;
}
