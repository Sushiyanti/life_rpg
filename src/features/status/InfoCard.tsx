/**
 * `InfoCard` — a titled surface.
 *
 * Written against the controlled style schema in `presentation/spec.ts`: it
 * receives a semantic `accent` token and resolves it to a CSS variable through
 * the interpreter rather than hardcoding a colour. When Phase 4 makes
 * presentation data-driven, this component needs no changes.
 */

import type { ReactNode } from 'react';

import type { Accent } from '../../presentation/spec';
import { resolveAccent } from '../../presentation/spec';

interface InfoCardProps {
  title: string;
  accent?: Accent;
  children: ReactNode;
}

export function InfoCard({ title, accent = 'neutral', children }: InfoCardProps) {
  return (
    <article
      className="info-card"
      style={{ borderTopColor: resolveAccent(accent) }}
      data-accent={accent}
    >
      <h3 className="info-card__title">{title}</h3>
      <div className="info-card__body">{children}</div>
    </article>
  );
}
