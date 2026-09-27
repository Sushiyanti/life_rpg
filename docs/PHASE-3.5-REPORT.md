# Phase 3.5 report — Domain Concept refinement

## Outcome

Phase 3.5 adds a typed Concept domain and the temporal/search foundations for later UI work, without replacing Quests, Skills, Narratives, Effects, Transactions, or the Phase 3 rule engine. Work was isolated on branch `phase-3.5`. **Phase 4 has not started.**

The system remains local-first and layered:

```text
React + TypeScript mirrors
        ↓ (existing typed IPC; no new Concept UI/commands in this phase)
Tauri composition root
        ↓
lr-application: ConceptService + SearchQuery + bounded RuleEngine
        ↓ focused ConceptStore / SearchStore / WorldStore ports
lr-persistence: SQLite migration 0007, atomic writes, compact FTS5 projection
```

## Architecture decisions

- **Concepts are subjects, not universal records.** A Concept has a Player owner, data-defined type, name/description, active state, JSON metadata, and created/updated timestamps. Quest, Skill Tree, Skill, Narrative Entry, Comment, Transaction, and Effect remain separate named types.
- **Type vocabularies are data-defined.** Concept type codes use the existing type registry under the `concept` namespace. Relationship codes and Progress Track definitions have dedicated tables. No new Rust enum/migration is needed to add a normal Concept type or relationship code.
- **Relationships are narrow and explicit.** A typed, directed Concept-to-Concept relationship supports codes such as `depends_on` and `part_of`. Same-Player ownership is checked in application and SQLite; self-relations and missing/cross-world targets fail closed. Explicit links to other named entity kinds are not a generic polymorphic graph.
- **Progress is opt-in and has explicit meaning.** Concepts can have multiple independent tracks (`numeric`, `percentage`, `experience`, `level`, `mastery`). Finite values, configured bounds, percentage limits, whole-number experience/level requirements, active definitions, and optional positive level are validated. There is no universal Concept XP field or inferred conversion.
- **Effect targeting stays closed.** Effects target a Player by default or a same-world Concept through a typed target representation, SQLite guards, DTO fields, and Concept-aware search. Effects do not gain generic arbitrary-entity targets.
- **Progress mutations cannot bypass rule atomicity.** The standalone progress mutation port was removed. `ConceptService::set_progress` plans the source event and all derived work through the existing bounded `WorldStore::apply_rule_chain` transaction. A bad derived action rolls back the root progress value/history too.
- **No generic storage framework or task infrastructure was added.** There are no daemons, schedulers, event bus, universal entity/property abstractions, or Phase 4 UI/presentation records.

## Domain and persistence changes

Migration **`0007_phase35_concepts.sql`** advances the schema from version 6 to **version 7**. It adds:

- `concepts`, `concept_relationship_types`, and `concept_relationships` with owner guards and indexes;
- `progress_track_definitions`, current `concept_progress_tracks`, and append-only `concept_progress_history`;
- immutable, unique-per-date `concept_state_snapshots`;
- explicit `concept_entity_links` for the named supported world entities, with same-Player/target-existence checks and cleanup triggers;
- Player-or-Concept Effect target columns/guards;
- a synchronized SQLite FTS5 table and source triggers/backfill for meaningful searchable entities;
- a widened persisted closed Rule event allow-list for `concept_progress_changed`, preserving Phase 3 Rule/audit rows;
- nullable Transaction `captured_at` with temporal ordering guard. Historical transactions retain `NULL`; migration does not guess capture time.

Concept progress history and snapshots are protected against update/delete. Foreign keys restrict deleting a Concept that still anchors immutable history, snapshots, or an Effect; mutable explicit links and non-historical relationships may clean up with their parent as defined by their schema. No application-level Concept delete workflow was added.

## Rule Engine compatibility

The new event is `concept_progress_changed`, carrying Player, Concept, Concept type, track code, prior/current value, and optional level. Its typed condition subjects cover prior/current value, level, Concept type, and track code. The bounded `set_concept_progress` action validates the destination Concept owner and track definition. Follow-on progress events use the existing deterministic priority/FIFO dispatch and depth/action/evaluation guards. Root changes, derived operations, XP ledgers, and rule audit remain one fail-atomic chain.

Only small compatibility behavior was added; Effect activation and Skill unlock rules, automatic level formulas, and arbitrary state predicates remain deferred.

## Temporal model

- **`created_at`** — creation of a mutable record.
- **`updated_at`** — latest mutation of a mutable record.
- **`occurred_at`** — when an event/change happened in the represented world.
- **`captured_at`** — when this installation observed/recorded it.

New Concept progress history persists both event and observation timestamps and rejects `occurred_at > captured_at`. Transactions expose optional `capturedAt` through the Rust DTO and TypeScript mirror; the domain and SQLite reject a capture moment before occurrence. Legacy transaction capture remains explicitly unknown. Existing Player/Skill snapshots retain their immutable `created_at` capture column; Concept snapshots use `captured_at`.

Several events may occur on one calendar date. A daily snapshot is an immutable state observation with one row per Concept/date, not a summary of that date. Duplicate daily capture fails. Missing dates remain gaps and are not converted to zero activity or synthetic state.

## Search/query approach

`lr-application::SearchQuery` / `SearchHit` provide typed filters and result references for Player, Concept, Quest, Skill Tree, Skill, Effect, Transaction, Comment, Narrative Entry, and Concept progress history. The query supports bounded sanitized FTS text, entity kind, Player, Concept association, type code, status, active state, timestamp range, sorting, limit, and offset. The Effect's active state is evaluated against the query instant. Concept relationships, explicit links, and Effect targets contribute to Concept-related filtering.

SQLite FTS5 stores only identity/kind/owner/type/status/active/time/name/body projection; it does not duplicate full aggregate state, metadata JSON, or Rule payloads. Triggers maintain it and migration 0007 backfills existing data. FTS user operators are quoted/sanitized; empty punctuation-only searches fail closed; limits are bounded.

A `ConceptService::detail` application read model composes a Concept with current tracks, relationships, explicit entity-link references, progress history, and snapshots. This supports future specialized screens without persisting “Page” entities. Tags were evaluated and deferred: no concrete reuse/filtering need was specified, and the current named relationship/type/query model suffices without prematurely creating another global vocabulary. No polished search/Concept UI was built.

## Documentation and contracts

- Rewrote `docs/DOMAIN-DESIGN-CODEX.md` as the canonical guide to Concepts, distinct entity types, data-defined vocabularies, directed relationships, progress semantics, timestamps, immutable daily snapshots/gaps, search, specialized pages, and explicit boundaries.
- Extended `docs/ARCHITECTURE.md` with Phase 3.5 layering, atomic progress-rule behavior, temporal semantics, FTS/query boundaries, and a Phase 3.5 roadmap row.
- Updated README status/layout and added this completion report.
- Added camelCase Effect target and Transaction capture fields to Rust DTOs and the TypeScript world/rule type mirrors. No new Concept IPC commands or React feature were introduced; application use cases/query are ready for a later integration surface.

## Verification

All checks ran on the `phase-3.5` branch:

- `cargo fmt --all -- --check` — passed.
- `cargo test --workspace` — **71 tests passed, 0 failed** (desktop 3, application 13, contracts 7, domain 11, persistence 37; doc-tests passed).
- Migration tests — clean schema 1→7, idempotence, continuation, version-5 Phase 2.1 through Phase 3 into 3.5, and preservation of Player XP/level, dynamic Stats, XP ledger, snapshots, existing Rule rows, and append-only Rule audit. Legacy Transaction capture remains null.
- Concept/persistence tests — typed creation and active state, relationship types/active state/owner/self/missing-reference guards, multiple track semantics and bounds, multiple same-day progress events with distinct occurrence times, unique immutable daily snapshot, Rule-derived Concept progress, fail-atomic rollback, Concept Effect target/search, typed search filters/sort/pagination, and file-backed persistence after reopen.
- `npm run typecheck` — passed.
- `npm test -- --run` — **32 tests passed** across 5 files.
- `npm run build:vite` — passed; production bundle generated.
- `git diff --check` — passed.

## Known limitations and intentional deferrals

- Concept CRUD, progress, and search are application-level capabilities but are not yet exposed as new Tauri commands or UI; Phase 4 may design the appropriate minimal IPC and presentation use.
- Search is a pragmatic local FTS5 query foundation, not a full search product. Ranking tuning, tags, arbitrary JSON search, and timeline UI are deferred.
- Concept detail currently returns related entity references rather than hydrating every Quest/Skill/Narrative/Transaction/Effect into a large aggregate payload. The query can be composed by consumers as needed.
- Concepts with immutable history/snapshots or Concept-targeted Effects cannot be physically deleted; no delete use case is defined. Active/inactive state is available for normal retirement.
- No graph inference, transitive dependency evaluation, skill unlock behavior, Concept-triggered Effect activation, XP-to-level conversion, background processing, sync, or multiplayer.

## Recommended Phase 4 starting point — not started here

Design the minimal typed IPC surface needed to expose `ConceptService` and `SearchQuery`, then build a specialized Concept detail and Search/Explorer view using those domain/query structures. Keep presentation/workspace state separate from Player/world truth; first decide the player-facing flow and only then add UI-owned persistence. Reuse the current typed target/track/query semantics rather than introducing a generic entity framework. **This is a handoff recommendation only; Phase 4 changes were not made.**
