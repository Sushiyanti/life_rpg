# Life RPG — Domain Design Codex

This is the canonical conceptual reference for the product's world model. Read it with [`ARCHITECTURE.md`](ARCHITECTURE.md) before changing domain behavior. The current implementation includes Phase 11's Player-owned Tags and portable Workspace filters, plus Phase 10's bounded gameplay Rules and Skill/Effect progression semantics. Phase 4 adds presentation, not new world meaning; the [Phase 4 report](PHASE-4-REPORT.md) records the UI boundary and the Phase 4.1 correctness hardening.

## Product philosophy

Life RPG is a **local-first application that represents a player's real life as a persistent game-like world**. The backend/domain establishes what something **is**, what state it has, what happened to it, and when that was observed. The frontend decides how that world **looks and feels**. Presentation must not redefine or corrupt domain meaning.

The database remains a typed relational world, not a universal object/property framework. Shared abstractions are used only where they express real shared semantics.

## Core vocabulary

| Entity | Represents / is not | Structured semantics |
|---|---|---|
| **Player** | A character representing the user in this world; not an account or the application. | Owns the local world and its Player-level progression. Level and optional descriptive labels are Player-authored; XP is a separate ledger and never computes or overwrites level. |
| **Tag** | A lightweight Player-owned organizational label; not a type, Concept, status, permission, or inferred classification. | Stable identity and transfer key; case-insensitive normalized name unique within the Player; optional description; active/archived/trashed lifecycle; created/updated timestamps. |
| **Tag relationship** | An explicit organizational assignment to a supported record; not a semantic change to the target. | Player-scoped relation to Quest, Stage, Branch, Session, Skill Tree, Skill, Concept, Effect, Narrative Entry, or Comment, with added/removed timestamps. Removal is retained as history; reattachment creates a new fact. |
| **Concept** | A meaningful subject in the Player's world, such as Python, Health, a project, a place, a person, or a goal. It is **not** a Quest, Skill, NarrativeEntry, Comment, or generic container. | Player-owned; data-defined Concept type; name/description/active state/metadata and created/updated timestamps. Concepts can have their own relationships and zero or more typed Progress Tracks. An immutable Concept-only transfer key supports workspace filter references; it is not a public account identity or general entity ID. |
| **Concept type** | A useful classification of a Concept; not a closed Rust enum. | Rows in the existing `type_definitions` registry under the `concept` namespace. Adding a type is normally data/configuration, not a code change or migration. Seed examples include subject, project, life_area, person, and place. |
| **Concept relationship** | A typed, directed relation between two Concepts, not a general graph engine. | Source, target, data-defined relationship code, active state, metadata, created/updated timestamps. Both endpoints must belong to the same Player; self-relations are rejected. Seed relationship codes include `related_to`, `parent_of`, `child_of`, `depends_on`, `part_of`, `derived_from`, and `prerequisite_of`. |
| **Quest** | A goal/objective in the world; not a Concept. | Existing Quest type/status/progress/lifecycle remains structured. A Quest may be explicitly linked to one or more Concepts. Parent/child and linked Skill ownership/cycle invariants remain intact. Optional ordered Stages and alternative Branches provide explicit hierarchy without requiring every Quest to use them. |
| **Quest Stage / Branch** | Optional breakdown and alternatives within a Quest; not independent Quests or Concepts. | Stages belong to one Quest. Branches belong to one Stage and its Quest. Both are same-Player constrained, ordered records with their own bounded status vocabularies. |
| **Quest Session** | A real, timestamped period of activity; not a daily summary or inferred streak. | Start/end timestamps, optional Quest/Stage/Branch/Skill/Concept context, status and authored notes/result. Effect context is an explicit typed Session↔Effect relation, never inferred from co-existence. Multiple sessions may occur on a date. Missing periods remain missing; no synthetic activity or snapshots are generated. |
| **Skill Tree / Skill** | A Player-owned grouping and a distinct ability/practice area; neither is a Concept. | Skill level, label, lifecycle, nonnegative XP, invested minutes, availability, and Rule authority remain independent. XP is recorded as a Skill-specific Transaction; level remains manually controlled. A parent/child hierarchy is organization, not a prerequisite graph. |
| **Stat** | A named Player-specific measure such as Patience; not a hard-coded field per future idea or untyped JSON. | `StatDefinition` defines code/name/unit/bounds/active state; `PlayerStat` stores one structured current value per Player+code. |
| **Progress Track** | One explicitly defined way to describe how a Concept is developing; not a universal `Concept.xp`. | A Concept may have zero or many independently named tracks. Definition has code, semantics, bounds, active state, metadata, and timestamps; current track has value, optional level, active state, metadata, and timestamps. `(concept, track_code)` is unique. |
| **Effect** | A temporary or indefinite modifier/condition. It targets the Player by default and may explicitly target a same-world Concept. It is not automatically allowed to target every entity. | One Effect shape and lifecycle; target is a closed `player` or `concept` variant. Type, start, optional expiry, deactivation time, and manual-vs-Rule source are structured. A future start is `scheduled`; an elapsed explicit expiry is `expired`; manual and Rule deactivation are distinct. Deriving expiry never writes to the database. |
| **Transaction** | An append-only historical change/event, not current state or a snapshot. | Structured Player/resource/amount/source and `occurred_at`; `captured_at` records local observation when known. XP preserves requested `amount` and actual `applied_amount`. Skill XP uses the same ledger with resource `skill_xp`, Skill identity, previous/current XP and source/reason metadata; it never changes Skill level. Legacy rows may have unknown capture time. |
| **Comment** | User annotation on a supported entity; not an intentional story or progression transition. | Historical authored text; target kinds are closed and target existence/same-world ownership is checked at insert. |
| **Narrative Entry / Content** | Intentional reusable authored material—notes, guides, guidance, briefings, stories, lore, instructions, readings, reminders, todos, reflections, journals, references, summaries, and introductions; not a Comment, Concept, automatic ledger, or CMS Page. | One Player-owned canonical typed record with title/body, optional author/source fields, metadata, active flag and created/updated times. Content kinds remain rows in `type_definitions` under `narrative_entry`, not a Rust enum. |
| **Content relationship** | An explicit use of one Content record at one supported world target; not inheritance, a generic graph edge, a target mutation, or a duplicate copy of content. | A timestamped Player-scoped fact with stable identity, closed target kind (`player`, Quest, Stage, Branch, Session, Skill, Skill Tree, Concept, Effect), role code, deterministic order, added time and optional removal time. Same-world ownership/target existence are checked in application logic and SQLite. Removing a relationship retains the fact and never deletes content or its target. |
| **History** | Records of changes and observations; not a catch-all `ConceptHistory` table. | Transactions remain the resource ledger; narrow append-only Skill history records availability/policy transitions; Concept progress uses append-only progress entries; Effects use append-only before/after lifecycle/change entries; Rules keep execution audits; snapshots preserve selected state; supported mutable entities gain before-image revisions. |
| **Lifecycle / Revision** | Recoverability for supported world records; not permanent deletion or an event-sourcing rewrite. | Active, archived and trashed are reversible lifecycle states. Revision snapshots preserve prior values; restoring a revision is a new mutation and generates new history. Soft deletion retains data for recovery. |
| **Presentation Preference** | A contextual visibility/layout choice; not world truth or a domain lifecycle state. | Per-Player, per-entity and per-context visibility, ordering, pin/collapse and optional display hints. Hidden entities continue to exist and can be included in search; active/archived/trashed remains a separate lifecycle filter. |
| **Daily Snapshot** | “This entity's state was captured on this calendar date.” It does not represent everything that happened all day. | Per-entity/date uniqueness, immutable JSON state with a schema version, and a precise capture timestamp (`created_at` on existing Player/Skill snapshots; `captured_at` on Concept snapshots). |
| **Rule** | Persistent declarative data interpreted by trusted application code; never an executable script or callback. | Versioned closed event/condition/action vocabulary, deterministic priority ordering, bounded chains, append-only execution audit. |
| **UI State / Workspace** | Interface-only selections/layout; not Player/world truth. | Contextual per-entity visibility is stored through Phase 3.6 preferences; declarative Workspace panels can carry supported any/all Tag filters. Workspace transfer v4 uses stable Tag keys, never local Tag row IDs. Neither changes domain truth. |

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
- **Narrative / Content** answers “what authored story, journal-style note, guidance, or reference did the Player intentionally record in the Content Guidebook?” It does not silently change progression. Journal-style Content remains distinct from Comments and Sessions.

The model permits multiple timestamped events on one date. A daily snapshot and a chronological event stream are different records with different meanings.

Quest activity Sessions are actual periods with explicit start and optional end times. They are not generated from snapshots, checked calendars, or elapsed-time assumptions. Breaks and missing dates are valid data gaps. Revisions and lifecycle state are separate from event and snapshot history: deletion is soft, and restoration adds a new mutation rather than erasing the intervening record.

## Lifecycle, recovery, and contextual visibility

Supported mutable aggregates receive database-captured before-image revisions. A restore validates and reapplies a prior snapshot through the owning persistence adapter, producing a new revision so recovery itself remains auditable. Lifecycle state is `active`, `archived`, or `trashed`; trashed records remain recoverable and are filtered separately from active records. Permanent destruction is not part of this phase.

Visibility belongs to presentation preferences keyed by Player, entity, and context (for example, a future quest-board context). It never changes ownership, existence, lifecycle, or domain permissions. Search applies a visibility preference only when the caller supplies a concrete context; filters for archived and trashed records are independent, opt-in choices.

## Search and query foundation

Global search is an application-level query over a compact SQLite FTS5 projection. The projection stores searchable identity, kind, owner, type/status/active markers, occurrence time, name, and bounded text, plus unindexed metadata for lifecycle and contextual visibility filtering—not full aggregate JSON, metadata blobs, or a second world database. SQLite triggers keep it synchronized, and migration backfill indexes existing rows.

Search results remain references to typed domain entities. The query supports text, entity kind, Player, related Concept, content target kind, type code, status, active state evaluated at the query instant for Effects, timestamp range, sort, limit, and offset. Text is sanitized to quoted AND terms so FTS operators cannot escape the query; inputs and result limits are bounded. Relevance uses FTS5 when text is present. Concept-related filtering follows explicit Concept links/Effect targets, active Concept relationships, and active Content↔Concept relationships.

Search covers Player, Concept, Quest, Quest Stage, Branch and Session, Skill Tree, Skill, Effect, Narrative Entry, Comment, Transaction, and Concept progress history. Contextual hidden-state filtering and archived/trashed inclusion are explicit query choices. Global Search also accepts a bounded list of Player-owned Tag IDs with `any`/`all` semantics, applied relationally to current assignments rather than copied into FTS text. Source/category-specific filters, ranking tuning, and broad arbitrary JSON search remain deferred. Specialized pages (Concept, Quest Board, Skill Tree, Content Guidebook, Search/Explorer) are built by the frontend from meaningful domain/query results; the backend does not create a `Page` entity for each screen.

## Phase 3 — declarative rule engine, extended compatibly

The application-layer interpreter accepts only closed typed Rust event, condition, and action variants serialized as data. Unknown variants, fields, incompatible subjects, and unsupported schema versions fail closed. There is no expression evaluator, database-stored code, scheduler, daemon, or polling.

Phase 3 introduced `quest_completed`, `player_xp_changed`, `stat_changed`, and `concept_progress_changed`; Phase 10 adds the typed gameplay events documented below. Conditions read only fields present on their event; they do not query arbitrary live state. `set_concept_progress` still validates same-Player ownership, active track definitions, bounds and finite values. All follow-on events use the same FIFO chain.

Rule planning is bounded (depth 8, 16 actions/rule, 32 actions/chain, 64 evaluations/chain) and deterministic. Root operations, all derived changes and successful execution history commit or roll back together through `WorldStore::apply_rule_chain`. Stale expected values/ownership and database constraints are rechecked in SQLite; failure audit is attempted separately after rollback.

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

- Additional UI customization beyond the first-generation Phase 4 workspace: unrestricted page authoring, a timeline, rich-text editing, and bespoke detail layouts for every entity kind. Contextual preferences are consumed by the implemented UI; Phase 4.1 hardening and its boundaries are recorded in the [Phase 4 report](PHASE-4-REPORT.md).
- Universal entity/property/event frameworks, a full knowledge graph, event-sourcing rewrite, a separate search database, CMS, or no-code rule platform.
- Arbitrary live-state rules, scheduling/background processing, Skill prerequisites/formulas, and inference across Concept graphs. XP-to-level conversion is not a progression rule: XP and manually authored levels remain independent.
- Cloud synchronization, accounts, authentication, HTTP API, and multiplayer.

Do not implement future concepts merely because they appear in the roadmap.


## Phase 5.1 — workspace presentation configuration

A **Workspace** belongs to exactly one Player world and stores only an interface name/template/default choice. A **Workspace Panel** is a reusable presentation instance inside one Workspace—not a canonical aggregate, relation, quest, or duplicate world record. Multiple instances may use the same source and carry distinct titles, filters, sort order and layout. The migration ledger is forward-only and currently reaches version 17: migrations 10–15 preserve and extend Workspace, Content, Effect/Session and Timeline data; migration 16 adds Skill availability/history, Effect deactivation provenance, safe Rule-event values and Skill XP Transaction support; migration 17 adds Player-owned Tags, explicit Tag assignment history and declarative Workspace Tag filters while preserving existing rows and lifecycle guards.

Panel source codes are closed (`player`, `quests`, `skills`, `concepts`, `progress`, `effects`, `activity`, `transactions`, `journal`). Per-source variants, status codes, sort choices and filter capabilities are validated in typed registries plus Rust domain validation and SQLite constraints. A Concept selector narrows the bounded global-search projection via the existing explicit Concept links/associations; it never converts the referenced Quest, Skill, session, ledger entry, or Narrative into a Concept. Export/import is versioned declarative JSON with an exact field allowlist.

Keep these independent axes explicit:

- **Entity lifecycle:** active / archived / trashed and recovery.
- **Entity contextual visibility:** per Player, record, and context (for example, Dashboard), without changing the record.
- **Panel visibility:** whether one panel instance renders in its workspace.
- **Workspace membership:** which presentation instances make up a saved view.

Moving, configuring, hiding, pinning, collapsing, importing, duplicating, or deleting a workspace/panel changes presentation settings only. The final workspace is protected and Player ownership is checked through application and SQLite boundaries. This customization layer has no XP-to-level derivation, no inferred activity, and no world-write side effects.

### Phase 5.2 — workspace transfer references

Workspace versions 2 and 3 are declarative presentation configuration, not a world export. Concept filters carry `{ key, name, typeCode }`; the immutable key is assigned to a Concept and is never an internal row ID. Same-world imports resolve the exact key to the destination's existing Concept. Across independently created worlds, names/type codes are suggestions only and require explicit player choice; an absent or ambiguous match remains unfiltered with a visible warning. Legacy version 1 Concept IDs are discarded during migration; validated versions 1 and 2 now upgrade to strict version 3. Workspace/panel writes are one transaction, and both the application service and persistence adapter enforce destination Player ownership. No Quest, Skill, Concept, Session, Transaction, or other world record is created by workspace import. See [Phase 5.2 report](PHASE-5.2-REPORT.md) for the original validation/compatibility contract and [Phase 9 report](PHASE-9-REPORT.md) for v3 behavior.

### Phase 6 — player-facing interaction

> Phase 6 improves interaction with the existing world; it does not redefine canonical world semantics merely to make the UI more game-like.

The Player Hub is a contextual projection of the currently selected Player's persisted records. “Current” and “active” refer to recorded aggregate/status/lifecycle fields—not inferred daily activity. The timeline contains only timestamped world records that exist; elapsed duration is shown only when a Session has valid recorded start and end times. Player/Skill levels, stats, Concept tracks, Quest progress, Session outcome, and contextual links change only through their existing explicit domain/application commands. Quick Capture creates an existing typed Narrative Entry; context attachment is opt-in, and an unattached note is valid.

Opening an Explorer record or following a Session's existing Quest/Skill/Concept context is navigation, not a world mutation. Workspace panels stay player-owned presentation configuration, and contextual visibility stays distinct from lifecycle and domain status. No new general graph, inferred progression, synthetic activity, account identity, or automatic leveling was introduced. See [Phase 6 report](PHASE-6-REPORT.md) for the interaction/UI boundary and verification.

### Phase 6.1 — Effect lifecycle and explicit Session context

`expires_at` is optional; `NULL` is intentional absence of a recorded expiry, not a default duration or a hidden “off” value. When present, expiry remains an authored timestamp and `expired` is derived at the query/display instant. No elapsed-time check mutates `deactivated_at`, synthesizes an Effect history entry, or runs in a daemon. Manual deactivation is a separate explicit Player action with its own timestamp and history event. A later read can therefore distinguish scheduled, active, expired and manually deactivated records without rewriting the record.

The dedicated append-only Effect history records created state, meaningful details changes, expiry changes, manual deactivation and Session-link/unlink actions with before/after snapshots. It complements rather than replaces Transactions, Concept progress history, entity revisions, or the rule audit. An explicit `SessionEffect` has a bounded role (`relevant`, `applied`, `removed`, or `observed`), Player/Session/Effect IDs and added/removed timestamps. Same-Player ownership is checked by the application and SQLite. Creating/deactivating an Effect from a selected Session may record that explicit action's relationship; the system never attaches all active Effects to every Session. Removing a relationship only marks that relationship removed; neither the Effect nor Session is deleted or deactivated.

### Phase 6.1.1 — expired Effect correction

An Effect whose recorded `expires_at` is at or before the current operation time is already `expired`; it cannot also be manually deactivated. The domain rejects the operation (including an exact expiry-time tie), so the application writes neither `deactivated_at` nor a `manually_deactivated` history entry. The Effects manager and Player Hub offer manual deactivation only for currently `active` Effects; scheduled, expired and manually deactivated records remain distinguishable and inspectable. A future-expiring or indefinite active Effect remains manually manageable, while a scheduled Effect cannot be manually deactivated before its start.

### Phase 7 — reusable Content & Guidance

`NarrativeEntry` is the canonical reusable Content aggregate; the persisted identifier/table name remains stable for backwards compatibility. Its kind is data-defined under the existing `narrative_entry` registry (including `introduction`), while relationship roles remain small data rows rather than a hard-coded future vocabulary. Content's source fields remain provenance fields, not implicit target associations; Phase 7 deliberately does not translate them into relationships.

Migration 13 evolves legacy `content_attachments` into timestamped relationship facts with a stable relationship ID, `sort_order`, and `removed_at`. It preserves every legacy row: active rows remain active, while a legacy inactive row records its pre-existing `updated_at` as the only known removal observation. Active rows are uniquely constrained by content/target/role; removal makes a later reattachment a new fact rather than overwriting history. Relationships are closed, same-world guarded, and separate from Content lifecycle, target lifecycle, and contextual presentation visibility. Content edits flow through the existing Narrative before-image/FTS triggers; content archive/trash/restore uses the existing lifecycle/recovery path. No rich text, automatic attachment, Page entity, generic graph, scheduler, or automatic progression is introduced.

Descriptive edits to an expired Effect do not alter its expiry or reactivate it. The owner may deliberately change the recorded expiry, including extending it into the future or clearing it to `NULL`; the ordinary before/after `expiry_changed` history event records that authored lifecycle edit. If the resulting start/expiry/manual-off facts evaluate as active, the derived display state becomes `active` without a synthetic reactivation event. No migration, clock-driven write, scheduler, or background process is needed. An expired Effect's explicit Session relationships and history remain intact.


### Phase 8 — unified read-only Timeline

The Timeline is a **read-oriented chronological projection of existing persisted world/history facts**, not a canonical event store. SQLite composes the current Player’s records directly; there is no `timeline_events` table, second FTS database, event backfill, or write-on-view. Stable source IDs retain the source record and the exact canonical entity reference used by existing detail navigation. `TimelineService` validates the closed category/entity/timestamp vocabulary, inclusive date bounds, sort, page size, and offset before the dedicated persistence port runs a Player-scoped, SQL-filtered `UNION ALL` page.

Categories stay distinct: Session records; Transactions; Effect-history rows; Content created/updated facts; authored Comments; Concept-progress history; mutable-record revisions; state snapshots; meaningful persisted Player/Quest/Skill timestamps; explicit lifecycle-history rows; and Content relationship created/removed facts. A Session has one item: its persisted start is the primary timestamp and its optional persisted end is additional detail; no completion event is synthesized when the end is absent. Date bounds use each item's primary timestamp. Where other sources have an independently recorded secondary time (such as Transaction capture or audit capture), the DTO labels both meanings. Snapshots are **captures**, not activity. Revisions describe a recorded mutable-record before-image; lifecycle facts carry explicit prior/current states. No missing dates, days, app opens, elapsed time, or inferred actions are filled in.

Effect expiry is protected: the Timeline reads only the append-only Effect history table. A stored expiry may influence a separate derived current-state display, but crossing expiry never creates a Timeline event; manual deactivation remains explicit. Content remains one `NarrativeEntry`: one creation plus a later authored update can be shown, while reuse across several targets does not duplicate Content events. An explicit Content relationship has a separate `relationship_history` category, with creation and removal represented only by their existing timestamps. Existing same-Player ownership and lifecycle rules are unchanged.

The UI uses a dedicated first-class Workspace route, with typed category/entity/Concept/date filters, explicit newest/oldest order, 50-row pages, readable date groups, no synthetic empty days, accessible loading/error/empty states, non-color category labels, truncation with full-summary access, and exact-source buttons. The Player Hub retains its compact 7/18-item contextual presentation but calls the same bounded query and applies its existing visibility preferences; “View full timeline” goes to the full route. Explorer, Guidebook, Effects and History/recovery remain separate rather than being replaced by the Timeline. Migration 14 adds only source-specific timestamp indexes needed by the composed query and is tested both for a fresh database and upgrade from schema 13. See [Phase 8 report](PHASE-8-REPORT.md) for sources, verification, native-build result, and intentionally deferred work.

### Phase 9 — Timeline panels and relationship context

The Timeline is an ordinary reusable Workspace panel source: multiple instances are valid, Builder settings are declarative, and the panel calls the existing read-only Timeline query. Category and entity-kind values remain closed vocabularies; date limits are inclusive calendar dates; exact entity IDs are paired with an entity kind and applied within the panel's Player scope. The panel's sort and result bound are presentation choices only. Migration 15 adds nullable constrained fields, leaving pre-existing workspaces unchanged.

Workspace transfer format 3 carries category, entity kind and date filters but never exports a local entity ID. An exact identity becomes a non-identifying local-record descriptor; preview/import reports that it must be reselected and imports that filter as neutral. Validated version 1 and 2 imports upgrade to v3. Relationship-history Timeline rows expose the recorded Content title/role, relationship identity and timestamps and provide separate navigation to the existing Content and target records; no duplicate Content or synthetic relationship facts are created. Workspace operations remain configuration-only and never mutate world state. See [Phase 9 report](PHASE-9-REPORT.md) for verification.


## Phase 10 — explicit gameplay Rules and Skill progression

Skill XP is an independent nonnegative whole-number measure. An explicit signed XP adjustment floors at zero and records requested versus applied delta in the immutable `transactions` ledger (`resource='skill_xp'`). Its metadata preserves Skill ID, previous/current XP, source and reason; the shared row also retains the canonical Skill identity and recorded/captured timestamps. Rule-generated XP uses the same ledger and source metadata. The Skill XP operation updates only `current_xp`; it never changes the Player-authored Skill `level`, `level_name`, `progression_label`, invested minutes or lifecycle. There is no XP-to-level formula or automatic leveling.

Availability is a separate axis from lifecycle: a Skill can be `locked` while its lifecycle status is `active`. `availability_control` is either `manual` or `rule_controlled`. A Player may explicitly lock/unlock at any time; a manual availability change reclaims manual authority. An enabled Rule can unlock only when the Skill is explicitly Rule-controlled. A Rule cannot override a manually controlled lock; this fails the chain atomically. Already-available unlocks are deterministic successful no-ops with audit but no duplicate history/event. No prerequisite is inferred from tree position, parent, name, order or level.

The narrow append-only `skill_history` stores only real availability/control transitions, before/after state, source, owner, Skill and recorded time. XP changes remain in Transactions rather than being duplicated in that table. The Timeline composes both sources directly: Skill XP Transaction rows and real Skill availability history. Daily Skill snapshots remain immutable captures, not mutation events.

Phase 10 extends the version-1 closed Rule vocabulary with `skill_xp_changed`, `skill_unlocked`, `session_started`, `session_finished`, `effect_created`, and `effect_deactivated`. Skill XP events carry Player/Skill identity, previous/current XP, requested/applied delta and source; unlock events carry identity, prior availability and source. Session events originate only from explicit Player Session commands. They carry recorded Session/context identifiers and recorded status/result/timestamps only: there is no inferred outcome or duration. Effect events identify the Player, Effect/type, optional explicit Concept target and manual/Rule source. Conditions can compare only the closed typed numeric/text subjects defined for their trigger; missing and cross-trigger fields fail closed.

The new bounded actions are `award_skill_xp` (Skill ID, nonzero signed delta and optional reason), `unlock_skill` (one Skill ID), `apply_effect` (registered type, bounded name/description, optional same-Player Concept target, positive intensity and optional expiry duration capped at one year), and `deactivate_effect` (one existing Effect ID). Ownership, lifecycle, availability policy, XP floors, expiry timing and type registry are checked by domain/application logic and rechecked by SQLite. Effects created by Rules use application IDs/Clock and ordinary creation history; they do not receive a synthetic Session relationship. Rule deactivation is recorded as `rule_deactivated` with structured source, distinct from Player `manually_deactivated` and derived `expired`. Scheduled or already-expired Effects cannot be explicitly deactivated. Expiry remains read-derived and never triggers a Rule.

All root commands, Rule follow-ons, Transactions, availability/effect/session history, state updates and successful Rule audits use the existing single bounded `WorldStore::apply_rule_chain` transaction. Failure in a later action rolls back the root and all planned operations; failure/guard audit follows the existing separate failure-audit policy. Rule order stays priority-descending then stable ID; generated facts use deterministic FIFO processing. The depth (8), action (32), evaluation (64), per-rule action (16), condition depth/node and repeated `(Rule ID, canonical event payload)` protections remain unchanged. There is no timer, scheduler, daemon, time-triggered Rule, synthetic activity, executable payload, arbitrary live-state query, XP-to-level conversion or universal prerequisite graph.

Migration 16 adds explicit Skill availability/control defaults (`available`/`manual`), Effect deactivation source backfill (`manual` for historical explicit off records), append-only Skill history, and the new closed Rule/Effect-history allow-list. It also losslessly rebuilds the shared Transaction table so `applied_amount` is valid for Player XP and Skill XP, preserving prior rows, IDs, capture times, indexes, search/Concept references and guards. Existing Skills remain available and Player-controlled; prior Effect deactivations remain manual; older Rule definitions/audits and Effect history rows remain readable. Fresh schema and schema-15 upgrade, integrity, and downgrade protection are covered by persistence tests. See the [Phase 10 report](PHASE-10-REPORT.md) for verification and compatibility evidence.

### Phase 11 — explicit Player-owned Tags

**Concepts describe meaning; Tags describe organization.** A Tag is a Player-authored canonical label with stable identity, unique normalized name within that Player, optional description, timestamps, and `active`/`archived`/`trashed` lifecycle. Tags are not Concepts, statuses, types, permissions, rules, or inferred classifications; no global Tag vocabulary is seeded. Renaming preserves the Tag ID and its relationships. Archived/trash lifecycle remains independent of both the target records and their assignments; restoring the Tag preserves those facts.

The closed target kinds are Quest, Stage, Branch, Session, Skill Tree, Skill, Concept, Effect, Narrative Entry, and Player-authored Comment. Transactions, revisions, snapshots, Rule audits, Workspace configuration, and other immutable facts are excluded. Each explicit `tag_relationships` fact records Player, Tag, target kind/ID, `added_at`, and optional `removed_at`. Removal retains the fact; reattachment creates a fresh relationship ID. Tag lifecycle changes do not synthesize assignment history. Application ownership checks and SQLite composite foreign keys/triggers enforce that Tag and target belong to the same Player and that new assignments use an active Tag.

Tag usage is a derived count of active relationship facts, scoped to the Tag's Player; no counter is cached. Name/description search is distinct from record filtering. Global Explorer's bounded any/all Tag filter uses indexed relational membership in SQLite, considers current (`removed_at IS NULL`) assignments, and retains ordinary Player, text, kind, status, lifecycle, visibility, sorting and paging semantics. Tag data is not copied into FTS documents. The contextual Tags section assigns, removes and navigates explicit relationships without changing the target's meaning, status, lifecycle, visibility, progression, history or canonical identity.

Only semantically supported Workspace panel sources expose Tag filters. The persisted panel stores at most 20 local Tag IDs plus `any`/`all`, and the application validates active same-Player ownership on save/import; unsupported sources (including Timeline) reject them. Workspace transfer v4 exports `{key, name}` Tag descriptors, never local row IDs. The immutable stable transfer key resolves exact matches; name-only candidates are suggestions requiring explicit Player choice, and unresolved Tags leave a filter neutral with visible feedback. Strict validated v1/v2/v3 imports remain supported.

Tags are excluded from the Timeline: a current Tag assignment can change after an event, so projecting current Tag state onto historical events would misstate tag-at-event-time semantics. Assignment history remains independently inspectable in Tag/Explorer surfaces; no Tag-at-event snapshots, Timeline backfill, automatic Tags, Tag Rule actions, hierarchy, inheritance, aliases, color semantics, or Concept inference are introduced. Migration 17 adds only the Tag tables/indexes/triggers, extends the shared recoverable lifecycle vocabulary, and adds bounded Workspace filter columns; fresh-schema, schema-16 upgrade, same-world, history, Search, transfer, and UI coverage are part of Phase 11 verification. See the [Phase 11 report](PHASE-11-REPORT.md).
