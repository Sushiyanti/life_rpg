import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
} from 'react';
import type { CoreClient } from '../../domain/ipc';
import type { Player } from '../../domain/world';
import { surfaceRegistry } from './surface-registry';
import type {
  EntitySurfaceRecord,
  SurfaceLoadResult,
  SurfaceContextValue,
  SurfaceKind,
  SurfaceMode,
} from './surface-types';
import './EntitySurface.css';

type ProviderProps = {
  children: ReactNode;
  client: CoreClient;
  player: Player | null;
  overview?: unknown;
  concepts?: unknown;
  onRefresh: () => Promise<void>;
};

const SurfaceContext = createContext<SurfaceContextValue | null>(null);

export function useEntitySurface(): SurfaceContextValue {
  const context = useContext(SurfaceContext);
  if (!context) {
    throw new Error('useEntitySurface must be used inside EntitySurfaceProvider');
  }
  return context;
}

export function useOptionalEntitySurface(): SurfaceContextValue {
  return useContext(SurfaceContext) ?? {
    openSurface: () => undefined,
    closeSurface: () => undefined,
    backSurface: () => undefined,
    stack: [],
  };
}

export function EntitySurfaceProvider({ children, client, player, onRefresh }: ProviderProps) {
  const [stack, setStack] = useState<EntitySurfaceRecord[]>([]);

  const openSurface = useCallback(
    (
      kind: SurfaceKind,
      id: string,
      options: { mode?: SurfaceMode; label?: string } = {},
    ) => {
      if (!id) return;
      const returnFocus = document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
      setStack((current) => [
        ...current,
        {
          kind,
          id,
          mode: options.mode ?? 'view',
          label: options.label,
          returnFocus,
        },
      ]);
    },
    [],
  );

  const restoreFocus = useCallback(
    (record?: EntitySurfaceRecord) => {
      window.setTimeout(() => record?.returnFocus?.focus(), 0);
    },
    [],
  );

  const closeSurface = useCallback(() => {
    setStack((current) => {
      restoreFocus(current[0]);
      return [];
    });
  }, [restoreFocus]);

  const backSurface = useCallback(() => {
    setStack((current) => {
      const closing = current[current.length - 1];
      restoreFocus(closing);
      return current.length > 1 ? current.slice(0, -1) : [];
    });
  }, [restoreFocus]);

  const context = useMemo(
    () => ({ openSurface, closeSurface, backSurface, stack }),
    [openSurface, closeSurface, backSurface, stack],
  );

  return (
    <SurfaceContext.Provider value={context}>
      {children}
      {stack.length > 0 && (
        <SurfaceStack client={client} player={player} onRefresh={onRefresh} />
      )}
    </SurfaceContext.Provider>
  );
}

function SurfaceStack({
  client,
  player,
  onRefresh,
}: {
  client: CoreClient;
  player: Player | null;
  onRefresh: () => Promise<void>;
}) {
  const { stack, closeSurface, backSurface, openSurface } = useEntitySurface();
  const current = stack[stack.length - 1];
  if (!current) return null;

  const descriptor = surfaceRegistry[current.kind];
  const unsupported = !descriptor;
  const [loaded, setLoaded] = useState<SurfaceLoadResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState(current.mode === 'edit');
  const [copyState, setCopyState] = useState('Copy');
  const [message, setMessage] = useState('');
  const headingRef = useRef<HTMLHeadingElement>(null);
  const dialogRef = useRef<HTMLElement>(null);

  useEffect(() => {
    let live = true;
    setLoading(true);
    setLoaded(null);
    setEditing(current.mode === 'edit');
    setMessage('');
    if (!descriptor) {
      setLoading(false);
      return () => { live = false; };
    }

    void descriptor
      .load(current.id, { client, playerId: player?.id ?? null })
      .then((result) => {
        if (!live) return;
        setLoaded(result);
        setLoading(false);
      })
      .catch(() => {
        if (!live) return;
        setLoaded({
          entity: null,
          error: 'The record could not be loaded.',
        });
        setLoading(false);
      });

    return () => {
      live = false;
    };
  }, [client, current.id, current.kind, current.mode, descriptor, player?.id]);

  useEffect(() => {
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    headingRef.current?.focus();
    return () => {
      document.body.style.overflow = previousOverflow;
    };
  }, [current.id, current.kind]);

  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      backSurface();
      return;
    }
    if (event.key !== 'Tab' || !dialogRef.current) return;

    const focusable = Array.from(
      dialogRef.current.querySelectorAll<HTMLElement>(
        'button, input, textarea, select, [tabindex]:not([tabindex="-1"])',
      ),
    ).filter((element) => !element.hasAttribute('disabled'));
    if (!focusable.length) return;

    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (!first || !last) return;
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  const copy = async () => {
    if (!descriptor || !loaded?.entity) return;
    try {
      await navigator.clipboard?.writeText(descriptor.copy(loaded.entity));
      setCopyState('Copied');
      window.setTimeout(() => setCopyState('Copy'), 1400);
    } catch {
      setCopyState('Copy unavailable');
    }
  };

  const onSaved = async (savedMessage: string) => {
    await onRefresh();
    setMessage(savedMessage);
  };

  const parent = stack.length > 1 ? stack[stack.length - 2] : null;
  const currentLoaded = loaded?.entity?.id === current.id ? loaded : null;
  const rendererProps = currentLoaded?.entity
    ? {
        entity: currentLoaded.entity,
        context: currentLoaded.context,
        client,
        editing,
        setEditing,
        onSaved,
        openSurface,
      }
    : null;

  return (
    <div className="entity-surface-layer" data-testid="entity-surface-layer">
      <button
        className="entity-surface-backdrop"
        aria-label="Close entity surface"
        onClick={closeSurface}
      />
      <aside
        className="entity-surface"
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="entity-surface-title"
        onKeyDown={onKeyDown}
      >
        <header className="entity-surface__header">
          <div className="entity-surface__heading">
            {parent && (
              <button className="text-link" onClick={backSurface}>
                ← Back to {parent.label ?? `${parent.kind} record`}
              </button>
            )}
            <span className="eyebrow">{current.kind} · contextual surface</span>
            <h2 id="entity-surface-title" tabIndex={-1} ref={headingRef}>
              {loading
                ? 'Loading…'
                : unsupported
                  ? 'Surface not available'
                  : currentLoaded?.entity
                  ? descriptor!.title(currentLoaded.entity)
                  : current.label ?? `${current.kind} record`}
            </h2>
            <p>
              {unsupported
                ? `The “${current.kind}” entity type does not yet have a contextual viewer.`
                : currentLoaded?.entity
                ? 'Inspect this record without leaving your current workspace.'
                : currentLoaded?.error ?? 'This record could not be loaded.'}
            </p>
          </div>
          <button
            className="entity-surface__close"
            aria-label="Close entity surface"
            onClick={closeSurface}
          >
            ×
          </button>
        </header>

        <div className="entity-surface__toolbar">
          <span className="type-pill">{editing ? 'Editing' : 'Details'}</span>
          <div>
            {descriptor?.renderEditor && currentLoaded?.entity && (
              <button
                className="button button--small button--cyan"
                onClick={() => setEditing((value) => !value)}
              >
                {editing ? 'Cancel edit' : 'Edit'}
              </button>
            )}
            {currentLoaded?.entity && !editing && (
              <button className="button button--small" onClick={() => void copy()}>
                {copyState}
              </button>
            )}
          </div>
        </div>

        <div className="entity-surface__body">
          {unsupported ? (
            <div className="surface-empty" data-testid="unsupported-surface">
              <p>This type has no contextual viewer yet.</p>
              <strong className="selectable">Requested type: {current.kind}</strong>
              <button className="button button--primary" onClick={closeSurface}>Close</button>
              {parent && <button className="button button--quiet" onClick={backSurface}>Back</button>}
            </div>
          ) : loading || !currentLoaded ? (
            <p className="muted">Resolving the saved record…</p>
          ) : currentLoaded.entity && rendererProps ? (
            <>
              {message && <p className="inline-feedback" role="status">{message}</p>}
              {editing && descriptor?.renderEditor
                ? descriptor!.renderEditor(rendererProps)
                : descriptor!.render(rendererProps)}
            </>
          ) : (
            <div className="surface-empty">
              <p>{currentLoaded.error ?? 'This record is no longer available.'}</p>
              <button className="button button--primary" onClick={closeSurface}>Close</button>
              {parent && <button className="button button--quiet" onClick={backSurface}>Back</button>}
            </div>
          )}
        </div>
      </aside>
    </div>
  );
}
