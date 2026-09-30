# Phase 6.0a — Contextual surface hardening

**Base:** `4160379` (`phase-6.0-ui-foundation`)
**Branch:** `phase-6.0a-surface-hardening`

## Implemented

- Replaced preloaded-array-only surface lookup with a typed resolver that refreshes the saved world identity before rendering a Quest, Concept, or Effect. IDs are authoritative; array position is never used as identity.
- Added typed `get_concept` and `set_concept_active` IPC paths so the reusable Concept renderer can load and edit a real record through the existing application boundary.
- Quest surfaces now discover attached Concepts from persisted `ConceptAssociation` rows. They no longer open the first Concept in memory.
- Concept surfaces now load persisted Concept-to-Concept relationships and open the selected related Concept by its actual ID. Multiple relationships remain independently navigable.
- Generalized the editing path to Player and Concept surfaces with explicit edit, save, and cancel states.
- Replaced raw JSON copy output with human-readable entity summaries.
- Preserved the mounted workspace and route while surfaces are open; nested close returns to the parent, then to the original trigger.
- Hardened independent scroll ownership with a constrained surface body and `overscroll-behavior: contain`.

## Historical constraints applied

- Phase 2.1: identity and state changes remain explicit and auditable; the surface does not invent or infer relationships.
- Phase 3.5: Concepts remain first-class entities with typed relationships rather than being flattened into a generic record.
- Phase 3.6: contextual visibility and player-managed progression are kept separate from surface presentation and editing.
- Phase 4.1: contextual actions carry the actual entity anchor; no context-free or index-based action is used.
- Phase 5: the workspace remains mounted and its persistent state is not replaced by route-driven remounting.

## Verified

- `npm run typecheck` — passed.
- `npm test -- --run src/test/EntitySurface.test.tsx` — 5 passed, covering Quest A/B identity, Effect C identity, actual Quest→Concept B attachment, Concept B→Concept C nesting, parent restoration, Player-independent Concept editing, human-readable copy, focus restoration, route/context preservation, and body scroll locking.
- Full frontend regression suite — passed after the final test correction: 50 tests across 7 files.
- `npm run build:vite` — passed; production bundle generated.
- `git diff --check` — passed.

## Not verified

- Native Rust/Tauri validation was not run because `cargo` is unavailable in this environment (`cargo: command not found`).
- The human-style acceptance flow against a live persisted desktop world was not completed in this environment. The automated tests use typed world-shaped fixtures and persisted-relationship-shaped association data; a real Tauri session is still required to visually verify long-content scrolling, multiple overflowing panels, and background/viewport behavior end to end.
