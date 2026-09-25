# Life RPG — Phase 1 Checkpoint Handoff

This document is a handoff for a new coding agent taking over the Life RPG project after completion of Phase 1.

## IMPORTANT

The current repository checkpoint is the source of truth.

The agent must inspect the repository and existing files before making changes.
Do NOT recreate the project from the specification.
Do NOT replace the existing architecture merely because another architecture is familiar.
Continue from the checkpoint.

---

## Product goal

Life RPG is a local-first personal application that models real life as an RPG/game world.

It is NOT intended to be a normal productivity app with game-themed styling.

The eventual system should model:

- Player
- Player state and daily history
- Quests and quest hierarchy
- Skill trees and skills
- Effects/buffs/debuffs
- Transactions/history
- Comments
- Narrative entries / briefings / journal
- Rules and progression
- Dynamic presentation
- Persistent UI state
- Workspaces/layouts/themes

The application is primarily a LOCAL DESKTOP APPLICATION / EXECUTABLE.

There is no requirement for a public server, remote clients, LAN access, cloud persistence, or HTTP APIs.

Preferred architecture: local desktop shell + domain/application core + SQLite persistence + frontend UI.

---

# Phase 1 that was actually implemented

## Technology

Current stack:

- Tauri v2
- Rust core
- React 18
- TypeScript
- SQLite via rusqlite with `bundled`
- Vite
- Vitest / Testing Library

IPC is through Tauri commands / in-process bridge.

There is NO Django, FastAPI, REST, GraphQL, WebSocket, or local HTTP backend.

That is intentional and should remain the architecture unless there is a demonstrated technical requirement to change it.

---

## Rust workspace

The root Cargo workspace contains:

- `crates/lr-domain`
- `crates/lr-application`
- `crates/lr-persistence`
- `crates/lr-contracts`
- `src-tauri`

Dependency direction is intentionally separated:

```text
lr-domain
   ↑
lr-application
   ↑
lr-persistence
   ↑
src-tauri
   ↑
React frontend via typed Tauri IPC
```

More precisely, `lr-domain` has no SQLite/Tauri dependency; `lr-application` has ports/use-cases; `lr-persistence` is the SQLite adapter; `src-tauri` is the composition root and command layer.

---

## Domain layer at checkpoint

Phase 1 deliberately does NOT contain Player/Quest/Skill/etc. aggregates yet.

It currently contains foundational value objects:

### `EntityId`

Validated opaque identifier.

### `SchemaVersion`

Supports schema version ordering and downgrade checks.

### `Iso8601Timestamp`

Validated RFC3339/ISO-8601 timestamp with UTC normalization.

Domain errors are defined in `crates/lr-domain/src/error.rs`.

---

## Persistence at checkpoint

SQLite is the authoritative local persistent store.

The database lives under Tauri's app data directory, using:

`life-rpg.sqlite3`

Current SQLite configuration includes:

- WAL journal mode
- foreign keys enabled
- synchronous=NORMAL
- busy timeout
- memory temp storage

Migrations are embedded into the binary and tracked in a `schema_migrations` ledger.

Current migrations:

1. `0001_core_ledger.sql`
2. `0002_health_probe.sql`
3. `0003_type_definition_registry.sql`

Current Phase 1 schema includes:

- `schema_migrations`
- `app_meta`
- `health_probe`
- `type_definitions`

---

## Type definition registry

Phase 1 already implemented a database-driven vocabulary system.

It is represented by:

`type_definitions`

with fields including:

- id
- namespace
- code
- label
- description
- sort_order
- is_active
- is_system
- metadata_json
- timestamps

It has a uniqueness constraint on `(namespace, code)`.

Seed namespaces include:

- quest
- skill_tree
- effect
- transaction
- narrative_entry
- comment_target

Example quest types already seeded:

- main
- side
- daily
- long_term
- challenge

This is intentionally data-driven so adding a conceptual subtype can be an INSERT instead of a code/migration change.

Do not replace this with a huge hard-coded enum unless there is a concrete reason.

---

## Persistence adapter

`crates/lr-persistence/src/sqlite_store.rs` contains `SqliteHealthStore`.

It deliberately supports two states:

- ready file-backed SQLite connection
- unavailable state with an in-memory fallback path used by the desktop shell

This means the UI can still open and report a storage problem instead of failing during startup.

Current design uses a single `Mutex<Connection>` because this is a single-user, single-process local application.

Do not redesign this for multi-process/server concurrency without a requirement.

---

## Application layer at checkpoint

`lr-application` currently contains the health/status use case and ports.

It demonstrates the intended future architecture:

```text
UI intent
  -> Tauri command
  -> application use case
  -> persistence port
  -> SQLite adapter
```

The frontend does not access SQLite directly.

---

## IPC contract

`crates/lr-contracts` defines serializable DTOs for the IPC boundary.

The frontend mirrors the contract in `src/domain/`.

Current contract covers the Phase 1 health/status surface.

Important design rule:

The frontend should not import Rust/domain types directly, and Rust core types should not become the frontend's UI types.

Phase 2 should add DTOs for Player/Quest/Skill/etc. following the existing pattern rather than bypassing the boundary.

---

## Tauri shell

`src-tauri` is the desktop composition root.

Registered commands currently include:

- `get_status`
- `get_world_location`
- `ping`

The app is configured as:

- product name: Life RPG
- identifier: `com.liferpg.desktop`
- default window around 1280x820
- minimum window around 960x640
- bundle targets include deb and AppImage

The CSP was already fixed during Phase 1 to allow Tauri IPC and the Vite development path.

Do not reintroduce a duplicate hand-written CSP in `index.html`.

---

## Frontend at checkpoint

React/TypeScript frontend currently has one real screen:

`StatusScreen`

The current frontend architecture is:

```text
Screen component
    -> use-case hook
    -> CoreClient
    -> Tauri IPC
    -> Rust application use case
```

`src/domain/ipc.ts` is intentionally the single place where the frontend calls `invoke`.

The frontend has:

- app shell
- status screen
- status components
- design tokens
- global styles
- presentation vocabulary
- unit/component tests

---

## Presentation foundation already exists

`src/presentation/spec.ts` already defines a controlled presentation vocabulary.

Current concepts include:

- presentation variant
- density
- emphasis
- surface
- radius
- shadow
- accent
- font size
- style spec

Current variants include examples such as:

- `quest_card_default`
- `quest_card_compact`
- `quest_card_epic`
- `quest_card_minimal`
- `timeline_entry`
- `journal_entry`

The important idea is that the database may eventually hold presentation data, while the frontend interprets a controlled schema.

Do NOT turn this into arbitrary executable JavaScript or unrestricted raw CSS loaded from SQLite.

---

# Phase 1 verification reported by the previous agent

The previous agent reports that it ran:

- `cargo test --workspace`
- `npm run typecheck`
- `npm test`
- `npm run build:vite`
- `cargo tauri build`

It also reports a headless launch of the release executable under Xvfb and verification that a real SQLite file was created, WAL was active, migrations applied, and seeded type definitions existed.

The report is in:

`docs/PHASE-1-REPORT.md`

The architectural rationale is in:

`docs/ARCHITECTURE.md`

These files should be read, but the actual repository/code remains the ultimate source of truth.

The Phase 1 report also explicitly records that pixel-level GUI rendering was NOT confirmed in the headless sandbox and that the health-probe command path itself was covered by tests.

---

# Known Phase 1 limitations

These are deliberate and are NOT bugs to solve by rewriting the foundation:

1. No Player aggregate yet.
2. No Quest aggregate yet.
3. No SkillTree/Skill aggregate yet.
4. No Effect aggregate yet.
5. No real transaction ledger yet beyond the infrastructure vocabulary.
6. No Comments model yet.
7. No NarrativeEntry model yet.
8. No rules engine yet.
9. No persisted UI state yet.
10. No workspace system yet.
11. No router/navigation yet.
12. Only the status screen exists.
13. `health_probe` is infrastructure and should not become game-domain data.

---

# Phase 2 continuation

The next agent should implement the previously defined Phase 2:

## Core persistent domain

Implement:

- Base entity behavior where appropriate
- Player
- TypeDefinition API/repository/service access if needed
- PlayerStateSnapshot
- Transaction
- Comment
- NarrativeEntry
- SkillTree
- Skill
- SkillStateSnapshot
- Quest
- Effect

The exact domain design should be adapted to the existing Rust architecture rather than blindly copied from an earlier Django/Python design.

Use Rust domain structs/value objects in `lr-domain`.

Use application ports/use-cases in `lr-application`.

Use SQLite SQL/repositories/adapters only in `lr-persistence`.

Use DTOs for the frontend boundary in `lr-contracts`.

Keep the frontend unaware of SQL.

---

# Important modeling rules for Phase 2

## Current state vs history

Do not collapse these concepts.

Example:

```text
Player.current_xp / level
        = current state

Transactions
        = historical changes

PlayerStateSnapshot
        = what the player looked like on a date
```

Snapshots are historical records.

Transactions should normally be append-only historical records.

## Types

Quest main/side/daily etc. should remain data-defined types, not separate classes/models.

Same principle for effects, skills, etc.

## JSON

Use JSON for open-ended metadata/extensions where appropriate.

Do NOT represent every domain field as JSON.

## IDs

Keep a consistent identity strategy with the existing `EntityId`/database strategy.

Do not randomly change identifiers halfway through the project.

## Relationships

Use real relational foreign keys wherever the relationship is known and meaningful.

## Comments

Use a generic attachment mechanism if appropriate, but keep referential behavior understandable and testable.

## Transactions

A logical domain operation that changes several records should be atomic.

---

# How to take over

Before editing:

1. Read `docs/PHASE-1-REPORT.md`.
2. Read `docs/ARCHITECTURE.md`.
3. Inspect the Cargo workspace and existing modules.
4. Inspect the migrations already present.
5. Inspect the frontend IPC contract and tests.
6. Verify the project actually builds/tests in the current environment.
7. Identify any genuine Phase 1 blocker before beginning Phase 2.

Do NOT rebuild Phase 1.

Do NOT introduce Django.

Do NOT introduce HTTP APIs.

Do NOT implement the rules engine.

Do NOT build the style sandbox yet.

Do NOT start Phase 3/4 work early.

At the end of the takeover:

- migrations must work from a clean database
- existing tests should remain passing, except for tests intentionally updated because the public contract evolved
- new domain tests must exist
- the project must remain runnable
- produce a concise report of files changed, schema changes, tests run, and known limitations

STOP after Phase 2.
