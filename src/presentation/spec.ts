/**
 * Presentation-layer vocabulary.
 *
 * The requirement is explicit: the *same* domain object must be renderable in
 * many ways, and presentation must be data rather than hardcoded branches. So
 * this module defines the controlled schema that the database will eventually
 * store — not arbitrary CSS, not executable JavaScript, just closed sets of
 * semantic tokens that the frontend interprets.
 *
 * Phase 1 ships the vocabulary and one interpreter. Phase 4 wires it to stored
 * presentation records and a style sandbox.
 */

/**
 * How a thing should be drawn. Deliberately a closed union: if a variant is not
 * listed, it is not selectable, which is what keeps the DB from becoming a
 * place where arbitrary code hides.
 */
export type PresentationVariant =
  | 'quest_card_default'
  | 'quest_card_compact'
  | 'quest_card_epic'
  | 'quest_card_minimal'
  | 'timeline_entry'
  | 'journal_entry';

/** Spacing/typography density. */
export type Density = 'compact' | 'comfortable' | 'spacious';

/** Visual priority. */
export type Emphasis = 'low' | 'normal' | 'high' | 'critical';

/** Controlled surface treatments, mapped to CSS by `resolveSurface`. */
export type Surface = 'flat' | 'raised' | 'glass';

/** Controlled radius scale. */
export type Radius = 'none' | 'small' | 'medium' | 'large' | 'pill';

/** Controlled shadow scale. */
export type Shadow = 'none' | 'small' | 'medium' | 'large' | 'glow';

/** Controlled accent palette. */
export type Accent = 'neutral' | 'gold' | 'purple' | 'cyan' | 'green' | 'red';

/** Controlled text size scale. */
export type FontSize = 'small' | 'medium' | 'large' | 'title';

/** A style description stored in the database and interpreted here. */
export interface StyleSpec {
  surface?: Surface;
  radius?: Radius;
  shadow?: Shadow;
  accent?: Accent;
  title?: {
    size?: FontSize;
    weight?: number;
  };
}

/** Presentation metadata attached to a domain object. */
export interface PresentationSpec {
  variant: PresentationVariant;
  density: Density;
  emphasis: Emphasis;
  icon?: string;
  style?: StyleSpec;
}

/** Every variant the frontend knows how to render. */
export const PRESENTATION_VARIANTS = [
  'quest_card_default',
  'quest_card_compact',
  'quest_card_epic',
  'quest_card_minimal',
  'timeline_entry',
  'journal_entry',
] as const satisfies readonly PresentationVariant[];

/**
 * Type guard for a stored variant token.
 *
 * This is the runtime half of the closed union: a row in the database could
 * contain anything, so before a component branches on a variant it must be able
 * to prove the token is one it understands. Unknown tokens fall back to
 * `DEFAULT_PRESENTATION` rather than rendering nothing.
 */
export function isKnownVariant(value: unknown): value is PresentationVariant {
  return (
    typeof value === 'string' &&
    (PRESENTATION_VARIANTS as readonly string[]).includes(value)
  );
}

/** Sensible default so a record with no presentation data still renders. */
export const DEFAULT_PRESENTATION: PresentationSpec = {
  variant: 'quest_card_default',
  density: 'comfortable',
  emphasis: 'normal',
};

/** Maps a controlled token to a CSS custom-property value. */
export function resolveSurface(surface: Surface): string {
  switch (surface) {
    case 'flat':
      return 'var(--surface-flat)';
    case 'raised':
      return 'var(--surface-raised)';
    case 'glass':
      return 'var(--surface-glass)';
  }
}

/** Maps a controlled radius token to a CSS value. */
export function resolveRadius(radius: Radius): string {
  switch (radius) {
    case 'none':
      return '0';
    case 'small':
      return '6px';
    case 'medium':
      return '10px';
    case 'large':
      return '16px';
    case 'pill':
      return '999px';
  }
}

/** Maps a controlled shadow token to a CSS value. */
export function resolveShadow(shadow: Shadow): string {
  switch (shadow) {
    case 'none':
      return 'none';
    case 'small':
      return 'var(--shadow-sm)';
    case 'medium':
      return 'var(--shadow-md)';
    case 'large':
      return 'var(--shadow-lg)';
    case 'glow':
      return 'var(--shadow-glow)';
  }
}

/** Maps a controlled accent token to a CSS custom property. */
export function resolveAccent(accent: Accent): string {
  return `var(--accent-${accent})`;
}

/** Maps controlled text sizing into a CSS font shorthand pair. */
export function resolveTitle(title: NonNullable<StyleSpec['title']>): {
  fontSize: string;
  fontWeight: number;
} {
  const sizes: Record<FontSize, string> = {
    small: 'var(--text-sm)',
    medium: 'var(--text-md)',
    large: 'var(--text-lg)',
    title: 'var(--text-xl)',
  };
  return {
    fontSize: sizes[title.size ?? 'medium'],
    fontWeight: title.weight ?? 600,
  };
}

/**
 * Turn a `StyleSpec` into a plain `style` object the component can spread.
 *
 * This is the *only* interpreter. It never emits raw user-supplied CSS — every
 * value comes from a switch over a closed union, so a malicious or buggy row in
 * the database can at worst produce a fallback, never arbitrary styling.
 */
export function styleToCss(spec: StyleSpec | undefined): Record<string, string> {
  if (!spec) return {};
  const css: Record<string, string> = {};

  if (spec.surface) css.background = resolveSurface(spec.surface);
  if (spec.radius) css.borderRadius = resolveRadius(spec.radius);
  if (spec.shadow) css.boxShadow = resolveShadow(spec.shadow);
  if (spec.accent) css.borderColor = resolveAccent(spec.accent);
  if (spec.title) {
    const t = resolveTitle(spec.title);
    css.fontSize = t.fontSize;
    css.fontWeight = String(t.fontWeight);
  }

  return css;
}
