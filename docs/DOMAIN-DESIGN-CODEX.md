# Life RPG — Domain Design Codex

This is the canonical conceptual reference for the product's world model. Read it with [`ARCHITECTURE.md`](ARCHITECTURE.md) before changing domain behavior. The current implementation includes Phase 3's bounded declarative rules, Phase 3.5's domain refinement, Phase 3.6's world semantics, and Phase 4's first-generation workspace and Explorer UI. Phase 4 adds presentation, not new world meaning; the [Phase 4 report](PHASE-4-REPORT.md) records its boundaries and limitations.

## Product philosophy

Life RPG is a **local-first application that represents a player's real life as a persistent game-like world**. The backend/domain establishes what something **is**, what state it has, what happened to it, and when that was observed. The frontend decides how that world **looks and feels**. Presentation must not redefine or corrupt domain meaning.

The database remains a typed relational world, not a universal object/property framework. Shared abstractions are used only where they express real shared semantics.

## Core vocabulary

| Entity | Represents / is not | Structured semantics |
|---|---|---|
| **Player** | A character representing the user in this world; not an account or the application. | Owns the local world and its Player-level progression. Level and optional descriptive labels are Player-authored; XP is a separate ledger and never computes or overwrites level. |
| **Concept** | A meaningful subject in the Player's world, such as Python, Health, a project, a place, a person, or a goal. It is **not** a Quest, Skill, NarrativeEntry, Comment, or generic container. | Player-owned; data-defined Concept type; name/description/active state/metadata and created/updated timestamps. Concepts can have their own relationships and zero or more typed Progress Tracks. |
| **Concept type** | A useful classification of a Concept; not a closed Rust enum. | Rows in the existing `type_definitions` registry under the `concept` namespace. Adding a type is normally data/configuration, not a code change or migration. Seed examples include subject, project, life_area, person, and place. |
| **Concept relationship** | A typed, directed relation between two Concepts, not a general graph engine. | Source, target, data-defined relationship code, active state, metadata, created/updated timestamps. Both endpoints must belong to the same Player; self-relations are rejected. Seed relationship codes include `related_to`, `parent_of`, `child_of`, `depends_on`, `part_of`, `derived_from`, and `prerequisite_of`. |
| **Quest** | A goal/objective in the world; not a Concept. | Existing Quest type/status/progress/lifecycle remains structured. A Quest may be explicitly linked to one or more Concepts. Parent/child and linked Skill ownership/cycle invariants remain intact. Optional ordered Stages and alternative Branches provide explicit hierarchy without requiring every Quest to use them. |
| **Quest Stage / Branch** | Optional breakdown and alternatives within a Quest; not independent Quests or Concepts. | Stages belong to one Quest. Branches belong to one Stage and its Quest. Both are same-Player constrained, ordered records with their own bounded status vocabularies. |
| **Quest Session** | A real, timestamped period of activity; not a daily summary or inferred streak. | Start/end timestamps, optional Quest/Stage/Branch/Skill/Concept context, status and authored notes/result. Multiple sessions may occur on a date. Missing periods remain missing; no synthetic activity or snapshots are generated. |
| **Skill Tree / Skill** | A Player-owned grouping and a distinct ability/practice area; neither is a Concept. | Existing types, lifecycle, XP and minutes remain structured. Player-authored levels and optional labels can be changed explicitly in either direction, independent of XP. Both can be linked to Concepts. |
| **Stat** | A named Player-specific measure such as Patience; not a hard-coded field per future idea or untyped JSON. | `StatDefinition` defines code/name/unit/bounds/active state; `PlayerStat` stores one structured current value per Player+code. |
| **Progress Track** | One explicitly defined way to describe how a Concept is developing; not a universal `Concept.xp`. | A Concept may have zero or many independently named tracks. Definition has code, semantics, bounds, active state, metadata, and timestamps; current track has value, optional level, active state, metadata, and timestamps. `(concept, track_code)` is unique. |
| **Effect** | A temporary or persistent modifier/condition. It targets the Player by default and may explicitly target a same-world Concept. It is not automatically allowed to target every entity. | One Effect shape and lifecycle; target is a closed `player` or `concept` variant. Type, start, expiry, manual deactivation, metadata, created/updated times are structured. Lifecycle is derived as scheduled/active/expired/manually deactivated. |
| **Transaction** | An append-only historical change/event, not current state or a snapshot. | Structured Player/resource/amount/source and `occurred_at`; `captured_at` records local observation when known. XP preserves requested `amount` and actual `applied_amount`. Legacy rows may have unknown capture time. |
| **Comment** | User annotation on a supported entity; not an intentional story or progression transition. | Historical authored text; target kinds are closed and target existence/same-world ownership is checked at insert. |
| **Narrative Entry** | Intentional journal/story content; not a Comment or automatic ledger. | Player-scoped typed record with created/updated times. It can be linked to a Concept. |
| **History** | Records of changes and observations; not a catch-all `ConceptHistory` table. | Transactions remain the resource ledger; Concept progress uses append-only progress entries; rules keep execution audits; snapshots preserve selected state; mutable supported entities gain append-only before-image revisions. Explicit Concept links and target references let queries compose them. |
| **Lifecycle / Revision** | Recoverability for supported world records; not permanent deletion or an event-sourcing rewrite. | Active, archived and trashed are reversible lifecycle states. Revision snapshots preserve prior values; restoring a revision is a new mutation and generates new history. Soft deletion retains data for recovery. |
| **Presentation Preference** | A contextual visibility/layout choice; not world truth or a domain lifecycle state. | Per-Player, per-entity and per-context visibility, ordering, pin/collapse and optional display hints. Hidden entities continue to exist and can be included in search; active/archived/trashed remains a separate lifecycle filter. |
| **Daily Snapshot** | “This entity's state was captured on this calendar date.” It does not represent everything that happened all day. | Per-entity/date uniqueness, immutable JSON state with a schema version, and a precise capture timestamp (`created_at` on existing Player/Skill snapshots; `captured_at` on Concept snapshots). |
| **Rule** | Persistent declarative data interpreted by trusted application code; never an executable script or callback. | Versioned closed event/condition/action vocabulary, deterministic priority ordering, bounded chains, append-only execution audit. |
| **UI State / Workspace** | Interface-only selections/layout; not Player/world truth. | Contextual per-entity visibility is stored through Phase 3.6 preferences; route and declarative dashboard layout are UI-local persisted state. Neither changes domain truth. |

## Progress is not one universal number

Progress semantics are explicit and are not interchangeable merely because values are numeric:

- **XP / experience** records nonnegative whole accumulated experience. It is independent of level; there is no XP-to-level formula in the world model. An optional suggestion may propose progress, but cannot mutate authored state unless explicitly accepted.
- **Level** is a positive whole ordinal/rank. It is not a quantity to sum or average like a generic stat.
- **Percentage** is bounded to `0..=100` (and may have stricter definition bounds).
- **Numeric** is a finite measure interpreted by its definition and optional minimum/maximum.
- **Mastery** is an explicitly named mastery measure. It does not silently imply an XP formula or a particular scale; the definition bounds, if any, govern it.
- Names such as familiarity, confidence, understanding, strength, or endurance are data-defined track codes with explicit semantics; they are not compiled fields on Concept.

The implementation validates finite values, definition bounds, percentage ranges, whole-number experience/level rules, positive optional track level, and active definitions in the domain and SQLite. Player/Skill levels are explicitly set by the Player. Concept tracks are `manual` or `rule_controlled`; rules cannot overwrite a manual track. Suggestions are proposals until explicitly accepted. The system does not convert between tracks or invent level-up/unlock formulas.

## Concepts relate to, but do not absorb, other entities

A Concept is a subject around which typed records may be organized. For example, a Concept `Python` may relate to another Concept `Programming`, while a Quest `Finish Python course`, Skill `Python Programming`, Narrative Entry, Comment, Transaction, or Effect remains its own entity and can carry an explicit Concept link/target where appropriate.

Relationships are simple directed edges with a data-defined code. They do not implement arbitrary graph traversal, ontology inference, or a universal relationship table across every entity class. Explicit cross-entity links are intentionally closed to named supported kinds. SQLite checks same-Player ownership and foreign-key/reference existence as a final guard.

## Current state, events, history, snapshots, and narrative

These answer different questions and must stay separate:

- **Current state** answers “what is true now?” It lives in structured aggregates/current-value tables.
- **Occurred events** answer “when did a real-world event happen?” Use `occurred_at` when meaningful. Multiple events may have the same calendar date, and a Concept progress change may be backdated as long as it is not later than capture.
- **Capture time** answers “when did this installation observe/record the event or state?” Use `captured_at` on Concept progress history, Concept snapshots, and newly written Transactions. Legacy Transaction rows retain `NULL` when the historical capture moment is unknown; do not fabricate it from `occurred_at`.
- **Created/updated time** answer when a mutable record was first created and last changed. Immutable event records do not receive a meaningless `updated_at`; snapshots keep their established capture column name.
- **Daily snapshots** answer “what state was observed on this date?” They are immutable, unique per entity/date, and capture an exact instant. A second normal capture for that date is rejected. Days without a snapshot are gaps in recorded data—not zero activity, a negative state, or evidence that nothing happened. No interpolation or fake snapshots are generated.
- **Transactions** answer “what resource change was recorded?” They are append-only. XP operations update Player state and add the matching Transaction atomically; requested and applied deltas remain distinct.
- **Narrative** answers “what story/journal content did the Player intentionally record?” It does not silently change progression.

The model permits multiple timestamped events on one date. A daily snapshot and a chronological event stream are different records with different meanings.

Quest activity Sessions are actual periods with explicit start and optional end times. They are not generated from snapshots, checked calendars, or elapsed-time assumptions. Breaks and missing dates are valid data gaps. Revisions and lifecycle state are separate from event and snapshot history: deletion is soft, and restoration adds a new mutation rather than erasing the intervening record.

## Lifecycle, recovery, and contextual visibility

Supported mutable aggregates receive database-captured before-image revisions. A restore validates and reapplies a prior snapshot through the owning persistence adapter, producing a new revision so recovery itself remains auditable. Lifecycle state is `active`, `archived`, or `trashed`; trashed records remain recoverable and are filtered separately from active records. Permanent destruction is not part of this phase.

Visibility belongs to presentation preferences keyed by Player, entity, and context (for example, a future quest-board context). It never changes ownership, existence, lifecycle, or domain permissions. Search applies a visibility preference only when the caller supplies a concrete context; filters for archived and trashed records are independent, opt-in choices.

## Search and query foundation

Global search is an application-level query over a compact SQLite FTS5 projection. The projection stores searchable identity, kind, owner, type/status/active markers, occurrence time, name, and bounded text, plus unindexed metadata for lifecycle and contextual visibility filtering—not full aggregate JSON, metadata blobs, or a second world database. SQLite triggers keep it synchronized, and migration backfill indexes existing rows.

Search results remain references to typed domain entities. The query supports text, entity kind, Player, related Concept, type code, status, active state evaluated at the query instant for Effects, timestamp range, sort, limit, and offset. Text is sanitized to quoted AND terms so FTS operators cannot escape the query; inputs and result limits are bounded. Relevance uses FTS5 when text is present. Concept-related filtering follows explicit Concept links/Effect targets and active Concept relationships.

Search covers Player, Concept, Quest, Quest Stage, Branch and Session, Skill Tree, Skill, Effect, Narrative Entry, Comment, Transaction, and Concept progress history. Contextual hidden-state filtering and archived/trashed inclusion are explicit query choices. Tags, source/category-specific filters, a timeline UI, ranking tuning, and broad arbitrary JSON search are intentionally deferred. Specialized pages (Concept, Quest Board, Skill Tree, Journal, Search/Explorer) are built by the frontend from meaningful domain/query results; the backend does not create a `Page` entity for each screen.

## Phase 3 — declarative rule engine, extended compatibly

The application-layer interpreter accepts only closed typed Rust event, condition, and action variants serialized as data. Unknown variants, fields, incompatible subjects, and unsupported schema versions fail closed. There is no expression evaluator, database-stored code, scheduler, daemon, or polling.

Supported typed events are `quest_completed`, `player_xp_changed`, `stat_changed`, and `concept_progress_changed`. The Concept progress event includes Player, Concept, Concept type, track code, prior/current value, and optional level. Conditions read only the fields present on their event; they do not query arbitrary live state. The additional action `set_concept_progress` validates same-Player ownership, active track definitions, semantic bounds, finite values, and optional level. Its follow-on event enters the same FIFO chain.

Rule planning is bounded (depth 8, 16 actions/rule, 32 actions/chain, 64 evaluations/chain) and deterministic. A source Concept progress update, all derived progress/XP/Quest/stat operations, and successful execution history commit or roll back together through `WorldStore::apply_rule_chain`. Stale expected values/ownership and database constraints are rechecked in SQLite. Failure audit is attempted separately after rollback. Effect targeting is modeled now; Effect activation/deactivation rule actions and Skill unlock mechanics remain deferred.

## Extensibility and integrity boundaries

- **Strongly typed/structured:** identity, owners, type references, statuses, values, bounds, relationship endpoints, target kinds, dates/times, occurrence/capture, and fields needed for integrity and queries.
- **Data-defined:** Concept/Quest/Skill/Effect/Narrative types, relationship codes, and Progress Track definitions. New meaningful type rows need not require a new Rust enum or schema migration.
- **Versioned JSON:** only intentionally extensible metadata and selected immutable snapshots. Core identity, measures, lifecycle, history, and searchable fields are not hidden in JSON.
- **Frontend-defined:** presentation, layout, transient UI state, and views that do not define world truth.
- **SQLite guards:** foreign keys and triggers reinforce same-Player relationships/Effect targets, valid Concept links, progress bounds, append-only histories, immutable snapshots, and synchronized FTS projection.

## Architecture boundaries

| Layer | Allowed responsibility |
|---|---|
| `lr-domain` | Pure world concepts, validated values, state transitions, and invariants; no database or UI. |
| `lr-application` | Use cases, typed search/detail queries, clocks/IDs, rule execution, and persistence ports; no SQL. |
| `lr-persistence` | SQLite migrations, row mapping, indexes/triggers, FTS projection, and atomic adapter. |
| `lr-contracts` | Serializable IPC DTOs; translates values without making frontend types domain types. |
| `src-tauri` | Composition root and thin commands; no SQL or game rules. |
| `src/` frontend | Typed IPC client and presentation/UI; no direct DB access. |

The intended flow remains **React + TypeScript → typed Tauri IPC → application service/port → SQLite adapter**.

## Intentionally deferred

- Phase 4 UI implementation: workspaces, workspace layout, Concept page, Search/Explorer, timeline, and visual polish. Phase 3.6 stores contextual preferences as backend data only; no screens consume them yet.
- Universal entity/property/event frameworks, a full knowledge graph, event-sourcing rewrite, a separate search database, CMS, or no-code rule platform.
- Tagging until a concrete organization use case justifies reusable tags.
- Arbitrary live-state rules, scheduling/background processing, Concept-triggered Effect activation, Skill unlock formulas, and inference across Concept graphs. XP-to-level conversion is not a planned progression rule: XP and manually authored levels remain independent.
- Cloud synchronization, accounts, authentication, HTTP API, and multiplayer.

Do not implement future concepts merely because they appear in the roadmap.
