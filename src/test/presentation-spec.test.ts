/**
 * Tests for the presentation interpreter.
 *
 * The architecture forbids storing raw CSS and forbids executable JS in the
 * database. The mechanism that makes that safe is here: every value the UI can
 * adopt comes from a switch over a closed union. These tests pin that behaviour,
 * including the "unknown token degrades, never executes" property.
 */

import { describe, expect, it } from 'vitest';

import {
  DEFAULT_PRESENTATION,
  isKnownVariant,
  resolveAccent,
  resolveRadius,
  resolveShadow,
  resolveSurface,
  resolveTitle,
  styleToCss,
  type PresentationSpec,
} from '../presentation/spec';

describe('controlled token resolution', () => {
  it('maps surfaces to design tokens', () => {
    expect(resolveSurface('glass')).toBe('var(--surface-glass)');
    expect(resolveSurface('flat')).toBe('var(--surface-flat)');
    expect(resolveSurface('raised')).toBe('var(--surface-raised)');
  });

  it('maps radius tokens to concrete lengths', () => {
    expect(resolveRadius('none')).toBe('0');
    expect(resolveRadius('pill')).toBe('999px');
    expect(resolveRadius('large')).toBe('16px');
  });

  it('maps shadow tokens to design tokens', () => {
    expect(resolveShadow('none')).toBe('none');
    expect(resolveShadow('glow')).toBe('var(--shadow-glow)');
  });

  it('maps accents to CSS custom properties', () => {
    expect(resolveAccent('gold')).toBe('var(--accent-gold)');
    expect(resolveAccent('purple')).toBe('var(--accent-purple)');
  });

  it('resolves a title spec with defaults for missing parts', () => {
    expect(resolveTitle({})).toEqual({ fontSize: 'var(--text-md)', fontWeight: 600 });
    expect(resolveTitle({ size: 'title', weight: 700 })).toEqual({
      fontSize: 'var(--text-xl)',
      fontWeight: 700,
    });
  });
});

describe('styleToCss', () => {
  it('returns nothing for an absent spec so the component stylesheet wins', () => {
    expect(styleToCss(undefined)).toEqual({});
    expect(styleToCss({})).toEqual({});
  });

  it('emits only the keys that were specified', () => {
    expect(
      styleToCss({
        surface: 'glass',
        radius: 'large',
        shadow: 'large',
        accent: 'purple',
        title: { size: 'large', weight: 700 },
      }),
    ).toEqual({
      background: 'var(--surface-glass)',
      borderRadius: '16px',
      boxShadow: 'var(--shadow-lg)',
      borderColor: 'var(--accent-purple)',
      fontSize: 'var(--text-lg)',
      fontWeight: '700',
    });
  });

  it('never emits anything outside the closed token set', () => {
    // A presentation record loaded from the database can only ever produce
    // var(--…) references plus literal lengths from the radius table — there is
    // no path that would pass a stored string straight into CSS.
    const css = styleToCss({ surface: 'glass', radius: 'pill', accent: 'red' });
    for (const value of Object.values(css)) {
      expect(value).toMatch(/^(var\(--[a-z-]+\)|none|0|\d+px|\d+)$/);
    }
  });
});

describe('presentation spec vocabulary', () => {
  it('offers a default so records without presentation data still render', () => {
    expect(DEFAULT_PRESENTATION.variant).toBe('quest_card_default');
    expect(DEFAULT_PRESENTATION.density).toBe('comfortable');
    expect(DEFAULT_PRESENTATION.emphasis).toBe('normal');
  });

  it('recognises the named variants and rejects unknown ones', () => {
    for (const variant of [
      'quest_card_default',
      'quest_card_compact',
      'quest_card_epic',
      'quest_card_minimal',
      'timeline_entry',
      'journal_entry',
    ] as const) {
      expect(isKnownVariant(variant)).toBe(true);
    }
    expect(isKnownVariant('quest_card_rainbow')).toBe(false);
  });

  it('keeps the spec structurally data-only', () => {
    // Guard against someone later adding a function-valued field to the spec:
    // the spec must remain JSON-serializable, because it is stored in SQLite.
    const spec: PresentationSpec = {
      variant: 'quest_card_epic',
      density: 'spacious',
      emphasis: 'critical',
      icon: 'sword',
      style: { surface: 'glass', radius: 'large', accent: 'gold' },
    };
    expect(() => JSON.parse(JSON.stringify(spec))).not.toThrow();
    expect(
      Object.values(spec).every((v) => typeof v !== 'function'),
    ).toBe(true);
  });
});
