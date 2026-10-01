import type { ReactNode } from 'react';
import type { CoreClient } from '../../domain/ipc';

export type SurfaceKind = string;
export type SurfaceMode = 'view' | 'edit';
export type SurfaceEntity = { id: string };

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

export type SurfaceLoadResult<T extends SurfaceEntity = SurfaceEntity, Context = unknown> = {
  entity: T | null;
  context?: Context;
  error?: string;
};

export type SurfaceRenderProps<T extends SurfaceEntity = SurfaceEntity, Context = unknown> = {
  entity: T;
  context: Context;
  client: CoreClient;
  editing: boolean;
  setEditing: (editing: boolean) => void;
  onSaved: (message: string) => Promise<void>;
  openSurface: SurfaceContextValue['openSurface'];
};

/** The typed definition used at an entity-specific registration boundary. */
export type SurfaceDescriptorDefinition<T extends SurfaceEntity, Context> = {
  kind: SurfaceKind;
  load: (id: string, context: SurfaceLoadContext) => Promise<SurfaceLoadResult<T, Context>>;
  title: (entity: T) => string;
  copy: (entity: T) => string;
  render: (props: SurfaceRenderProps<T, Context>) => ReactNode;
  renderEditor?: (props: SurfaceRenderProps<T, Context>) => ReactNode;
};

/** Erased descriptor consumed by generic stack infrastructure. */
export type SurfaceDescriptor = {
  kind: SurfaceKind;
  load: (id: string, context: SurfaceLoadContext) => Promise<SurfaceLoadResult>;
  title: (entity: SurfaceEntity) => string;
  copy: (entity: SurfaceEntity) => string;
  render: (props: SurfaceRenderProps) => ReactNode;
  renderEditor?: (props: SurfaceRenderProps) => ReactNode;
};

/**
 * Erases a descriptor only at the registry boundary. Entity-specific modules
 * retain their concrete entity/context types; generic infrastructure does not.
 */
export function defineSurface<T extends SurfaceEntity, Context>(
  definition: SurfaceDescriptorDefinition<T, Context>,
): SurfaceDescriptor {
  return {
    kind: definition.kind,
    load: async (id, context) => {
      const result = await definition.load(id, context);
      return { entity: result.entity, context: result.context, error: result.error };
    },
    title: (entity) => definition.title(entity as T),
    copy: (entity) => definition.copy(entity as T),
    render: (props) => definition.render({ ...props, entity: props.entity as T, context: props.context as Context }),
    renderEditor: definition.renderEditor
      ? (props) => definition.renderEditor!({ ...props, entity: props.entity as T, context: props.context as Context })
      : undefined,
  };
}
