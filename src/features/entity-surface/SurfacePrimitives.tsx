import type { ReactNode } from 'react';
import type { SurfaceContextValue } from './surface-types';

export function Field({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return <div className="entity-field"><span>{label}</span><strong className={mono ? 'selectable' : undefined}>{value || '—'}</strong></div>;
}

export function EntitySection({ eyebrow, title, children }: { eyebrow: string; title: string; children: ReactNode }) {
  return <section className="entity-section"><div><span className="eyebrow">{eyebrow}</span><h3>{title}</h3></div>{children}</section>;
}

export function RelationButton({ label, name, onClick }: { label: string; name: string; onClick: () => void }) {
  return <button className="entity-relation" onClick={onClick}><span>{label}</span><strong>{name}</strong><small>Open this related record →</small></button>;
}

export function SurfaceActions({ children }: { children: ReactNode }) {
  return <div className="entity-form__actions">{children}</div>;
}

export type SurfaceOpen = SurfaceContextValue['openSurface'];
