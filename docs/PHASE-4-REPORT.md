# Phase 4 — Dynamic World UI, Workspace & Explorer

**Branch:** `phase-4-ui`  
**Starting point:** `phase-3.6` at `cc96792`  
**Scope:** First-generation local desktop UI over the Phase 3.6 world model; no Phase 5 work.

## Summary

Phase 4 replaces the title/status-only presentation with an application shell and working typed-IPC-backed world screens. It does not redesign domain semantics. Player and Skill levels remain manually authored, Concept progress remains manual unless explicitly delegated, Session periods represent activity actually started/finished, contextual visibility remains independent of lifecycle, and restore operations remain auditable.

The UI is built for the existing Rust + Tauri + React/TypeScript + SQLite desktop application. No network service, remote database, daemon, scheduler, or frontend database access was added.

## What was implemented

### Application shell and context

- Persistent sidebar navigation for Overview, Player, Quests, Skills, Skill Trees, Concepts, Effects, Journal, World Explorer, History & Recovery, Rules, and System Health.
- Active Player/world identity and a quick-create entry point in the shell.
- Last route and selected Player are stored as local UI state. The empty-world state explains that activity is only recorded when the Player records it and offers first-Player creation.
- Shared layout/design tokens provide the dark, information-dense, game-like desktop shell with responsive layout rules.

### Structured dashboard workspace

- Allow-listed panel definitions cover source, title, name filter, item limit, ordering, visibility, pinned priority, collapse state, density, and card/row layout.
- Panels can be added/revealed, hidden, reordered, pinned, collapsed, filtered, and adjusted through a usable customization panel.
- Dashboard layout is stored as structured browser-local UI state; it is not submitted as a fake world entity. Per-record visibility continues to use the Phase 3.6 presentation-preference service keyed by Player, entity, and context.
- The dashboard surfaces Player-authored level, data-defined stats, active Quest/Skill/Concept/Effect/Session/Journal panels, and hidden-item awareness. Panel contents are filtered through their contextual preferences.

### World pages and activity

- **Player:** identity, manual level and labels, data-defined stats, active Effects, explicit Player snapshots, and local progression history display.
- **Quests:** creation/status, optional Stages and Branches, context visibility controls, attached Notes/Guidance, and real Session records. Sessions can start at the Player's explicit action time and finish with a recorded status; no missing-day activity is created.
- **Skills / Skill Trees:** tree organization, manually authored Skill levels and labels, recorded practice time, explicit Skill snapshots, and contextual visibility for both trees and Skills.
- **Concepts:** typed Concept creation, independent progress tracks, typed directed Concept-to-Concept relationships, and contextual visibility. Concepts remain distinct from Quests, Skills, and Narrative entries.
- **Effects:** current/inactive status, recorded timestamps, contextual hide/show, and explicit deactivation.
- **Journal:** reusable data-defined Narrative entries. The editor exposes the Phase 3.6 seeded human-facing entry kinds: Note, Briefing, Story, Reflection, Reminder, Journal, Guide, Guidance, Lore, Instruction, Reading, To Do, Reference, and Summary.
- **Shared detail/Explorer surfaces:** typed result kind and metadata, Concept relationships and explicit cross-entity associations, content attachments using seeded roles, comments for supported targets, lifecycle, and revisions where the backend supports them. Related Concepts and associated world rows are navigable from the inspector.
- **World Explorer / History:** text search with record kind, type, Concept relationship, active/inactive, time range, sorting, page size/offset, and separate include-hidden/archive/trash filters. Result kinds remain explicit. History exposes supported before-image snapshots and explicit restore; restoration is described as a new history-producing change. Unsupported lifecycle kinds are shown as “Not tracked,” rather than being mislabeled Active.

## Presentation and state boundaries

- Canonical world mutations go through `CoreClient` → Tauri commands → application services → persistence. `src/domain/ipc.ts` remains the only frontend module that calls Tauri `invoke`.
- Contextual item visibility is persisted by the existing Phase 3.6 presentation-preference API. The “reveal hidden” affordance is a temporary view filter; per-record Show/Hide actions persist the preference.
- Dashboard composition and route are presentation-only local state. Neither is represented as a Player or other world entity.
- Existing lifecycle filters stay independent of visibility. Hidden records remain searchable when explicitly included; archived/trashed inclusion is separately controlled.
- No XP-to-level conversion, automatic stat calculation, timeline interpolation, scheduled behavior, or destructive purge was introduced.

## Backend/application additions

Only missing paths needed by the UI were added, using existing services and stores:

- Tauri Concept adapters for list/create, relationship types, and typed Concept relationship creation; the existing `ConceptService` enforces same-Player ownership and registered relationship types.
- Typed CoreClient command declarations for those Concept operations.
- A typed `WorldService` comment-list query and Tauri adapter for supported detail pages.
- Presentation update calls carry optional variant and density values already modeled by Phase 3.6.
- Tauri commands are registered in the existing in-process handler. No migration or schema change was needed; the existing maximum schema version remains 8.

## Tests and verification

- `cargo fmt --all -- --check` — passed.
- `cargo test --workspace` — passed, **79 Rust tests** across the Tauri shell, application, contracts, domain, and persistence crates.
- `npm run typecheck` — passed.
- `npm test` — passed, **39 frontend tests across 6 files**, including 8 Phase 4 behavior tests for navigation, hidden-item visibility, dashboard state persistence, search filtering, manual levels, Stage/Branch/Session presentation, Concept relationship creation, and history restoration.
- `npm run build:vite` — passed; production bundle built successfully.
- Migration verification — the native app launched against its normal app-data path and created `~/.local/share/com.liferpg.desktop/life-rpg.sqlite3`. Read-only SQLite verification confirmed schema version **8**, migration entries 1–8, and the Phase 3.6 tables. Workspace migration tests also passed for fresh creation, idempotence, partial resume, Phase 1 upgrades, Phase 2.1-to-later upgrades, and preservation of historical data.
- Startup/UI smoke checks — the normal `npm run dev` Tauri command compiled and started the native executable under Xvfb; it created and migrated its local database. Browser preview confirmed the application shell and navigation landmarks. In a browser without Tauri IPC the app correctly shows an offline/connection warning. Frontend behavior tests exercise navigation and representative major routes.
- `git diff --check` — passed before final review.

## Known limitations and deferred work

- This is a first-generation workspace, not an arbitrary visual-programming system. Panel types are a closed allow-list; filter expressions are plain text, not executable code.
- Route selection and dashboard layout are browser-local UI state; entity contextual preferences are saved in the world database. Cross-device UI synchronization is not implemented.
- The Explorer reuses a common inspector rather than implementing a bespoke full-detail workflow for every record kind. Some kinds are intentionally read-only or show lifecycle as untracked when Phase 3.6 does not support those operations.
- Concept-to-Concept relationships can be listed and created in this UI; relationship deactivation/editing is not surfaced.
- Narrative editing after creation, rich text/Markdown, a timeline, arbitrary relationship-graph traversal, a complete Effect-creation workflow, permanent deletion, and additional domain/action types are deferred.
- The browser-only Vite preview has no Rust/Tauri IPC runtime and is not a web deployment target. The native Tauri app is the intended runtime.
- The headless native startup smoke test emitted non-fatal accessibility/DRI warnings from Xvfb and the process was later stopped by the sandbox; the executable reached startup and created the schema-8 database.

## Files changed

- Shell/state: `src/App.tsx`, `src/app/AppShell.tsx`, `src/app/AppShell.css`.
- World UI: `src/features/world/WorldWorkspace.tsx`, `WorldWorkspace.css`, `WorldExplorer.tsx`, `WorldExplorer.css`, `PlayerCharacter.tsx`, `EffectsScreen.tsx`.
- Typed frontend boundary and tests: `src/domain/ipc.ts`, `src/domain/world.ts`, `src/test/Phase4Workspace.test.tsx`, `src/test/ipc.test.ts`.
- Minimal application/Tauri paths: `crates/lr-application/src/services/world.rs`, `crates/lr-application/src/services/semantics.rs`, `crates/lr-contracts/src/world.rs`, `src-tauri/src/state.rs`, `src-tauri/src/commands/concepts.rs`, `src-tauri/src/commands/world.rs`, `src-tauri/src/commands/semantics.rs`, `src-tauri/src/lib.rs`.
- Verification fixture update: `crates/lr-persistence/src/semantics_store.rs`.
- Documentation: `README.md`, `docs/ARCHITECTURE.md`, `docs/DOMAIN-DESIGN-CODEX.md`.

**Phase boundary:** Phase 4 only. Phase 5 was not started.
