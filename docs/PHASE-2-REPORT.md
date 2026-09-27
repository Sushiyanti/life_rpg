# Phase 2 report — Persistent core domain

Phase 2 adds a **real, local, persistent RPG world** while preserving the Phase 1
architecture:

```text
React + TypeScript → typed Tauri IPC → application services/ports → SQLite adapter
```

There is no HTTP API, server process, cloud backend, or frontend SQL access.

## Implemented domain

| Area | Implementation |
|---|---|
| Player | Cached level/XP, active state, metadata and timestamps. XP is state; it is not the history itself. |
| Type definitions | Existing database registry remains data-driven. `0004` adds the `skill` seed vocabulary; new types are normal registry rows. |
| Quest | Generic type-defined quest with optional parent/skill references, progress, reward, status and timestamps. |
| Skill Tree / Skill | Player-owned trees and nested skills with state, XP, invested minutes and completion state. |
| Effects | Generic type-defined buffs, debuffs, conditions and modifiers with expiry and intensity. |
| Transactions | One append-only ledger for XP, time, money, or any named resource. |
| Snapshots | Immutable, daily player and skill snapshots, each unique per entity/date. |
| Comments | Generic attachment with a closed target vocabulary and repository-side target-existence verification in the insert transaction. |
| Narrative entry | Player-scoped intentional content, separate from comments. |

## Schema and migration behavior

`0004_phase2_domain.sql` is appended after the untouched Phase 1 migrations. It
creates `players`, `skill_trees`, `skills`, `quests`, `effects`, `transactions`,
`player_state_snapshots`, `skill_state_snapshots`, `comments`, and
`narrative_entries`.

The schema uses structured columns for known state and JSON only for extensible
metadata/state. It includes foreign keys, indexes for common owner/history
queries, status/range checks, `ON DELETE` behavior, composite foreign keys to
`type_definitions(namespace, code)`, and uniqueness for normal daily snapshots.

Comments cannot use a conventional polymorphic foreign key. Instead, the
persistence adapter verifies the target table record exists **inside the same
SQLite transaction** that inserts the comment; a `CHECK` restricts the target
kind to supported entity families.

## Atomic operations

The SQLite adapter performs these logical changes in one transaction:

1. XP award: update cached Player state + append ledger row.
2. Quest completion with reward: complete Quest + update Player + append reward.
3. Skill time investment: update invested minutes + append time ledger row.
4. Comment creation: verify polymorphic target + insert Comment.

## Application, IPC, and frontend

`WorldStore` is the application's persistence port and `WorldService` provides
use cases. SQL remains in `lr-persistence::world_store`; Tauri commands only
adapt requests to application services and DTOs. `lr-contracts::world` owns the
camelCase IPC payloads, and `src/domain/world.ts` mirrors them in TypeScript.

The frontend intentionally adds only `WorldPanel`, a compact developer
verification panel for creating a local player and awarding XP. It is not a
Phase 3 rules engine or a full RPG UI.

## Deliberate Phase 2 boundaries

- No rules engine, triggers, conditions, or action framework.
- No dynamic presentation/workspace persistence or style sandbox.
- No cloud sync, accounts, multiplayer, HTTP APIs, or server process.
- No broad end-user RPG UI beyond the small verification panel.

Phase 3 should begin with a rules model over this persistent core, retaining the
current rule that all multi-row world changes are atomic and that history,
cached state, and snapshots stay separate.
