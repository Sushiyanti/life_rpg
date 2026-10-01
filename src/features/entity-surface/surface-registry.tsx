import { ConceptSurface } from './surfaces/ConceptSurface';
import { EffectSurface } from './surfaces/EffectSurface';
import { PlayerSurface } from './surfaces/PlayerSurface';
import { QuestSurface } from './surfaces/QuestSurface';
import { loadConcept, loadEffect, loadPlayer, loadQuest, type ConceptSurfaceContext, type QuestSurfaceContext } from './surface-loaders';
import { defineSurface, type SurfaceDescriptor, type SurfaceEntity } from './surface-types';
import type { Concept, Effect, Player, Quest } from '../../domain/world';

const readable = (value: string) => value.replaceAll('_', ' ');

export const surfaceRegistry: Record<string, SurfaceDescriptor> = {
  player: defineSurface<Player, undefined>({
    kind: 'player',
    load: loadPlayer,
    title: (entity) => entity.name,
    copy: (entity) => `Player: ${entity.name}\nLevel: ${entity.level}${entity.levelName ? ` · ${entity.levelName}` : ''}\nExperience: ${entity.currentXp.toLocaleString()}\n${entity.description ?? ''}`,
    render: (props) => <PlayerSurface {...props} />,
    renderEditor: (props) => <PlayerSurface {...props} />,
  }),
  quest: defineSurface<Quest, QuestSurfaceContext>({
    kind: 'quest',
    load: loadQuest,
    title: (entity) => entity.title,
    copy: (entity) => `Quest: ${entity.title}\nStatus: ${readable(entity.status)}\nProgress: ${entity.progress}%\n${entity.description ?? ''}`,
    render: (props) => <QuestSurface {...props} />,
  }),
  concept: defineSurface<Concept, ConceptSurfaceContext>({
    kind: 'concept',
    load: loadConcept,
    title: (entity) => entity.name,
    copy: (entity) => `Concept: ${entity.name}\nType: ${readable(entity.typeCode)}\nStatus: ${entity.isActive ? 'Active' : 'Inactive'}\n${entity.description ?? ''}`,
    render: (props) => <ConceptSurface {...props} />,
    renderEditor: (props) => <ConceptSurface {...props} />,
  }),
  effect: defineSurface<Effect, undefined>({
    kind: 'effect',
    load: loadEffect,
    title: (entity) => entity.name,
    copy: (entity) => `Effect: ${entity.name}\nType: ${readable(entity.typeCode)}\nStatus: ${entity.deactivatedAt ? 'Inactive' : 'Active'}\nIntensity: ${entity.intensity}\n${entity.description ?? ''}`,
    render: (props) => <EffectSurface {...props} />,
  }),
};

export function registerSurface(descriptor: SurfaceDescriptor): SurfaceDescriptor {
  surfaceRegistry[descriptor.kind] = descriptor;
  return descriptor;
}

export type { SurfaceEntity };
