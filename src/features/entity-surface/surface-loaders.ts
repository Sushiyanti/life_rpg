import type { CoreClient } from '../../domain/ipc';
import type { Concept, Effect, Player, Quest } from '../../domain/world';
import type { SurfaceEntity, SurfaceLoadContext, SurfaceLoadResult } from './surface-types';

export type QuestSurfaceContext = { attachedConcepts: Concept[] };
export type ConceptSurfaceContext = { relatedConcepts: Concept[] };

const missing = <T extends SurfaceEntity, Context>(message: string): SurfaceLoadResult<T, Context> => ({ entity: null, error: message });

function belongsToPlayer(entity: { id?: string; playerId?: string }, playerId: string | null): boolean {
  return !playerId || entity.playerId === playerId || entity.id === playerId;
}

async function conceptsForIds(client: CoreClient, ids: string[], playerId: string | null): Promise<Concept[]> {
  const rows = await Promise.all(ids.map((id) => client.getConcept(id)));
  return rows.filter((concept): concept is Concept => Boolean(concept && belongsToPlayer(concept, playerId)));
}

export async function loadPlayer(id: string, context: SurfaceLoadContext): Promise<SurfaceLoadResult<Player, undefined>> {
  const entity = await context.client.getPlayer(id);
  if (!entity || !belongsToPlayer(entity, context.playerId)) return missing('This Player is no longer available in the current world.');
  return { entity, context: undefined };
}

export async function loadQuest(id: string, context: SurfaceLoadContext): Promise<SurfaceLoadResult<Quest, QuestSurfaceContext>> {
  const entity = await context.client.getQuest(id);
  if (!entity || !belongsToPlayer(entity, context.playerId)) return missing('This Quest is no longer available in the current world.');
  const associations = await context.client.listConceptAssociations({ entityKind: 'quest', entityId: id });
  const activeIds = associations.filter((association) => association.isActive).map((association) => association.conceptId);
  return { entity, context: { attachedConcepts: await conceptsForIds(context.client, activeIds, context.playerId) } };
}

export async function loadConcept(id: string, context: SurfaceLoadContext): Promise<SurfaceLoadResult<Concept, ConceptSurfaceContext>> {
  const entity = await context.client.getConcept(id);
  if (!entity || !belongsToPlayer(entity, context.playerId)) return missing('This Concept is no longer available in the current world.');
  const relationships = (await context.client.listConceptRelationships(id)).filter((relationship) => relationship.isActive);
  const relatedIds = relationships.map((relationship) => relationship.sourceConceptId === id ? relationship.targetConceptId : relationship.sourceConceptId);
  return { entity, context: { relatedConcepts: await conceptsForIds(context.client, relatedIds, context.playerId) } };
}

export async function loadEffect(id: string, context: SurfaceLoadContext): Promise<SurfaceLoadResult<Effect, undefined>> {
  if (!context.playerId) return missing('An active world is required to open this Effect.');
  const entity = (await context.client.listEffects(context.playerId)).find((effect) => effect.id === id);
  if (!entity) return missing('This Effect is no longer available in the current world.');
  return { entity, context: undefined };
}
