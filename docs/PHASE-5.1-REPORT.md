# Phase 5.1 — Workspace Presentation Capabilities

**Branch:** `phase-5.1-workspace-capabilities` (from `phase-5-workspaces`)  
**Schema:** 10 (forward migration 0010)  
**Boundary:** Player-owned presentation settings; canonical world records remain unchanged.

## Scope and invariants

Phase 5.1 extends the SQLite-backed Phase 5 workspace system into a more configurable presentation layer. Workspaces remain presentation-only, Player/world-owned records; panels are independent reusable instances, so multiple panels can use the same source with different titles and configurations. Panel or workspace changes never mutate canonical world state. No saved value is HTML, JavaScript, raw SQL, or an arbitrary predicate. Phase 6 was not started.

## Persistence and migration

- **Migration added:** `crates/lr-persistence/src/migrations/0010_phase51_workspace_capabilities.sql`.
- Migration 10 follows schema version 9, transactionally rebuilds `workspace_panels`, preserves existing panel IDs and presentation state, normalizes legacy filter aliases to canonical per-source values, removes the old `(workspace_id, panel_type)` uniqueness constraint, and adds typed filter/sort/layout fields and indexes. Prior migration files remain untouched.
- SQLite bounds panel source/variant/status/sort vocabularies, density, booleans, item count (1–50), recent-day filter (1–365), title length, and responsive span (1–2). The Rust domain validates each value against its source before persistence.
- Workspace, panel, and Concept-filter operations are Player-scoped. A panel Concept must belong to that same Player. The last workspace cannot be deleted; deleting a default workspace promotes a replacement atomically.
- Workspace lists, default choice, and panel definitions/configuration live in SQLite. Browser storage keeps only runtime selection/route hints and supports a one-time safe import of the old Phase 4 dashboard layout.

## Panel model and filters

Closed panel sources are `player`, `quests`, `skills`, `concepts`, `progress`, `effects`, `activity` (recorded Quest Sessions), `transactions` (ledger), and `journal` (Narrative Entries). Filter/status vocabularies are source-specific:

- Quest: `open`, `active`, `completed`, `abandoned`.
- Skill: `active`, `paused`, `completed`, `archived`.
- Concept and progress: `active`, `archived`.
- Effect: `active`, `inactive`.
- Recorded Session: `in_progress`, `completed`, `interrupted`.

Other supported, data-defined filters include active state where applicable, exact lowercase type codes, a Player-owned related Concept, and recent time windows from 1–365 days. The existing typed global search service remains the query executor; panel code only builds typed bounded queries. Session updates and direct Session-to-Concept filter matching use the established Session search projection and stored Concept relation.

Per-source sort choices cover name, status, create/update time, progress/level, Session start, and Transaction occurrence. Counts are bounded to 1–50. Registered presentation variants are cards, rows, compact, detailed, timeline, tree, and metrics only where that source supports them; density is cozy or compact.

Each instance persists title, order, one/two-column span, visibility, pin, and collapsed state. Responsive CSS stacks panels on narrow screens. Reordering supports drag-and-drop plus accessible up/down buttons. Panel visibility remains separate from contextual entity visibility, pinning, and lifecycle state.

## Builder and workspace lifecycle

The Dashboard builder supports template-based creation (Overview, Focus, Learning, Health, Review), rename, make-default, duplicate, delete, add/remove panels, repeated source instances, variant/density, source-specific status/type/Concept/recent filters, result count, pin/collapse/visibility, layout span, and order. The last-workspace delete action is disabled in the UI and independently rejected in the backend. Workspace deletion explains it removes presentation settings only. Empty, success, and error states are surfaced inline.

The active workspace is a per-Player runtime selection and survives restart when the ID still exists; otherwise the view falls back to the persisted default, then the first workspace. The persisted default is independent of the currently open workspace. Deleting the default promotes a surviving workspace.

## Import and export

Implemented as versioned JSON (`format: "life-rpg-workspace", version: 1`). Exports contain only the workspace name/template and explicit portable panel options—no Player, workspace, or panel database IDs, timestamps, or world records. Import strictly rejects missing/unknown fields, unsupported format/version/source/variant/filter combinations, malformed values, and out-of-range values. Export passes through the same validator. Import creates independent workspace and panel records.

## Verification

- `cargo fmt --all` — passed.
- `cargo test --workspace` — passed: **46 persistence tests**, plus application/domain/contracts and Tauri tests. Coverage includes v9→v10 migration and status normalization, repeated sources, Player ownership, default lifecycle, Concept-filter ownership, Session search filters and updates, workspace file reopen durability, domain vocabulary, and IPC DTO shape.
- `./node_modules/.bin/tsc --noEmit` — passed.
- `./node_modules/.bin/vitest run` — passed: **54 tests across 8 files**, including repeated-source builder behavior, bounded filters, source-to-search mapping, IPC payloads, and strict import/export validation.
- `./node_modules/.bin/vite build` — passed.
- `git diff --check` — passed during final review.
- No third-party UI/drag-drop library, external asset, or component reference was used; the builder uses local React components, native drag events, and explicit button controls.

## Deferred work and limitations

- Filter choices are finite and source-specific; there is no user-authored boolean expression, arbitrary column selector, raw SQL, or executable configuration.
- Registered variants render current typed world projections; user-defined rendering code and plugins are intentionally absent.
- Layout is a responsive two-column span/order model, not an infinite canvas or arbitrary grid editor.
- Panel search remains within existing bounded search/result limits. This phase does not redesign Search, add a daemon/background service, or introduce cloud sync. Import/export transfers configuration only, not world records or entity identity mappings.
- Phase 6 and broader game mechanics remain explicitly out of scope.
