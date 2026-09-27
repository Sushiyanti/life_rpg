# Life RPG — Architecture

This document records *why* the system is shaped the way it is. The README
describes how to use it; this describes the reasoning, the boundaries, and the
decisions that will matter in later phases.

For the product-specific vocabulary and current invariants, see the [Domain
Design Codex](DOMAIN-DESIGN-CODEX.md), [Phase 2.1 report](PHASE-2.1-REPORT.md),
the [Phase 3 report](PHASE-3-REPORT.md), [Phase 3.5 report](PHASE-3.5-REPORT.md),
[Phase 3.6 report](PHASE-3.6-REPORT.md), and [Phase 4 report](PHASE-4-REPORT.md).

---

## 1. The seven responsibilities

The brief separates seven concerns. Each is a question, and exactly one layer is
allowed to answer it:

| # | Responsibility | Question | Phase 1 implementation |
|---|---|---|---|
| 1 | **Domain** | What exists in the world? | `crates/lr-domain` — value objects, error vocabulary |
| 2 | **State** | What condition is it in? | *(Phase 2)* — cached fields on aggregates |
| 3 | **History** | What happened in the past? | *(Phase 2)* — append-only ledger; Phase 1 shows the pattern via `health_probe` |
| 4 | **Rules** | How does the world change? | `lr-application` — closed declarative event/condition/action interpreter |
| 5 | **Application** | What commands/queries operate on the world? | `crates/lr-application` — ports + `HealthService` |
| 6 | **Presentation** | How should a thing look? | `src/presentation/spec.ts` — controlled style schema |
| 7 | **UI state / Workspace** | How does the user want the interface arranged? | `src/App.tsx`, `src/features/world/WorldWorkspace.tsx` — route, panel, and contextual preferences |
| — | **Persistence** | How is everything saved? | `crates/lr-persistence` — SQLite adapter |

Two of these deserve emphasis.

### Current state is not history

The brief is explicit, and the schema honours it:

- **Current state** — `Player.total_xp = 4200`. A cache. Fast reads, and
  reconstructible.
- **History** — `transactions: +25 XP (quest "Ship the parser"), -10 XP (missed
  a daily)`. Append-only, authoritative, never rewritten.
- **Snapshots** — `player_state_snapshot(player, date, level, xp, stats)`,
  immutable after creation, one normal snapshot per player per day.

These must not collapse into one mechanism, because doing so destroys the thing
the application is for: being able to ask "what did last month look like?" and
"how did I get here?" separately.

Phase 1 proves the pattern in miniature. `health_probe` is append-only — rows are
inserted and never updated — while `RoundTripProof.probe_rows` is a *derived
count*. The status screen shows both, so the distinction is visible on screen,
not just in a document.

### The domain must not depend on the UI

This is enforced by build configuration rather than discipline:

```toml
# crates/lr-domain/Cargo.toml — note what is absent
[dependencies]
thiserror.workspace = true
chrono.workspace = true
```

No `tauri`, no `rusqlite`, and no wire-format serialization derives. The domain
uses a small `serde_json` dependency only to validate the explicitly versioned
snapshot JSON value. If a developer tries to add a UI concern to the domain,
they must first edit the manifest and justify it in review. `cargo test -p
lr-domain` still compiles with zero infrastructure, which is the practical
payoff.

---

## 2. Layer boundaries and their contracts

### Domain → nothing

Pure functions, in → out. No I/O. No clock reads (time enters as
`Iso8601Timestamp`, which is a *value*, not a *call*).

Phase 1 ships three value objects and an error type, chosen because every later
aggregate needs them:

| Type | Why it must exist before aggregates |
|---|---|
| `EntityId` | Opaque, validated identity. Stored as `TEXT`, used as a React key, and neither side may assume its shape — that is what keeps storage swappable. |
| `Iso8601Timestamp` | Normalized, validated, sortable. Without it, every aggregate invents its own date handling and the transaction ledger becomes unfilterable. |
| `SchemaVersion` | `Ord`, so the downgrade guard (`is_behind`) is a total order rather than an integer comparison scattered through the code. |

`DomainError` deliberately has no infrastructure variants. "Database is locked"
is not a rule violation; it is a storage failure. Keeping them apart is what
lets rules be unit-tested with no I/O anywhere in sight.

### Application → Domain only

Written against **ports** (traits), never adapters:

```rust
pub trait HealthStore: Send + Sync {
    fn diagnostics(&self) -> Result<StoreDiagnostics, StorageError>;
    fn schema_report(&self) -> Result<SchemaReport, StorageError>;
    fn verify_round_trip(&self, token: &str, written_at: &str)
        -> Result<RoundTripProof, StorageError>;
}

pub trait Clock: Send + Sync {
    fn now_rfc3339(&self) -> String;
    fn now_unix_nanos(&self) -> u128;
}
```

Note what is absent: no `Connection`, no `Row`, no SQL, no path. The adapter is
free to be SQLite today and something else later without a line changing in this
crate.

The `Clock` port exists for one concrete reason: **determinism**. The health
service derives its probe token from the injected clock, so tests assert the
exact token `probe-deadbeef-0`. A service that called `Utc::now()` inline could
not be tested that way.

**`HealthService::run()` never fails.** It returns a report whose `status` is
`ok` / `degraded` / `failed` and whose `problems` vector carries the details.
A status screen that propagates an error is useless precisely when it is needed,
so failure is modelled as *data*, not as control flow. Storage being down is
still a successful call.

### Persistence → Application ports

The only crate that knows SQLite exists. Two obligations:

1. **Translate errors at the edge.** `rusqlite::Error` → `PersistenceError` (keeps
   SQL context for logs) → `StorageError` (three variants the application can
   reason about). A `rusqlite` type never escapes this crate.
2. **Degrade, never panic.** `SqliteHealthStore` is an enum:

   ```rust
   enum State {
       Ready { conn: Mutex<Connection>, location: Option<PathBuf> },
       Unavailable { reason: String },
   }
   ```

   Why an enum instead of `Result<Connection, _>`? Because a constructor that
   returns `Err` forces the UI to handle a bootstrap failure it cannot even
   display. The desktop shell must always open a window; the status screen is
   *how* the user learns storage is broken. `Unavailable` makes that state
   representable and reportable.

   A test opens a database at `/proc/self/mem/…` to prove an unwritable path
   produces a graceful `Unavailable`, not a panic.

### Shell → Application + Persistence

`src-tauri` is the composition root: the only place the layers meet.

```rust
pub fn bootstrap(app_data_dir: &Path, now: &str) -> AppState {
    AppState::new(HealthService::new(
        SqliteHealthStore::open_file(world_db_path(app_data_dir), now),
        SystemClock,
    ))
}
```

Commands are thin adapters: take `State`, call one use case, convert to a DTO. If
a command grows an `if` that encodes a game rule, that rule is in the wrong layer.

### Contract → Application (one-way)

`lr-contracts` mirrors application results as serializable DTOs. Two rules:

- **Serialization never touches the domain.** `HealthReport` has no `serde`
  derive; only its DTO mirror does. Adding a field to the core cannot silently
  change the wire format.
- **camelCase on the wire**, so TypeScript stays idiomatic without a mapping
  layer.

---

## 3. Why Tauri v2 and in-process IPC

Requirements: one user, offline-first, no LAN, no remote clients. Tauri v2's
`#[tauri::command]` bridge gives typed request/response over an **in-process
channel**.

Rejected alternatives:

| Option | Why not |
|---|---|
| Electron | Ships a whole Chromium runtime. Tauri uses the OS webview, so the binary is an order of magnitude smaller and the core is Rust, not Node. |
| Local HTTP server (FastAPI/Axum/Express) | Opens a port, needs CORS and auth decisions, has a startup race, and adds a serialization hop — for zero benefit, since the only client is in the same process tree. Explicitly ruled out by the requirements. |
| Qt / GTK native widgets | Fast, but the presentation layer needs to be dynamic and data-driven (variants, densities, workspaces). A DOM with CSS custom properties is a far better fit for a *style interpreter* than a native widget tree. |
| `tauri-plugin-sql` | Lets the frontend write SQL. That inverts the layering: the UI would become the source of truth for game state. |

---

## 4. Testing strategy

Tests are placed where a regression would be *invisible*:

| Suite | Location | What it protects |
|---|---|---|
| Domain value objects | `crates/lr-domain/src/value.rs` | Identity/timestamp validation, version ordering |
| Health use case | `crates/lr-application/src/services/health.rs` | Verdict logic against a fake store: healthy, behind-schema, too-new-schema, mismatched read-back, unreachable, deterministic tokens |
| Migration runner | `crates/lr-persistence/src/migrations.rs` | Fresh migrate, idempotent re-run, partial resume, downgrade refusal, table creation, type extensibility, uniqueness |
| SQLite adapter | `crates/lr-persistence/src/sqlite_store.rs` | Round trip, durability across reopen, WAL + FK pragmas, in-memory vs file, unwritable path, full seam through the service |
| Composition root | `src-tauri/src/lib.rs`, `state.rs` | Path resolution, real bootstrap, fallback behaviour, durable restarts |
| Contract drift | `crates/lr-contracts/src/lib.rs` | The exact JSON key set the TypeScript mirror depends on |
| IPC boundary | `src/test/ipc.test.ts` | Command names match the Rust registrations; every failure normalizes to one shape |
| Presentation interpreter | `src/test/presentation-spec.test.ts` | Tokens map to closed values; nothing outside the token set can be emitted |
| Rule interpreter + SQLite seam | `crates/lr-application/src/rules.rs`, `services/rule_engine.rs`, `crates/lr-persistence/src/world_store.rs` | Closed variants, deterministic ordering, multi-event chaining, bounds, loop guards, rollback, and durable audit |
| Rule IPC/UI | `crates/lr-contracts`, `src/test/world-ipc.test.ts`, `RulePanel.tsx` | CamelCase DTO parity and typed command payloads |
| Status screen | `src/test/StatusScreen.test.tsx` | Loading, healthy, pending migration, failed, unreachable — all render |

Two of these are worth singling out.

**The contract-drift test.** `contract_drift_matches_the_typescript_mirror`
asserts the exact top-level JSON keys and the exact keys of every nested object.
`src/domain/health.ts` mirrors them by hand. If either side changes alone, the
test fails. That is the mechanism that keeps a hand-written boundary honest
without a codegen step.

**The unwritable-path test.** Asserting that a *failure* is graceful is easy to
skip and easy to get wrong. `open_file("/proc/self/mem/world.sqlite3")` must
produce `Unavailable`, and it does.

---

## 5. Decisions made without stopping to ask

The brief permits — in fact prefers — a sensible documented default over
blocking on a question. These were chosen during Phase 1:

| Decision | Choice | Rationale |
|---|---|---|
| Migration tracking | Ledger table (not just `user_version`) | Records name + timestamp — real history, which is the point of the app |
| `type_definitions` seed location | Inside migration 0003 | A fresh world must be usable immediately; seeding is schema, not user data |
| DB filename | `life-rpg.sqlite3` | Descriptive; avoids the generic `app.db` that makes backups ambiguous |
| Fallback on storage failure | In-memory world + startup warning | A blank window communicates nothing; a degraded-but-open window can explain itself |
| Presentation token format | CSS custom properties (`var(--accent-gold)`) | Lets a future theme record retarget the whole UI without touching components |
| Frontend state management | `useState` + a use-case hook, no library | One screen does not justify a store; the hook shape is what later phases will scale |
| Router | None in Phase 1 | Adding routes for one screen is speculative structure |
| Contract generation | Hand-mirrored + drift test | Codegen adds a build step and a failure mode; a drift test gets the same safety for less machinery |

---

## 6. Phase 3 declarative rule execution

### Vocabulary and trigger semantics

The event is the trusted trigger source, distinct from a scheduled/state-only query. Version 1 emits `quest_completed`, `player_xp_changed`, `stat_changed`, and (Phase 3.5) `concept_progress_changed`; each Rust event variant has structured semantic fields. The event is serialized only for audit. A Rule definition has a schema version (currently 1), event-kind trigger, typed condition tree, and ordered closed action list. No rule column or payload is interpreted as executable code.

Conditions support event-kind matching, numeric/text comparisons, `ALL`, `ANY`, and `NOT`. Numeric subjects are selected from event fields; asking an event for a subject it does not carry returns no match. This version does not query arbitrary current entity state or schedule evaluations. Actions support XP award/removal, valid Quest completion, setting/modifying a data-defined Player Stat, and the constrained `set_concept_progress` action. Concept actions check same-Player ownership, active data-defined track semantics/bounds, and finite values. Other action families remain deferred until their behavior is designed.

### Ordering and atomicity

For a given event, enabled rules are ordered by priority descending, then stable Rule ID ascending. Rules are evaluated in that order. Ordered actions append follow-on typed events, processed FIFO after the current event's rule set. Iteration does not depend on SQL row order, frontend ordering, map iteration, or randomness. IDs/timestamps are supplied by application services, with event source order retained.

The evaluator plans root and derived changes using validated domain values (`Player::apply_xp`, `Quest::complete`, `PlayerStat::new`, and Concept Progress Track transitions). It sends typed `RuleOperation`s and successful/condition-failed audit records through `WorldStore::apply_rule_chain`. The SQLite adapter rechecks expected prior values and ownership, inserts related XP Transactions and immutable Concept progress history, applies all Player/Quest/Stat/Concept state, and appends audit records in one transaction. Failure in a later action therefore commits none of the initiating state change or earlier planned actions. When the batch itself fails, the adapter transaction rolls back; the application attempts a separate failure-audit insert. No SQL or game-rule branches are placed in Tauri commands or React.

The execution audit is debugging history, not a replacement for Transactions or daily snapshots. It records Rule ID, chain ID, typed event kind/payload, condition result, configured actions, status/error, depth, and time; SQLite rejects updates/deletes to those rows.

### Safety boundaries and known omissions

Conditions are nested at most 8 levels, each logical group has at most 16 children and the full tree at most 128 nodes; a Rule has at most 16 actions, a chain at most 32 actions and 64 evaluations, and maximum accepted chain depth is 8. The evaluator also rejects a repeated `(Rule ID, canonical event payload)` pair. A guard abort records its reason and leaves root/domain writes unapplied. These limits also bound malformed or accidental trigger cycles.

There is no evaluator for arbitrary expressions, no script/action plug-in mechanism, no scheduler/daemon, and no distributed event bus. Conditions read only the typed event payload, not arbitrary live entity state. No skill XP/unlock, Effect activation, start/abandon/fail Quest action, or narrative action is exposed yet. Those omissions are deliberate; the Phase 4 UI consumes only supported operations and does not add gameplay rules.

---

## 7. Phase 3.5 domain refinement

### Concepts stay distinct from activities and records

A `Concept` is a Player-owned meaningful subject (for example a topic, project, place, or life area), not a universal wrapper. Quest, Skill, Skill Tree, Narrative Entry, Comment, Transaction, and Effect remain distinct typed entities. A closed cross-entity link table and the explicit Effect target variant connect only supported entity kinds to a Concept. Concept-to-Concept relationships are directed, typed, data-defined records with same-Player guards; this is not a graph inference engine.

Concept types live in the existing `type_definitions` registry under the `concept` namespace. Relationship codes and Progress Track definitions are data-defined. Progress carries explicit semantics (`numeric`, `percentage`, `experience`, `level`, `mastery`) with finite/bounds checks; it does not introduce a universal XP property or imply conversions between measures.

### Time is not a daily activity record

`created_at` means initial record creation, `updated_at` the last mutation of mutable state, `occurred_at` a real-world event time when meaningful, and `captured_at` the installation's observation/recording time. New Concept progress entries store both occurred and captured values. New Transactions store captured time when known; legacy Transactions retain unknown capture time rather than receiving a backfilled guess. Existing Player/Skill snapshots keep their established `created_at` column as their precise capture time; Concept snapshots use `captured_at`.

Daily snapshots are immutable, unique per entity/date observations, not summaries of everything that happened during the day. Multiple chronological records may occur on one date. Missing dates are gaps in recorded data, not zero or negative activity, and are not auto-filled. Snapshot rows are not rewritten by migration.

### Search is a typed query over a compact text projection

`lr-application::SearchQuery` and `SearchHit` provide filtering/sorting/pagination without teaching the frontend SQL. SQLite FTS5 indexes only bounded identity/type/status/time/name/body text; triggers and migration backfill synchronize it. Full aggregate rows, metadata JSON, and rule payloads are not copied into a second database. Query filters include entity kind, Player, Concept association, type/status/active, time range, and bounded text. Effect active state is evaluated against the query instant; explicit Concept links, relationships, and targets drive related-Concept filtering. The first World Explorer UI is implemented in Phase 4; tags, arbitrary JSON search, ranking tuning, and a timeline remain deferred.

`ConceptService::detail` composes current progress, Concept relationships, explicit related entity references, progress history, and snapshots into a backend read model. It is data for specialized views, not a persisted page entity.

---

## 8. Phase 3.6 world semantics

### Player authority over progression

Player and Skill levels are explicit, manually controlled state. XP remains an independent accumulated ledger; no formula converts it to a level, and a change in XP cannot rewrite the Player's authored level or label. Concept progress definitions declare whether a track is manual or rule-authorized. Automatic rules cannot mutate a manual track. Where a derived proposal is useful, it is persisted as a suggestion and has no effect until the Player explicitly accepts it; acceptance and the progress-history/rule chain commit atomically.

### Structured Quest activity and real sessions

Stages and Branches are optional typed child records, not a universal requirement or a generic tree framework. Each Stage belongs to a Quest and each Branch to a Stage under that same Quest. Quest Sessions record actual started/ended periods and context. They do not synthesize daily activity from calendars, snapshots, durations, or gaps. Existing snapshot semantics remain: a missing date means no observation was recorded.

### Recovery and presentation are separate concerns

SQLite before-image revision triggers capture mutable aggregate edits. Recovery uses a new mutation with its own revision, not an overwrite of history. Active/archived/trashed lifecycle is soft and reversible; trashed records remain stored. Contextual visibility is a per-player presentation preference and is never a domain state, deletion, or permission. Search applies it only when a caller requests a specific context, while lifecycle filters remain separate.

Phase 3.6 exposes the use cases through typed contracts and thin Tauri adapters. It establishes data and behavior used by the Phase 4 presentation layer below.

## 9. Phase 4 presentation and workspace

The React shell is a presentation client, not a second domain. `AppShell` owns route navigation and active Player selection; `App.tsx` restores the selected route and world from local UI storage and coordinates typed `CoreClient` reads/writes. Feature screens use those DTOs and command methods only—React has no SQLite path and invokes Tauri only inside `src/domain/ipc.ts`.

The dashboard is a structured, allow-listed set of panel definitions (source, title, context, filter, limit, order, visibility, pin, collapse, density and card/row variant). It does not evaluate persisted code or HTML. Workspace composition and selected route are UI-local state; entity visibility is persisted per Player and context through Phase 3.6 preferences. This distinction avoids pretending a dashboard panel is a world entity merely to reuse an entity-preference table. Presentation changes therefore do not modify canonical entity records.

World Explorer is an application-query client for the existing FTS5 search use case. Kind/type, Concept relationship, active state, time range, sorting, bounded paging, and separate hidden/archive/trash inclusion remain explicit filters. Reusable detail surfaces display typed entity metadata, Concept relationships and associations, attached content, comments, lifecycle, and supported revisions. Restore is an explicit application command and adds history; there is no permanent purge UI.

Major feature routes cover Player, Quests with optional Stage/Branch and real Session activity, Skills and Skill Trees, Concepts and their typed relationships/progress tracks, Effects, Journal content, Search/Explorer, and History/Recovery. Simple Quests remain valid without hierarchy. Session start/finish calls are player-initiated; no background process or missing-day inference is introduced. The browser-only Vite preview has no Tauri bridge and intentionally displays a connection warning; the shipped target is the local desktop app.

The Phase 4.1 corrective pass preserves this UI architecture while closing three integrity gaps: Session creation is offered only from an explicit Quest/Stage/Branch/Skill context; the visibility-specific application/store operation updates only `is_visible` and leaves order, pin, collapse, variant, and density untouched; and Quest descriptions travel through the typed CoreClient → Tauri → WorldService path into the existing Quest persistence fields. These are correctness fixes to the Phase 4 presentation boundary, not new world semantics or a redesign. The [Phase 4 report](PHASE-4-REPORT.md) records the regression coverage and focused empty-state refinement.

The first-generation scope intentionally defers unrestricted workspace authoring, arbitrary custom panels, rich text/markdown editing, global graph traversal, complete per-kind bespoke detail pages, and permanent deletion. See [Phase 4 report](PHASE-4-REPORT.md) for the detailed UI inventory and verification.

## 10. Roadmap

| Phase | Scope | Status |
|---|---|---|
| **1** | Foundation: workspace, shell, SQLite, migrations, layering, health screen, tests | **complete** |
| **2** | Persistent world aggregates, type vocabulary, transactions and snapshots | **complete** |
| **2.1** | Canonical daily state history, dynamic stats, XP policy, lifecycle and hierarchy hardening | **complete in `phase-2.1`** |
| **3** | Typed, declarative event/condition/action engine, bounded chains, rule audit and atomic SQLite execution | **complete in `phase-3`** |
| **3.5** | Typed Concepts, relationships, progress tracks/history, temporal clarity, and global search foundation | **complete in `phase-3.5`** |
| **3.6** | Manual progression authority, Quest Stages/Branches, real Sessions, recoverable revisions/trash, contextual visibility, and suggestions | **complete in `phase-3.6`** |
| **4** | Dynamic presentation, workspace/UI-state persistence, specialized views and layout | **complete in `phase-4-ui`; correctness hardening in `phase-4.1-ui-fixes`** |
| **5** | Player-owned persistent workspaces and safe portable transfer (Phases 5–5.2) | **complete in `phase-5.2-workspace-transfer-integrity`** |
| **6** | Player Hub, contextual Quest/Session interaction, Quick Capture, recorded activity, Concept/Explorer and workspace navigation | **complete in `phase-6-player-experience`** |

### Phase 2.1 integrity decisions

- The `WorldStore` application port is implemented by `lr-persistence::world_store`;
  SQLite remains absent from domain, application, contracts, and frontend layers.
- Data-defined Player stats avoid one source-code field and migration per stat.
  Core identity, values, bounds, and ownership remain structured columns.
- Player and Skill snapshots preserve explicitly selected, versioned canonical
  state and have a unique entity/date key. They are not aggregate serialization.
- XP events preserve requested and actually applied deltas separately; a zero
  floor is enforced in domain operations and SQLite. Related state/history writes
  are one transaction.
- Recursive SQLite triggers enforce owner/tree consistency and cycle prevention;
  FKs alone only prove that a referenced row exists.
- Quest lifecycle deliberately remains the Phase 2 set; pause/failure semantics
  await an intentional gameplay/rules design instead of accumulating unused states.


## 7. Persistent workspaces and declarative panel instances (Phases 5–5.2)

A workspace is **Player/world-owned presentation configuration**, not a world entity, quest, or account preference. Migration 9 introduced `workspaces` and `workspace_panels`; migration 10 rebuilds the panel table forward-only to retain existing rows while permitting repeated source types and storing bounded filter/layout settings. The application port and service enforce Player ownership; SQLite remains behind the `SemanticsStore` boundary. Tauri commands are DTO adapters, and the frontend uses a typed `CoreClient`.

A workspace can contain several independent panel instances for one source—for example, separate in-progress and completed Quest panels. Each panel stores a closed source identifier and typed title, presentation variant, density, lifecycle/type/Concept/recent-time filters, sort, result limit, position, one-or-two-column span, panel visibility, pinning and collapsed state. Domain validation cross-checks a source's allowed filters and variants; SQL adds field, range and foreign-key constraints. Search remains the query executor. No panel field is executable code, raw SQL, HTML, or an arbitrary predicate.

Panel membership/visibility is distinct from entity existence, lifecycle (active/archived/trashed), and per-entity contextual visibility. A panel may be hidden without hiding the underlying Quest; an entity hidden in the Dashboard context remains a world record and can be exposed separately through contextual preference/search controls. Workspace edits never call domain-mutation operations. Removing a panel deletes only that presentation instance; deleting a workspace removes its panels, but the final workspace of a Player is protected. Deleting the default workspace atomically assigns a replacement when possible.

The browser may remember the last-opened workspace ID as runtime selection only. Workspace and panel definitions, including the database default and every display option, are canonical in SQLite. Migration 10 preserves version 9 panel IDs and fields and intentionally drops the old `(workspace_id, panel_type)` uniqueness restriction. Forward-only migration 11 adds an immutable, unique, Concept-only transfer key; it does not add global identity or synchronization to other world entities.

No drag-and-drop/layout library was added: native draggable cards and explicit up/down buttons share persisted ordering, and bounded grid spans collapse at responsive breakpoints. Current transfer files are strict declarative version 2 JSON. Concept filters export a stable Concept-only key plus name/type; local IDs, Player IDs, timestamps and world records are excluded. The preview resolves an exact key within the destination Player automatically, offers same-type/name candidates only for explicit choice, and otherwise leaves the filter neutral with a warning. Version 1 input is migrated after validation while discarding its untrusted local Concept IDs; unknown fields and unsupported versions fail closed. One typed native import command validates the full request and atomically writes the workspace and panels, with Player ownership checked in both application and SQLite layers. See [Phase 5](PHASE-5-REPORT.md), [Phase 5.1](PHASE-5.1-REPORT.md), and [Phase 5.2](PHASE-5.2-REPORT.md) for the contract and verification.

## Phase 6 — Player-facing interaction over the existing world

The Player Hub composes already-persisted Player, Quest, Session, Effect, Concept-progress, Skill, Journal, snapshot and workspace records into a contextual starting point. It is a presentation/query surface, not a new world aggregate. Hub rows and persistent workspace panels navigate to the exact record through the Player-scoped Explorer; explicit return-to-origin state preserves the initiating view. Session detail can follow the Quest, Skill, and Concept IDs already recorded on that Session. Concept associations continue to use the existing typed, Player-owned association service rather than a general graph.

Quest transitions, real Session start/end timestamps and authored outcome/notes use the existing commands. Quick Capture writes an existing Narrative Entry through the application boundary and attaches only to a context the Player selected; a successfully saved entry is reported as saved even if a later refresh fails. Timeline events are derived only from persisted timestamps and real records. Duration appears only when valid start/end timestamps exist. Empty or missing days, levels, stats, activity and progress are never inferred or fabricated. Session result/status and recoverable record lifecycle remain separate concepts.

Workspace panels remain canonical Player-owned configuration in SQLite and unchanged by Hub presentation or navigation. A navigation or visibility-only operation does not mutate domain truth; the existing, explicit presentation-preference command remains separate. No backend/domain behavior or external UI library was introduced in Phase 6. See [Phase 6 report](PHASE-6-REPORT.md) for detail and validation.
