# Phase 6 — Player Experience

**Branch:** `phase-6-player-experience`
**Base:** `phase-5.2-workspace-transfer-integrity`
**Scope:** A player-facing interaction layer over the existing local world model. No new domain entity, database migration, IPC command, or backend service was introduced.

## Product boundary

> Phase 6 improves interaction with the existing world; it does not redefine canonical world semantics merely to make the UI more game-like.

The Player Hub makes the selected Player's recorded state, current objectives, real Sessions, Effects, Concepts/progress, Skills, notes, dated activity, and persistent workspace reachable from one place. These are queries/presentations of existing records, not a new dashboard aggregate. Player-authored levels remain independent from XP. Time and duration are displayed only when the corresponding timestamps have actually been recorded. The UI invents no streaks, missing days, stats, progression, or activity.

## Player Hub, States, and navigation

- The first usable route after selecting/creating a Player is **Player Hub**; the established Overview remains available and the saved workspace is reachable from the Hub.
- The Hub names the active Player/world and distinguishes Player-authored level from recorded XP. It exposes real Quests, Sessions, Effects, Skill practice, Concept progress, authored stats, notes, snapshots, and recent activity with informative empty states.
- Hub and workspace rows open their exact world records in the existing Player-scoped Explorer. Return-to-origin preserves the initiating view. Explorer labels record lifecycle separately from domain status and identifies visibility as a presentation preference.
- A Session detail view follows only the Quest, Skill, and Concept identifiers actually recorded on that Session. Concept navigation continues through typed existing Concept associations; no universal graph was added. Reading or navigating records does not issue world mutations.
- The Hub embeds the same Player-owned persistent Overview workspace and panel configuration. Panel setup and workspace data remain in SQLite; hiding a panel remains distinct from hiding an entity or changing its lifecycle.

## Quest, Session, and capture interactions

- Quest controls expose the valid explicit transition (`open` → start); already active Quests offer Session start and explicit completion. They do not claim a Quest is complete merely because its Session ended.
- A Session starts only when the Player invokes the existing command. The Hub shows active Sessions and provides an end/outcome form with optional authored result and notes. Drafts are preserved on failed writes. Completed/interrupted status, recorded time, and recoverable entity lifecycle stay distinct.
- **Quick Capture** is reachable from the shell and Hub. It reuses existing Narrative Entry kinds and persists through the application client. Any context link must be selected explicitly; a stand-alone note is valid. A later refresh failure cannot make a successfully persisted entry appear unsaved.
- The activity timeline is derived from actual persisted records/timestamps, with stable ordering. Invalid, missing, or negative Session duration is omitted rather than guessed.
- Workspace panels drill into their typed source records, including Session-to-Quest/Skill/Concept context. Existing Explorer revision, recovery, and history views remain available in the same route.

## Supporting corrections and technical decisions

- Fresh Player startup previously requested an out-of-range search limit; the initial query is now bounded to the native maximum of 200, and Explorer no longer offers an invalid 250-result choice. Tests protect both boundaries.
- A native-window walkthrough exposed an overly narrow Session outcome editor; its responsive card now uses a full-width grid row.
- DTO status types match Rust's `active` Quest and `in_progress` Session literals.
- No new database migration or Rust behavior was necessary. No external UI/component library was added; the work uses the repository's existing React, TypeScript, CSS, Tauri, and testing stack.

## Verification

- `cargo fmt --all -- --check` — passed.
- `cargo check --workspace` — passed.
- `cargo test --workspace` — passed (all workspace tests, including persistence reopen and migration coverage).
- `npm run typecheck` — passed.
- `npm test` — passed: 12 test files, 84 tests.
- `npm run build:vite` — passed.
- `git diff --check` — passed.

A real Tauri desktop window was launched against a disposable `XDG_DATA_HOME`. A fresh SQLite database reached schema version 11 through migrations 1–11. The manual flow created a temporary Player, captured a stand-alone note, started and completed a real Quest Session with an authored outcome, inspected History/Recovery and a Concept record, and returned to the saved Overview workspace. The database contained the expected seven default workspace panels; Player, Quest, completed Session, Concept association and Narrative Entry survived an actual desktop-process restart. The temporary world was separate from user data and is removed after this smoke check.

Frontend behavior tests additionally cover truthful empty/current state, hidden-record awareness, active Effects, explicit Quest and Session actions, Quick Capture and keyboard dismissal, persisted panel drill-down, exact Explorer selection/return, explicit Session-to-Concept navigation without a mutation, timestamp-derived duration/timeline behavior, startup bounds, and valid search page sizes.

## Documentation and limitations

`ARCHITECTURE.md` and `DOMAIN-DESIGN-CODEX.md` now describe the Phase 6 presentation/interaction boundary and update the completed Phase 5–6 roadmap. `PHASE-4-REPORT.md` was inspected: its Phase 4/4.1 branch metadata was already accurate, so it required no change.

The Hub is intentionally limited to the records and context already returned by existing world queries. Session details can follow the Quest, Skill, or Concept IDs already present; this phase adds no inferred or retroactive links. No global command palette, automatic progression, cloud sync, new graph/query framework, additional editor model, or Phase 7 work was introduced. The temporary native smoke world is disposable and not part of the repository.
