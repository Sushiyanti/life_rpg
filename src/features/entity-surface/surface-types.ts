import type { ReactNode } from 'react';
import type { CoreClient } from '../../domain/ipc';
import type { Concept, Effect, Player, Quest } from '../../domain/world';

export type SurfaceKind = 'player' | 'quest' | 'concept' | 'effect' | (string & {});
export type SurfaceMode = 'view' | 'edit';
export type SurfaceEntity = Player | Quest | Concept | Effect;

export type EntitySurfaceRecord = {
  kind: SurfaceKind;
  id: string;
  mode: SurfaceMode;
  label?: string;
  returnFocus?: HTMLElement | null;
};

export type SurfaceContextValue = {
  openSurface: (kind: SurfaceKind, id: string, options?: { mode?: SurfaceMode; label?: string }) => void;
  closeSurface: () => void;
  backSurface: () => void;
  stack: EntitySurfaceRecord[];
};

export type SurfaceLoadContext = {
  client: CoreClient;
  playerId: string | null;
};

export type SurfaceLoadResult = {
  entity: SurfaceEntity | null;
  attachedConcepts: Concept[];
  relatedConcepts: Concept[];
  error?: string;
};

export type SurfaceRenderProps<T extends SurfaceEntity> = {
  entity: T;
  attachedConcepts: Concept[];
  relatedConcepts: Concept[];
  client: CoreClient;
  editing: boolean;
  setEditing: (editing: boolean) => void;
  onSaved: (message: string) => Promise<void>;
  openSurface: SurfaceContextValue['openSurface'];
};

export type SurfaceDescriptor<T extends SurfaceEntity = SurfaceEntity> = {
  kind: SurfaceKind;
  load: (id: string, context: SurfaceLoadContext) => Promise<SurfaceLoadResult>;
  title: (entity: T) => string;
  copy: (entity: T) => string;
  render: (props: SurfaceRenderProps<T>) => ReactNode;
  renderEditor?: (props: SurfaceRenderProps<T>) => ReactNode;
};
