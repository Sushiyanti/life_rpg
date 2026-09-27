# Phase 2.1 report — Domain integrity and state history

Phase 2.1 hardens the Phase 2 persistent world without beginning the rules engine or expanding the UI. It stays inside the local-first architecture and adds migration `0005_phase21_integrity.sql` after all prior migrations.

## State and history

Player snapshot capture is an application use case. A single SQLite transaction reads the current Player, dynamic stat definitions/values, and effects active at capture time, then inserts a daily snapshot. Its canonical JSON is an explicit, versioned payload (`schemaVersion: 1`, stats, active effects); level, XP, date, and capture time remain structured columns. Skill snapshots are similarly captured by an application use case and preserve level, XP, status, invested minutes, and versioned tree/parent context. Both entity/date pairs are unique; duplicate normal daily captures fail, and history can be retrieved in date order.

Dynamic stats use structured `StatDefinition` and `PlayerStat` records. Identity, value, bounds, activity, and useful description/unit fields remain queryable columns, while narrowly scoped JSON metadata stays extensible. Adding a stat definition or a Player's value is a data operation, not a schema change.

## Integrity and lifecycle

XP has a zero floor in the domain and database. Transactions preserve both the **requested event amount** and **actual applied state delta**. For example, a -20 penalty when current XP is 5 records `amount = -20`, `applied_amount = -5`, and a resulting balance of zero. XP totals use the applied delta. Migration 5 repairs legacy negative cached balances with a correction event before installing SQLite XP-floor triggers.

Atomic XP awards, rewarded Quest completion, and Skill-time investment validate event, owner, and state consistency in one SQLite transaction. Standalone XP ledger writes are rejected. Failed or mismatched operations leave no partial state/history. Transactions and daily snapshots are database-enforced append-only/immutable, and snapshot payloads must be valid JSON.

Effect lifecycle is explicit: scheduled, active, expired, or manually deactivated. Additive `deactivated_at` preserves a non-destructive manual-off marker; active-at-time reads exclude future, expired, and manually ended effects. Quest parent/linked-Skill references must stay in one Player world, while a Skill's parent must stay in its tree. Domain setters reject direct self-parenting; recursive SQLite triggers reject indirect cycles and cross-owner updates/inserts because ordinary foreign keys cannot express these semantics.

## Layers and migration

The typed application/service/IPC path now covers Player and Skill snapshot capture/retrieval, dynamic stat definition/value operations, manual Effect deactivation, and XP's applied amount. SQLite remains inside `lr-persistence`, and Tauri handlers remain thin adapters. Migration 5 is additive: no earlier migration is edited. It adds stat tables, XP applied-delta and Effect deactivation columns, and triggers for XP/stat constraints, ownership, cycles, and historical immutability. Migration tests cover clean zero-to-current setup, Phase 1 forward upgrade, negative-XP repair from the Phase 2 schema, and idempotence.

## Design decisions and limits

Player snapshot JSON is an explicitly shaped world-state contract, not a dump of Rust aggregates, UI state, or arbitrary Player metadata. Capture uses UTC clock time; inactive stat definitions are omitted, and only then-active effects are included. Quest status remains the Phase 2 vocabulary (`open`, `active`, `completed`, `abandoned`); pause/failure policy is deferred. Comments and Narrative Entries stay separate authored concepts; no broad edit/delete workflow is added. Phase 3 rules, UI-state/presentation/workspace persistence, complete RPG screens, cloud sync, accounts, authentication, an HTTP API, and multiplayer remain intentionally unimplemented.

## Verification results

| Check | Result |
|---|---:|
| `cargo test --workspace` | **50 passed**, 0 failed (including doc tests) |
| `npm run typecheck` | Passed |
| `npm test -- --run` | **30 passed**, 0 failed |
| `npm run build:vite` | Passed |
| `git diff --check` | Passed |
| Migration/reopen coverage | Zero-to-v5, Phase 1 upgrade, v4 XP repair, unique daily snapshots, and database reopen verified |

The permanent product vocabulary and extension rules are in [`DOMAIN-DESIGN-CODEX.md`](DOMAIN-DESIGN-CODEX.md).
