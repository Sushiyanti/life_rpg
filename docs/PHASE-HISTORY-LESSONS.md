# Life RPG project-history lessons

This audit records engineering lessons from the project’s actual phase commits and reports. It is intentionally focused on reusable application foundations rather than future product scope.

## Phase 1 — layered local-first foundation

**What worked:** the repository established a clear React/TypeScript → typed Tauri IPC → Rust application/domain → SQLite persistence boundary. The health/status path and migration ledger made storage availability observable without putting SQL in the UI.

**What caused later problems:** the initial architecture was strong but the frontend boundary began accumulating compact convenience methods as later phases added domains. The lesson is not to remove the boundary, but to keep each new command typed, named once, and covered at the IPC layer.

**Lesson:** preserve layered ownership. UI code orchestrates presentation; application services own use cases; persistence owns storage and integrity.

## Phase 2 — persistent world domain

**What worked:** Player, Quest, Skill, Effect, transaction, narrative, and progression records became real persisted domain entities. The typed command adapters made those entities available without direct storage access from React.

**What caused later problems:** broad world overviews became convenient for screens, but they are not an appropriate resolver for every contextual interaction. A detail surface must request the smallest entity-specific read it needs.

**Lesson:** use overview queries for overview screens and single-record queries for entity surfaces. Never substitute a preloaded array for authoritative identity.

## Phase 2.1 — integrity, history, and explicit progression

**What worked:** snapshots, transaction semantics, stat definitions, persistence checks, and explicit progression mutation made changes auditable and recoverable.

**What caused later problems:** a UI can look editable while still lacking a meaningful domain mutation. The surface editor must expose only commands that really exist and must keep save/cancel semantics explicit.

**Lesson:** do not invent editable fields in the UI. Every save path must call a typed domain command and preserve history/integrity guarantees.

## Phase 3 — semantics and relationships

**What worked:** Concept relationships, progress, rules, search, lifecycle, revisions, attachments, and associations were implemented as first-class domain semantics. The database triggers enforce same-player ownership for Concept relationships and Effect targets.

**What caused later problems:** the frontend initially used the relationship API as a concept-only query and then scanned all Concepts to discover Quest associations. That created unnecessary N+1 work and obscured the real relationship contract.

**Lesson:** expose all existing backend filters in the typed client. Relationship-driven navigation must query by both entity kind and entity ID, then resolve only the returned related IDs.

## Phase 3.5 — Concepts and search

**What worked:** Concept records, Concept relationships, progress tracks, and search remained first-class instead of flattening everything into a generic record.

**Lesson:** entity-specific renderers are appropriate; generic infrastructure should provide the lifecycle, while each renderer owns the meaning of its fields and relationships.

## Phase 3.6 — world semantics, lifecycle, and recovery

**What worked:** lifecycle state, revisions, recovery, presentation preferences, content attachment, and relationship associations were separated from domain meaning. Visibility was explicitly treated as presentation state.

**Lesson:** a surface may show lifecycle and visibility metadata, but it must not conflate hidden, archived, trashed, inactive, and deleted. Missing records need an honest explanation and an exit path.

## Phase 4 UI and 4.1 fixes

**What worked:** the dynamic world UI used typed clients, player-scoped data, contextual actions, and targeted integrity fixes. The UI remained grounded in persisted data.

**What caused later problems:** route-driven rendering and contextual interaction could easily remount or replace a workspace. The later workspace work corrected that direction.

**Lesson:** entity inspection should be contextual when it is small and reversible. Do not replace the page tree just to show one record.

## Phase 5 — persistent workspaces

**What worked:** workspace records and panels made page context durable: selected workspace, panel layout, filters, density, and display preferences belong to the player’s saved context.

**Lesson:** modal surfaces must mount above the workspace, not recreate it. Body locking belongs to the modal; surface-body scrolling belongs to the surface; underlying workspace state remains mounted.

## 6.0 / 6.0a lessons

The first 6.0 implementation proved that a contextual drawer/dialog could open without route replacement. 6.0a corrected real identity and relationship mistakes, added human-readable copy, and demonstrated Player and Concept mutation. However, it still kept the provider, resolver, registry-like branching, renderers, editors, formatters, and styles together. It also refreshed broad world data and scanned every Concept for Quest associations. Its focus handling moved focus to a heading but did not contain keyboard navigation inside the modal.

6.0b corrects those architectural shortcuts. Surface infrastructure now owns stack, overlay, focus, Escape, body locking, and common toolbar behavior. A registry maps kinds to entity-specific loaders, renderers, editors, title functions, and copy formatters. Quest resolution uses filtered persisted association queries; Concept resolution loads only its own relationship targets; Effect resolution uses a player-scoped effect query. Stale records render an explanatory state with Close and, when nested, Back. The history of the project therefore leads to a concrete rule: keep domain ownership in Rust, keep presentation registration in React, and make future entity surfaces additive rather than invasive.

## 6.0c finalization lesson

An extensible registry can still hide an accidentally entity-specific loader contract. The generic result must carry only an entity plus optional descriptor-owned context, and an unsupported kind must fail explicitly rather than fall back to an unrelated descriptor. An extensibility test is only meaningful when it exercises the actual provider, loader, renderer, and stack behavior—not merely registry insertion.

## Rules for future agents

1. Read the relevant phase diff and report before changing a cross-cutting foundation.
2. Preserve the typed IPC and persistence boundary; do not put SQL or relationship semantics in React.
3. Treat IDs and persisted relationships as authoritative.
4. Prefer the smallest entity-specific query over broad overview refreshes.
5. Keep generic infrastructure free of entity-specific conditionals; register new kinds separately.
6. Make editability honest: expose only real commands and provide Save/Cancel lifecycle.
7. Keep workspaces mounted when a contextual surface opens.
8. Test the contract, not merely the existence of a button.
9. Document environment limitations instead of claiming native or visual validation that did not occur.
