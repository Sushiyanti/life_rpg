import type { Effect } from '../../../domain/world';
import { EntitySection, Field } from '../SurfacePrimitives';
import type { SurfaceRenderProps } from '../surface-types';

export function EffectSurface({ entity: effect }: SurfaceRenderProps<Effect, undefined>) {
  return <EntitySection eyebrow="Current state" title="Effect details"><div className="entity-grid"><Field label="Status" value={effect.deactivatedAt ? 'Inactive' : 'Active'} /><Field label="Type" value={effect.typeCode.replaceAll('_', ' ')} /><Field label="Intensity" value={String(effect.intensity)} /><Field label="Record ID" value={effect.id} mono /></div><p className="entity-copy">{effect.description || 'No description has been recorded for this Effect.'}</p></EntitySection>;
}
