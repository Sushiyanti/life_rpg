# Phase 3.6 — World Semantics Report

**Branch:** `phase-3.6`  
**Scope:** Complete the domain semantics needed before Phase 4 UI design.  
**Phase 4 UI:** Not started.

## Summary

Phase 3.6 makes player authority, real activity, recoverability, and presentation context explicit in the world model. It extends existing typed entities rather than turning all records into Concepts or introducing a generic object framework.

## Domain decisions and implementation

- **Manual progression authority:** Player and Skill levels (with optional descriptive labels) are explicitly set. XP remains a separate ledger; no XP-to-level conversion is performed. Concept Progress Tracks declare `manual` or `rule_controlled` control. A dedicated command requires the Player to explicitly delegate a track before automated rule writes may affect it; delegation can be revoked.
- **Suggestions, not silent mutation:** Rule/application proposals for Concept progression are persisted as pending suggestions. They have no effect until accepted. Acceptance applies the track value, progress history, suggestion resolution and related rule events in the same atomic chain. Rejection leaves progress unchanged.
- **Quest hierarchy:** A Quest may have ordered Stages; a Stage may have alternative Branches. Same-Player/parent ownership is checked in application logic and persistence constraints. Simple Quests do not need these child records.
- **Real activity periods:** Quest Sessions have actual start/end timestamps, status, optional Quest/Stage/Branch/Skill/Concept context, and notes/results. The model does not infer activity from a calendar or fabricate records for gaps.
- **Content and Concept associations:** Narrative content can be attached to supported entities with a typed target and role. Concepts can be associated with explicitly supported record kinds; records retain their original identity and semantics.
- **Recoverability:** Migration `0008_phase36_world_semantics` adds lifecycle and revision structures. Before-image triggers record edits for supported aggregates. `active`, `archived`, and `trashed` are reversible states; soft deletion preserves data. Restoring a revision is itself recorded as a new mutation.
- **Contextual visibility:** Visibility, ordering, pinning and collapse preferences are scoped by Player, entity, and UI context. They are presentation preferences, not domain state. Search filters hidden/archived/trashed records only when the corresponding context/lifecycle option is supplied.
- **Search:** FTS5 keeps typed identity and text separate from unindexed filtering metadata. Search now applies contextual visibility and lifecycle filters while retaining safe query sanitation and bounded limits.
- **IPC boundary:** New serializable Phase 3.6 DTOs and Tauri adapters expose stage/branch/session, content/association, lifecycle/revision, presentation preference, and suggestion use cases. Player and Skill progression setters are exposed separately. No UI components or screens were added.

## Persistence and migration

Schema version advances to **8** via a new append-only migration. The migration adds progression labels/control metadata, Quest Stage/Branch/Session tables, attachment and association records, lifecycle and revision tables, presentation preferences, progress suggestions, FTS metadata/triggers, and revision capture triggers. Existing migration bodies remain unchanged. Fresh databases and migration registration are covered by persistence tests.

## Verification

- `cargo test --workspace` — **79 tests passed**, no failures.
- `cargo check --workspace` — **passed**, including the new IPC DTOs and Tauri command registrations.
- `npm run typecheck` — **passed**.
- `npm test` — **31 tests passed** across 5 files.
- `npm run build:vite` — **passed**.
- `git diff --check` — **passed**.

## Explicit non-goals

No XP-to-level formula, synthesized daily activity, destructive delete, implicit overwrite of authored state, generic universal entity framework, inference engine, or Phase 4 UI/workspace implementation is part of this milestone.
