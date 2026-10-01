import { ConceptSurface } from './surfaces/ConceptSurface';
import { EffectSurface } from './surfaces/EffectSurface';
import { PlayerSurface } from './surfaces/PlayerSurface';
import { QuestSurface } from './surfaces/QuestSurface';
import { loadConcept, loadEffect, loadPlayer, loadQuest } from './surface-loaders';
import type { SurfaceDescriptor, SurfaceEntity } from './surface-types';
import type { Concept, Effect, Player, Quest } from '../../domain/world';

const readable = (value: string) => value.replaceAll('_', ' ');

export const surfaceRegistry: Record<string, SurfaceDescriptor> = {
  player: {
    kind: 'player',
    load: loadPlayer,
    title: (entity) => (entity as Player).name,
    copy: (entity) => {
      const player = entity as Player;
      return `Player: ${player.name}\nLevel: ${player.level}${player.levelName ? ` · ${player.levelName}` : ''}\nExperience: ${player.currentXp.toLocaleString()}\n${player.description ?? ''}`;
    },
    render: (props) => <PlayerSurface {...props} entity={props.entity as Player} />,
    renderEditor: (props) => <PlayerSurface {...props} entity={props.entity as Player} />,
  },
  quest: {
    kind: 'quest',
    load: loadQuest,
    title: (entity) => (entity as Quest).title,
    copy: (entity) => {
      const quest = entity as Quest;
      return `Quest: ${quest.title}\nStatus: ${readable(quest.status)}\nProgress: ${quest.progress}%\n${quest.description ?? ''}`;
    },
    render: (props) => <QuestSurface {...props} entity={props.entity as Quest} />,
  },
  concept: {
    kind: 'concept',
    load: loadConcept,
    title: (entity) => (entity as Concept).name,
    copy: (entity) => {
      const concept = entity as Concept;
      return `Concept: ${concept.name}\nType: ${readable(concept.typeCode)}\nStatus: ${concept.isActive ? 'Active' : 'Inactive'}\n${concept.description ?? ''}`;
    },
    render: (props) => <ConceptSurface {...props} entity={props.entity as Concept} />,
    renderEditor: (props) => <ConceptSurface {...props} entity={props.entity as Concept} />,
  },
  effect: {
    kind: 'effect',
    load: loadEffect,
    title: (entity) => (entity as Effect).name,
    copy: (entity) => {
      const effect = entity as Effect;
      return `Effect: ${effect.name}\nType: ${readable(effect.typeCode)}\nStatus: ${effect.deactivatedAt ? 'Inactive' : 'Active'}\nIntensity: ${effect.intensity}\n${effect.description ?? ''}`;
    },
    render: (props) => <EffectSurface {...props} entity={props.entity as Effect} />,
  },
};

export function registerSurface<T extends SurfaceEntity>(descriptor: SurfaceDescriptor<T>): SurfaceDescriptor<T> {
  surfaceRegistry[descriptor.kind] = descriptor as SurfaceDescriptor;
  return descriptor;
}
