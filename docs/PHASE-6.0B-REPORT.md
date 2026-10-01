# Phase 6.0b — Extensible contextual surfaces

**Base:** Phase 6.0a surface hardening checkpoint
**Branch:** `phase-6.0b-extensible-surfaces`

## Delivered

- Decomposed the former monolithic Entity Surface into:
  - `SurfaceProvider.tsx` for stack lifecycle, focus, Escape, body locking, Back/Close, and stale-state handling.
  - `surface-registry.tsx` for additive entity descriptors.
  - `surface-loaders.ts` for entity-specific lazy resolution.
  - `SurfacePrimitives.tsx` for common field/section/relation presentation.
  - one renderer module each for Player, Quest, Concept, and Effect.
  - `EntitySurface.tsx` compatibility facade for existing imports.
- Replaced broad overview/scanning shortcuts with:
  - `get_quest` for single Quest identity resolution.
  - `list_effects` for player-scoped Effect resolution.
  - filtered `list_concept_associations` client calls for actual Quest-to-Concept relationships.
  - relationship-target Concept reads for Concept-to-Concept navigation.
- Added explicit stale/missing states, human-readable copy, accessible status feedback, Save/Cancel editing, nested Back navigation, and stack-wide Close.
- Added a keyboard focus loop, Escape handling, body-scroll ownership, contained surface overscroll, and responsive drawer/dialog layout.
- Updated the UX Constitution and added `PHASE-HISTORY-LESSONS.md` so future work extends the registry instead of reopening infrastructure.

## Validation

- `npm run typecheck` — passed.
- `npm test -- --run` — passed: 7 files, 52 tests.
- `npm run build:vite` — passed.
- `git diff --check` — passed.
- Rust validation was not run because `cargo` and `rustc` are not installed in the current sandbox. The TypeScript boundary, command names, and generated frontend build were validated; native Tauri compilation remains an explicit environment limitation.

## Behavioral guarantees covered by tests

- Entity IDs are resolved directly rather than by array position.
- Quest A opens the actual attached Concept B, and Concept B opens the actual related Concept C.
- Effect C is loaded from the player-scoped Effect query and copied as human-readable text.
- Concept Active/Inactive editing calls the typed mutation and reports success.
- Nested Back unwinds one surface; Close removes the complete stack.
- Body scroll locks while a surface is open, Tab remains contained, and focus returns to the original trigger.
- A new registry descriptor can be registered without changing generic stack mechanics.
- Filtered association arguments are pinned at the IPC boundary.
