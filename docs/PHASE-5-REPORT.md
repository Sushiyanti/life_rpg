# Phase 5 — Persistent Player Workspaces

**Milestone:** persistent workspace foundations (schema 9)  
**Follow-up:** expanded safely on `phase-5.1-workspace-capabilities` (schema 10)  
**Base:** `phase-4.1-ui-fixes`  
**Phase 5 implementation commit:** `8fba38f` — `Add persistent player workspaces and builder`

## Delivered

Phase 5 moved workspace and panel definitions from browser storage into SQLite. A workspace is presentation configuration owned by one Player; it is not a canonical world entity. The workspace and panel tables, `SemanticsStore` port and service, serializable DTOs, typed `CoreClient`, and thin Tauri commands form the persistence/API path. At creation, the app seeds declarative templates such as Overview, Focus, Learning, Health, and Review. The selected/default workspace is per Player world.

The initial Phase 5 panel vocabulary was deliberately bounded. Panels used built-in source types, a constrained card/row presentation, a small status filter, order, pin/collapse state, and a bounded item count. Phase 5.1 extends this base through a forward-only migration without rewriting migration history or losing version 9 panel IDs/configuration.

## Boundary

Workspace/panel create, rename, selection, configuration, hiding, and removal only affect presentation. They do not change a Quest, Skill, Concept, Player, Session, Effect, Transaction, lifecycle state, or history. Per-entity contextual visibility remains distinct from panel visibility and workspace membership. Player ownership is enforced across the application/storage boundary; the final workspace is retained.

## Compatibility

The version 9 schema is retained as migration history. Migration 10 upgrades it forward, adding the more complete data-defined presentation model while preserving existing values. The application includes a one-time compatibility import of the old Phase 4 dashboard layout when a Player has no persisted workspace; the legacy browser key is removed after a successful import. See [Phase 5.1 report](PHASE-5.1-REPORT.md) for exact schema, options, tests, import/export, and remaining limitations.
