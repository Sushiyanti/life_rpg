# Life RPG — Domain Design Codex

This is the concise conceptual reference for the product's world model. Read it with [`ARCHITECTURE.md`](ARCHITECTURE.md) before changing domain behavior. The current implementation is Phase 2.1; future areas below are boundaries, not features to build early.

## Product philosophy

Life RPG is a **local-first application that represents a player's real life as a game-like world**. It is more than a productivity or task-list app: its purpose is to give the player's state, progression, quests, skills, effects, events/history, narrative, and eventually customizable presentation a coherent, persistent game-world representation. The user's world stays on their machine in SQLite; there is no account, cloud dependency, or server.

The backend/domain establishes what an entity **is**, its state, and what happened to it. The frontend decides how that world **looks and feels**. Presentation must not change or corrupt domain meaning.

## Core vocabulary

| Concept | Represents / is not | Time, extension, and relationships |
|---|---|---|
| **Player** | A character representing the user in this world; not a login/account or the whole application. | Current aggregate: name, level, XP, active flag, metadata. Owns quests, skill trees, effects, stats, transactions, snapshots, and narrative. Core columns are structured. |
| **State** | What is true of an entity now; not an event log or a historical record. | Current state lives on aggregates and current-value tables. Structured invariants are domain/database-enforced where possible. |
| **Daily State Snapshot** | A preserved answer to “what did this character/skill look like on this date?”; not a live view or an event. | Immutable historical record, unique per entity/date. Player v1 stores level and XP in columns and versioned `state_json` containing active dynamic stats and currently active effects. Skill snapshots store level, XP, status, invested minutes and versioned tree/parent context. Capture reads and inserts atomically. |
| **Stat** | A named, player-specific measure such as Patience or Programming Focus; not a hard-coded field for each future game concept, and not an untyped Player blob. | `StatDefinition` describes a data-defined code/name/description/unit/bounds/active state; `PlayerStat` stores one structured value per player+code. New concepts are rows, not migrations. Metadata is reserved for genuinely extensible detail. Snapshots preserve both definition context and value. |
| **Quest** | A goal/objective in the world; not a separate table per genre or a generic UI card. | Current state and type are structured. Main/side/daily/challenge are normally registry-backed variants of one concept. Phase 2 statuses are `open` (planned), `active`, `completed`, and `abandoned`; pause/failure transitions are intentionally not represented yet. |
| **Quest hierarchy** | Parent/child decomposition of goals; not permission to cross between players' worlds. | Current relationship. Parent and linked skill must belong to the same Player. Self-parenting and indirect cycles are rejected. SQLite triggers enforce ownership and acyclicity as a final guard. |
| **Skill Tree** | A Player-owned grouping of related abilities; not the skill itself or a presentation layout. | Current entity with an explicit `is_active` flag. Type is data-defined. Owns its Skills. |
| **Skill** | An ability/practice area that can progress; not a Quest. | Current state includes level, XP, invested minutes, and status (`active`, `paused`, `completed`, `archived`). A skill and its parent must live in the same tree; parent cycles are rejected. |
| **Skill state** | The present progression of one Skill; not its full history. | Current fields stay structured. Daily history is a separate `SkillStateSnapshot`; time-investment events are separate Transactions. |
| **Effect** | A temporary or persistent modifier/condition attached to a Player; not an XP/resource ledger entry. | Shared entity; Buff and Debuff are effect types/categories, not separate domain classes. Lifecycle is derived explicitly: `scheduled`, `active`, `expired`, or `manually deactivated`. Expiry is time-based; `deactivated_at` is a durable, non-destructive manual-off marker. Snapshots capture only effects active at capture time. |
| **Buff / Debuff** | Positive/negative semantics of an Effect; not distinct storage models. | Data-defined `effect` types; one Effect lifecycle and storage shape. |
| **Transaction** | An append-only historical change/event (XP, time, or another named resource); not current state or a snapshot. | Player and timestamp are structured. XP stores `amount` (requested event) and `applied_amount` (actual state delta). For a penalty that exceeds current XP, requested remains negative while applied is clamped so resulting XP is zero. XP events are written only through atomic state-changing operations. |
| **Comment** | User-authored annotation attached to a supported entity; not intentional story content or a domain transition. | Historical text. Its polymorphic target is checked for existence in the same insert transaction; target kinds are closed and explicit. |
| **Narrative Entry** | Intentional journal/story content in the world; not a Comment or automatic ledger. | Player-scoped authored record, separate from progression. Its kind is extensible through the type registry. |
| **Rule** | A future controlled way to make world transitions automatic; not an ad-hoc callback or persisted user script. | Not implemented in Phase 2.1. Phase 3 may introduce a constrained **Condition → Trigger → Action** model; exact runtime semantics are deliberately unspecified here. |
| **UI State** | Interface-only choices such as current selection or panel arrangement; not Player state. | Frontend/presentation concern unless a later phase deliberately persists it in a separate store. It must not affect domain validity. |
| **Presentation** | The visual rendering of domain concepts; not the concepts themselves. | Frontend-defined and controlled. Variants, density, and style do not redefine Quest/Skill/Effect semantics. |
| **Workspace** | A future saved arrangement of views/tools; not a Skill Tree or domain container. | Not persisted in Phase 2.1. If added, it must remain distinct from Player/world entities. |

## Current state, events, snapshots, and narrative

These records answer different questions and must stay separate:

- **Current state** answers “what is true now?” It is optimized for ordinary reads and is represented by structured aggregate/table fields.
- **Transaction history** answers “what event/change was recorded?” It is append-only. XP penalties preserve both the requested event and actual applied delta; generic resource amounts remain their recorded amount.
- **Daily snapshots** answer “what state was observed on this date?” They are immutable point-in-time records and never replace the current aggregate or ledger.
- **Narrative records** answer “what story/journal content did the user intentionally record?” They do not silently change stats or progression.

An XP award/penalty updates Player state and inserts its Transaction in one SQLite transaction. Quest completion plus an optional XP reward, and Skill time investment plus its Transaction, are also atomic. A failed validation/write must leave both the current state and history unchanged.

## Extensibility boundary

Guiding principles: **do not hardcode every future game concept; do not turn the application into untyped JSON.**

- **Strongly typed/structured:** identity, owner links, level, XP, status, dates/timestamps, stat code/value, bounds, transaction amount, hierarchy, and relationships needed for constraints or queries.
- **Database-defined:** Quest/Skill Tree/Skill/Effect/Transaction/Narrative type vocabulary and Player Stat definitions. A new stat or semantic type should normally be inserted as data, not compiled into a field or schema migration.
- **Versioned JSON:** only intentionally extensible snapshots and metadata. Player snapshot `state_json` has `schemaVersion: 1`, `stats`, and `activeEffects`; these are explicit semantic fields, not serialized Rust structs. Skill snapshot JSON preserves `schemaVersion`, `skillTreeId`, and `parentSkillId`. Core historical fields remain columns.
- **Frontend-defined:** layout, labels/presentation choices that do not define domain truth, and transient UI state.

## Lifecycle and integrity decisions

- **Player:** one active/inactive flag; no duplicate status enum. XP is always at least zero.
- **Quest:** planned/open → active → completed or abandoned. Phase 2.1 does not invent pause/failed states or a rules engine.
- **Skill:** explicit current status among active/paused/completed/archived. State changes and invested time do not erase prior Transactions or snapshots.
- **Skill Tree:** active/inactive flag; archive/delete workflow is not added here.
- **Effect:** scheduled/active/expired/manual-off is evaluated against timestamps and the deactivation marker. Deactivation is retained rather than deleting the Effect.
- **Transaction:** append-only; it has no active/inactive lifecycle. Correction is another event, never rewriting history.
- **Comment / Narrative Entry:** authored records; neither is a progression lifecycle. Phase 2.1 does not add a broad edit/delete workflow.
- **Hierarchy:** relational FKs prove referenced rows exist, but not shared ownership or absence of cycles. SQLite triggers additionally check owner/tree consistency and recursive ancestor chains. Application/domain checks reject direct self-parenting early; database checks remain authoritative for the stored graph.
- **Snapshot:** unique per player/date or skill/date. A second ordinary capture for that day is rejected; historical rows are not overwritten.
- **Stat:** one current value for each `(player, stat_code)`; definition bounds and active status are enforced in the domain and database. A later value overwrites current state, while daily snapshots preserve observed history.

## Architecture boundaries

| Layer | Allowed responsibility |
|---|---|
| `lr-domain` | Pure world concepts, validated values, state transitions, and invariants. No SQLite, Tauri, React, IPC, or file access. |
| `lr-application` | Commands/queries, use cases, clocks/IDs, and persistence ports expressed in domain terms. No SQL or desktop/frontend types. |
| `lr-persistence` | SQLite schema/migrations, row mapping, constraints/triggers, and atomic storage adapter. The only Rust layer that knows SQL. |
| `lr-contracts` | Serializable, camelCase IPC DTOs. It translates domain/use-case results without making frontend types domain types. |
| `src-tauri` | Composition root and thin command adapters; no SQL and no game rules. |
| `src/` frontend | Typed IPC client and presentation/UI. No direct DB access; UI appearance does not determine domain meaning. |

The intended flow remains **React + TypeScript → typed Tauri IPC → application service/port → SQLite adapter**.

## Intentionally future work (not implemented here)

- **Phase 3 rules engine:** constrained Condition/Trigger/Action runtime; not in this branch.
- **Dynamic UI state, presentation persistence, workspaces, and Style Sandbox:** separate interface concerns; no schema/runtime here.
- **Complete character UI, quest board, and skill-tree UI:** beyond the small Phase 2 verification surface.
- **Cloud synchronization, accounts, authentication, HTTP API, and multiplayer:** outside the local-first single-user architecture.

Do not implement these early merely because the conceptual model mentions them.
