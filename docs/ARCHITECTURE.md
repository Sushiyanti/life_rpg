# Life RPG — Architecture

This document records *why* the system is shaped the way it is. The README
describes how to use it; this describes the reasoning, the boundaries, and the
decisions that will matter in later phases.

For the product-specific vocabulary and current invariants, see the [Domain
Design Codex](DOMAIN-DESIGN-CODEX.md), [Phase 2.1 report](PHASE-2.1-REPORT.md),
the [Phase 3 report](PHASE-3-REPORT.md), [Phase 3.5 report](PHASE-3.5-REPORT.md),
[Phase 3.6 report](PHASE-3.6-REPORT.md), [Phase 4 report](PHASE-4-REPORT.md), [Phase 9 report](PHASE-9-REPORT.md), [Phase 10 report](PHASE-10-REPORT.md), and [Phase 11 report](PHASE-11-REPORT.md).

---

## 1. The seven responsibilities

The brief separates seven concerns. Each is a question, and exactly one layer is
allowed to answer it:

| # | Responsibility | Question | Current implementation |
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
       Unavailable { reason: String, location: Option<PathBuf> },
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

The event is the trusted trigger source, distinct from a scheduled/state-only query. Phase 3 introduced `quest_completed`, `player_xp_changed`, `stat_changed`, and (Phase 3.5) `concept_progress_changed`; Phase 10 extends the same closed, typed vocabulary as described below. The event is serialized only for audit. A Rule definition has a schema version (currently 1), event-kind trigger, typed condition tree, and ordered closed action list. No rule column or payload is interpreted as executable code.

Conditions support event-kind matching, numeric/text comparisons, `ALL`, `ANY`, and `NOT`. Numeric/text subjects are selected from event fields; a mismatched or absent field fails closed. Rules do not query arbitrary current entity state or schedule evaluations. Phase 3 actions include Player XP, Quest completion, Player Stats and constrained Concept progress; Phase 10 adds bounded Skill and Effect actions.

### Ordering and atomicity

For a given event, enabled rules are ordered by priority descending, then stable Rule ID ascending. Rules are evaluated in that order. Ordered actions append follow-on typed events, processed FIFO after the current event's rule set. Iteration does not depend on SQL row order, frontend ordering, map iteration, or randomness. IDs/timestamps are supplied by application services, with event source order retained.

The evaluator plans root and derived changes using validated domain values. It sends typed `RuleOperation`s and audit records through `WorldStore::apply_rule_chain`. SQLite rechecks expected prior values and ownership, inserts related Transactions and append-only history, applies all state and appends audit records in one transaction. Failure in a later action therefore commits none of the initiating state change or earlier planned actions. When the batch itself fails, the transaction rolls back; the application attempts a separate failure-audit insert. No SQL or game-rule branches are placed in Tauri commands or React.

The execution audit is debugging history, not a replacement for Transactions or daily snapshots. It records Rule ID, chain ID, typed event kind/payload, condition result, configured actions, status/error, depth, and time; SQLite rejects updates/deletes to those rows.

### Safety boundaries and known omissions

Conditions are nested at most 8 levels, each logical group has at most 16 children and the full tree at most 128 nodes; a Rule has at most 16 actions, a chain at most 32 actions and 64 evaluations, and maximum accepted chain depth is 8. The evaluator also rejects a repeated `(Rule ID, canonical event payload)` pair. A guard abort records its reason and leaves root/domain writes unapplied. These limits also bound malformed or accidental trigger cycles.

There is no evaluator for arbitrary expressions, no script/action plug-in mechanism, no scheduler/daemon, and no distributed event bus. Conditions read only typed event payloads, not arbitrary live state. Phase 10 adds only the documented Skill, explicit Session, and Effect operations; other action families remain deferred until their behavior is designed.

---

## 7. Phase 3.5 domain refinement

### Concepts stay distinct from activities and records

A `Concept` is a Player-owned meaningful subject (for example a topic, project, place, or life area), not a universal wrapper. Quest, Skill, Skill Tree, Narrative Entry, Comment, Transaction, and Effect remain distinct typed entities. A closed cross-entity link table and the explicit Effect target variant connect only supported entity kinds to a Concept. Concept-to-Concept relationships are directed, typed, data-defined records with same-Player guards; this is not a graph inference engine.

Concept types live in the existing `type_definitions` registry under the `concept` namespace. Relationship codes and Progress Track definitions are data-defined. Progress carries explicit semantics (`numeric`, `percentage`, `experience`, `level`, `mastery`) with finite/bounds checks; it does not introduce a universal XP property or imply conversions between measures.

### Time is not a daily activity record

`created_at` means initial record creation, `updated_at` the last mutation of mutable state, `occurred_at` a real-world event time when meaningful, and `captured_at` the installation's observation/recording time. New Concept progress entries store both occurred and captured values. New Transactions store captured time when known; legacy Transactions retain unknown capture time rather than receiving a backfilled guess. Existing Player/Skill snapshots keep their established `created_at` column as their precise capture time; Concept snapshots use `captured_at`.

Daily snapshots are immutable, unique per entity/date observations, not summaries of everything that happened during the day. Multiple chronological records may occur on one date. Missing dates are gaps in recorded data, not zero or negative activity, and are not auto-filled. Snapshot rows are not rewritten by migration.

### Search is a typed query over a compact text projection

`lr-application::SearchQuery` and `SearchHit` provide filtering/sorting/pagination without teaching the frontend SQL. SQLite FTS5 indexes only bounded identity/type/status/time/name/body text; triggers and migration backfill synchronize it. Full aggregate rows, metadata JSON, and rule payloads are not copied into a second database. Query filters include entity kind, Player, Concept association, type/status/active, time range, and bounded text. Effect active state is evaluated against the query instant; explicit Concept links, relationships, and targets drive related-Concept filtering. The first World Explorer UI is implemented in Phase 4; Timeline is implemented in Phase 8 and Tags in Phase 11. Arbitrary JSON search and ranking tuning remain deferred.

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

World Explorer is an application-query client for the existing FTS5 search use case. Kind/type, Concept relationship, explicit Tag IDs with bounded any/all matching, active state, time range, sorting, bounded paging, and separate hidden/archive/trash inclusion remain explicit filters. Tag membership is relationally evaluated in SQLite from current assignments; Tag names/relationships are not copied into FTS documents. Reusable detail surfaces display typed entity metadata, Concept relationships and associations, Tags, attached content, comments, lifecycle, and supported revisions. Restore is an explicit application command and adds history; there is no permanent purge UI.

Major feature routes cover Player, Quests with optional Stage/Branch and real Session activity, Skills and Skill Trees, Concepts and their typed relationships/progress tracks, Effects, the Content Guidebook, Search/Explorer, and History/Recovery. Journal-style entries remain a valid Content kind, but the Guidebook is the current authored-content surface. Simple Quests remain valid without hierarchy. Session start/finish calls are player-initiated; no background process or missing-day inference is introduced. The browser-only Vite preview has no Tauri bridge and intentionally displays a connection warning; the shipped target is the local desktop app.

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
| **6.1 / 6.1.1** | Effect lifecycle correction, append-only Effect history, and explicit Session context | **complete in `phase-6.1.1-effect-lifecycle-final`** |
| **7** | Reusable authored Content/Guidance, timestamped closed-world relationships, Guidebook UI, and content-aware search | **complete in `phase-7-content-guidance`** |
| **8** | Unified read-only, Player-scoped Timeline over persisted timestamp sources | **complete in `phase-8-timeline`** |
| **9** | Reusable declarative Timeline workspace panels, exact-source/date filters, and relationship context navigation | **implemented on `phase-9-workspaces-presentation`** |
| **10** | Explicit Skill XP/unlock and immediate Effect Rule actions, with bounded atomic chains and auditable history | **complete in `phase-10-gameplay-rules-progression`** |
| **11** | Reusable Player-owned Tags, explicit relationships, relational Search/Explorer filters, Workspace filters, and portable transfer | **complete in `phase-11-tags-organization`** |

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

A workspace can contain several independent panel instances for one source—for example, separate in-progress and completed Quest panels. Each panel stores a closed source identifier and typed title, presentation variant, density, lifecycle/type/Concept/Tag/recent-time filters, sort, result limit, position, one-or-two-column span, panel visibility, pinning and collapsed state. Tag IDs are local Player-owned references with `any`/`all` semantics and are available only on supported canonical panel sources. Domain/application ownership checks and SQL constraints reject invalid sources, unowned Tags, malformed bounds, and foreign Player data. Search remains the query executor. No panel field is executable code, raw SQL, HTML, or an arbitrary predicate.

Panel membership/visibility is distinct from entity existence, lifecycle (active/archived/trashed), and per-entity contextual visibility. A panel may be hidden without hiding the underlying Quest; an entity hidden in the Dashboard context remains a world record and can be exposed separately through contextual preference/search controls. Workspace edits never call domain-mutation operations. Removing a panel deletes only that presentation instance; deleting a workspace removes its panels, but the final workspace of a Player is protected. Deleting the default workspace atomically assigns a replacement when possible.

The browser may remember the last-opened workspace ID as runtime selection only. Workspace and panel definitions, including the database default and every display option, are canonical in SQLite. Migration 10 preserves version 9 panel IDs and fields and intentionally drops the old `(workspace_id, panel_type)` uniqueness restriction. Forward-only migration 11 adds an immutable, unique, Concept-only transfer key; migration 17 adds immutable Player-scoped Tag transfer keys without adding global identity or synchronization to world entities.

No drag-and-drop/layout library was added: native draggable cards and explicit up/down buttons share persisted ordering, and bounded grid spans collapse at responsive breakpoints. Strict declarative transfer v4 extends v3 with portable Tag references while preserving validated v1/v2/v3 upgrades. Concept filters export a stable Concept-only key plus name/type; Tag filters export an immutable Tag transfer key plus human-readable name; local row IDs, Player IDs, timestamps and world records are excluded. Exact stable keys resolve within the destination Player. Name-only Tag candidates are suggestions requiring explicit choice; unresolved Tag filters remain neutral with visible feedback. Exact Timeline record IDs are represented by non-identifying local-record descriptors and remain neutral until reselected. Unknown fields and unsupported versions fail closed. One typed native import command validates the full request and atomically writes the workspace and panels, with Player ownership checked in both application and SQLite layers. See [Phase 5](PHASE-5-REPORT.md), [Phase 5.1](PHASE-5.1-REPORT.md), [Phase 5.2](PHASE-5.2-REPORT.md), and the [Phase 11 report](PHASE-11-REPORT.md).

## Phase 6 — Player-facing interaction over the existing world

The Player Hub composes already-persisted Player, Quest, Session, Effect, Concept-progress, Skill, authored Content, snapshot and workspace records into a contextual starting point. It is a presentation/query surface, not a new world aggregate. Hub rows and persistent workspace panels navigate to the exact record through the Player-scoped Explorer; explicit return-to-origin state preserves the initiating view. Session detail can follow the Quest, Skill, and Concept IDs already recorded on that Session. Concept associations continue to use the existing typed, Player-owned association service rather than a general graph.

Quest transitions, real Session start/end timestamps and authored outcome/notes use the existing commands. Quick Capture writes an existing Narrative Entry through the application boundary and attaches only to a context the Player selected; a successfully saved entry is reported as saved even if a later refresh fails. Timeline events are derived only from persisted timestamps and real records. Duration appears only when valid start/end timestamps exist. Empty or missing days, levels, stats, activity and progress are never inferred or fabricated. Session result/status and recoverable record lifecycle remain separate concepts.

Workspace panels remain canonical Player-owned configuration in SQLite and unchanged by Hub presentation or navigation. A navigation or visibility-only operation does not mutate domain truth; the existing, explicit presentation-preference command remains separate. No backend/domain behavior or external UI library was introduced in Phase 6. See [Phase 6 report](PHASE-6-REPORT.md) for detail and validation.

## Phase 6.1 — Effect lifecycle, history and Session context

Effects retain their closed Player-or-Concept target boundary. `expires_at = NULL` means that no expiry was recorded: the Effect remains active until its explicit start/deactivation facts imply otherwise. A supplied expiry is a recorded timestamp; `scheduled`, `active`, `expired`, and `manually deactivated` are derived from the stored start, expiry and manual-deactivation timestamps at read/presentation time. Crossing an expiry never writes `deactivated_at`, emits a synthetic event, or requires a scheduler or polling process.

At the domain boundary, manual deactivation is rejected when the Effect's recorded expiry is at or before the operation time, including an exact tie. This prevents redundant manual-off history after expiry; the UI likewise offers the action only for derived-active Effects. Indefinite and future-expiring active Effects remain manually deactivatable, and scheduled Effects cannot be turned off before their start. Expired Effects remain editable: descriptive edits leave the expiry/lifecycle alone, while an explicit expiry edit (including moving it forward or clearing it) is recorded as `expiry_changed`. That deliberate authored change may make the derived state active again; it does not create an automatic-reactivation event.

Migration 12 adds Player-scoped `effect_history` and `session_effects` relations. History rows are append-only before/after facts for creation, details, expiry, manual deactivation, and Session-link changes; typed link snapshots include the Session, Effect, bounded role and removal timestamp. Effect create/update/deactivate plus their history and optional explicit Session relationship commit atomically through `SemanticsService` and `SemanticsStore`. Application validation and SQLite ownership triggers both reject cross-Player targets and Session links.

Typed Effect commands pass through `CoreClient` and Tauri adapters into application services; React does not write to SQLite. `EffectsScreen` supports Player/Concept targeting, optional expiry, edits with before/after history, active-only manual deactivation, and history inspection. Session details use an explicit relationship panel to link an existing Effect, create one in Session context, or remove only the relationship. Expiry or manual deactivation never removes historical Session relationships. No active Effect is attached merely because a Session exists. The local-first React → typed IPC → application → SQLite boundary remains unchanged; no external UI package, cloud dependency, daemon, background process, or generic event-sourcing framework was introduced. See [Phase 6.1.1 report](PHASE-6.1.1-REPORT.md) for the final correction and verification details.

## Phase 7 — reusable authored Content & Guidance

Phase 7 deliberately evolves `NarrativeEntry` instead of introducing a second Content aggregate or a generic Page/CMS system. `WorldService` owns typed creation, lookup, and revision-aware updates; `SemanticsService` owns explicit relationship attachment/removal and resolves every target against the active Player world before persistence. The existing `NarrativeEntry` before-image and FTS triggers remain the sole content-change synchronization mechanism—an edit produces a normal recoverable revision and updates the one canonical FTS document for that content record. There is never one search document per attachment.

Migration **13** rebuilds the legacy attachment table forward-only into `content_attachments` facts with stable IDs, data-defined roles, `sort_order`, `created_at`, `removed_at`, and active-only uniqueness. Its polymorphic target kind is closed to Player, Quest, Stage, Branch, Session, Skill, Skill Tree, Concept, and Effect. Application checks and SQLite triggers both enforce content ownership and target existence in the same Player world. Removal timestamps a fact; reattaching produces another fact. Target lifecycle, content lifecycle, and presentation visibility remain independent. If a legacy physical target deletion occurs, active relationships are marked removed rather than erased; normal lifecycle/trash operations do not alter links.

The React **Content Guidebook** is a plain-text authoring and inspection surface. Its typed `CoreClient` calls use Tauri DTOs only; it has no SQLite path. It supports browsing/filtering/searching typed content, editing author/source fields, opening existing lifecycle/recovery UI, inspecting relationship history, and attaching an existing canonical record to Player/Quest/Skill/Concept/Effect targets. World Explorer retains contextual attachment panels—including Stage, Branch, and Session—and supports typed content-target filtering. Rich text, automatic/inferred relationships, universal graph traversal, scheduling, and content-driven progression remain explicitly out of scope. See [Phase 7 report](PHASE-7-REPORT.md) for verification and compatibility details.

## Phase 8 — unified read-only Timeline

`TimelineService` is a dedicated application query over the `TimelineStore` port; it is separate from both global Explorer search and History & recovery. SQLite composes the existing Player-owned source tables into a bounded, typed page. Stable source IDs, canonical entity kind/ID, closed category/state codes, explicit primary/secondary timestamp meanings, optional exact Concept context, and deterministic tie ordering survive through Rust contracts, Tauri IPC and the React `CoreClient`. Migration 14 adds source-scoped timestamp indexes only; it creates no event table or copied timeline state.

The projection includes Session records (start as primary time, persisted end as optional detail), Transactions, Effect history, Content create/update timestamps, authored Comments, Concept-progress history, entity revisions, immutable snapshots, lifecycle audit facts, explicit Content relationship creation/removal, and meaningful persisted Player/Quest/Skill record timestamps. Categories preserve the difference between current records and historical facts; Content and relationship changes are separate; attached Concepts are not inferred from prose. The query applies Player ownership and optional time/category/entity/Concept filters in SQL before returning at most 200 rows, with validated bounded offsets and explicit newest/oldest ordering.

No current-state-only record is given a fabricated event time. The Timeline creates no app-open, elapsed-time, expiry, scheduled, daily, or missing-day event and performs no write, backfill, scheduler, or background polling. The Player Hub’s short activity list calls the same bounded query and filters only its existing contextual visibility choices; “View full timeline” opens the first-class route, while History & recovery remains distinct. Timeline source navigation delegates to the existing Player-scoped Explorer/detail route. See [Phase 8 report](PHASE-8-REPORT.md) for implementation and verification details.


## Phase 9 — Timeline as a declarative Workspace panel

The Timeline is now a reusable panel source in the Workspace registry and Builder. Each instance stores only presentation/query settings: optional closed category and entity-kind filters, an exact entity ID paired with its entity kind, inclusive `from`/`through` calendar dates, optional Concept context, ordering, and a bounded result limit. The panel invokes the same read-only Timeline query port as the full screen; it creates no snapshot, cached panel rows, copied event table, or alternate event vocabulary. Full-screen Timeline and Player Hub entry points remain available.

Migration **15** adds nullable Timeline filter columns to `workspace_panels`, with source-specific closed-value, range, and non-Timeline guard constraints. Existing rows and their IDs/options remain intact and receive neutral (`NULL`) Timeline filters. Exact identity is scoped to the selected Player by the Timeline query; domain validation requires a closed entity kind when an exact ID is supplied. Relationship-history rows carry the persisted Content attachment ID, Content title, role, created/removed timestamps and existing target identity, so the Timeline can expose navigation to both records without changing either.

Workspace transfer format **3** carries Timeline category, entity kind and inclusive date filters. Like other source-world-local entity IDs, an exact Timeline ID is never exported: it is represented by a non-identifying “specific local record” descriptor. Import previews call this out and leave that filter neutral until the player reselects a local record. Strict v1 and v2 files are validated and upgraded to v3; unknown versions, extra keys, unsupported source filters and invalid ranges fail closed. The application import remains one atomic declarative configuration write and imports no world records. See [Phase 9 report](PHASE-9-REPORT.md) for checks and native-smoke results.


## 11. Phase 10 — explicit gameplay Rules and progression

Phase 10 extends the existing interpreter; it does not add a second execution path. Its closed `RuleEvent`, `RuleCondition`, `RuleAction`, and `RuleOperation` types add Skill XP/unlock and explicit Session/Effect lifecycle operations. Rule conditions project only each typed event's declared values; unsupported or absent subjects evaluate false and invalid trigger/subject combinations fail validation. Serde rejects unknown fields and variants. The Rule authoring surface remains typed controls, not raw executable data.

Skill XP is a separate ledger resource (`skill_xp`) with a Skill source ID. Requested and applied signed amounts, before/after XP, reason/source, occurrence and capture values remain inspectable. XP floors at zero and is independent from manually authored Skill level, label, invested time and lifecycle. Skill availability is separate from lifecycle. A small `manual`/`rule_controlled` authority field fails closed: explicit Player lock/unlock always works and returns authority to manual, while a Rule unlock is permitted only for Rule-controlled Skills. Already-unlocked is a successful audited no-op with no repeated history or follow-on event. No hierarchy-derived prerequisite or automatic level behavior exists.

The new Rule actions are Skill XP adjustment, policy-checked Skill unlock, immediate Effect creation and actionable Effect deactivation. Effect creation uses the application clock and ordinary canonical identity/history, optional registered type/Concept target/intensity/expiry bounds, and no invented Session association. Manual and Rule deactivation write separate append-only Effect history and explicit source values. Expiration remains derived; it never writes, emits a Rule event or triggers a scheduler. Scheduled and expired Effects reject explicit deactivation. Session start/finish triggers are emitted only by actual Player commands and include only recorded values; optional result stays absent when not authored, and no duration/outcome is inferred.

`WorldStore::apply_rule_chain` remains the sole persistence boundary for the root operation, all planned Skill/Effect/Session/Player/Quest/Stat/Concept changes, ledger/history rows and successful execution audits. The SQLite adapter validates owner and expected prior state and writes the entire successful plan in one SQLite transaction; later failure leaves no partial root or derived state. Guard/failure audit is handled by the established post-rollback path. Deterministic Rule priority/ID order, FIFO events, depth 8, action 32, evaluation 64, per-rule action 16, bounded conditions and repeated Rule/event detection remain unchanged. A Skill-XP self-trigger regression proves depth abort and full state/ledger rollback.

Migration 16 is forward-only. It safely defaults historic Skills to available/manual and tags existing explicit Effect off records as manual, adds a narrow append-only Skill availability-history table, expands only closed Rule and Effect history values, and rebuilds the Transaction applied-delta constraint for Skill XP while preserving IDs, rows, FKs, search/Concept guards, and indexes. Tests cover fresh creation, schema-15 preservation and downgrade refusal. The existing Timeline query now projects Skill XP Transactions, Skill availability history and distinctly attributed Effect history; it remains a read-only composition of canonical records, with no copied event store.

The feature remains inside the existing local boundary: React uses typed `CoreClient` methods, Tauri commands adapt DTOs, application services own decisions and injected time, and SQLite is confined to `lr-persistence`. No scheduler, daemon, external integration, code evaluation, general prerequisite graph, synthetic activity or Phase 11 feature is part of this change. See [Phase 10 report](PHASE-10-REPORT.md) for exact verification and native smoke results.

## 12. Phase 11 — reusable Tags and world organization

Tags are explicit Player-owned organizational labels; Concepts continue to represent semantic meaning. A Tag has a structured identity, immutable transfer key, normalized Player-unique name, optional description, lifecycle, and timestamps. Renaming keeps identity; archiving/trashing/restoring the Tag never mutates its targets or assignment facts. Usage is a derived count of active Tag relationships, not a cached value.

The closed assignment target set is Quest, Quest Stage, Quest Branch, Session, Skill Tree, Skill, Concept, Effect, Narrative Entry, and authored Comment. It deliberately excludes Workspace configuration, Transactions, Revisions, Snapshots, Rule audits, and other immutable history. Each relationship records an explicit Player assignment with stable identity, `added_at` and optional `removed_at`; removal is historical, and reattachment creates another fact. Lifecycle/visibility/status/progression axes remain independent. Application validation and SQLite triggers/composite FKs enforce Tag and target ownership in the same Player world; a Tag must be active for a new assignment. There are no inferred, generated, aliased, nested, hierarchical, or Rule-authored Tags.

Tag Manager searches Tag name/description separately from record Search, displays derived usage, inspects relationship history, and navigates to exact target records. Explorer and supported canonical Workspace panels filter by bounded Tag IDs using any/all current-relationship membership in the SQLite query, preserving Player, lifecycle, contextual visibility, text, sort and pagination semantics. Tag names are not synchronized into FTS. Timeline intentionally has no generic Tag filter and excludes Tag lifecycle/assignment timestamps: mutable current organization cannot be projected onto a past event without explicit tag-at-event-time semantics. The Tag relationship history remains inspectable outside Timeline.

Migration **17** creates structured `tags` and `tag_relationships`, indexes Player/name and current/history relationship paths, enforces same-world target ownership and immutable history, extends the shared recoverable lifecycle vocabulary, and adds bounded Tag ID/match-mode columns to Workspace panels. Workspace transfer **v4** carries only `{key, name}` descriptors, never local Tag IDs; v1/v2/v3 are strictly validated and upgraded. Exact keys auto-resolve within the destination Player; similar names are only explicit choices, and unresolved filters remain neutral with feedback. No Tag row or target record is imported. See the [Phase 11 report](PHASE-11-REPORT.md) for migration upgrade coverage, verification totals, native smoke results, and known limitations.


## 13. Phase 12 — Release Hardening & Data Safety

**Current roadmap state:** Phases 1–11 are complete. Phase 12 hardens storage and recovery without introducing a gameplay system or a schema migration. Phase 13 is real-world release-candidate validation; the project is not yet v1.0 and has no release-date commitment.

### Local world storage and startup

The authoritative local world remains the single SQLite database in the Life RPG application-data directory. The application keeps SQLite foreign-key enforcement, a five-second busy timeout, WAL journaling, and `synchronous=NORMAL`; WAL is selected only after an existing file has been inspected and any required pre-migration checkpoint has been verified. The durability choice is appropriate to a single-user desktop workload: committed WAL state is included in snapshots, while SQLite performs recovery after ordinary process interruption. Filesystem loss, a full disk, or hardware failure are not claimed to be transactionally solvable.

A missing database is initialized through the normal migration chain. An existing zero-byte, non-regular, newer-schema, or inconsistent-ledger file is not reset. Before migrating an existing supported database, the application writes and validates a self-describing pre-migration backup under the application-data `backups` directory. A failed checkpoint prevents migration. Each migration remains an atomic SQLite migration transaction; if one fails, the original file and the checkpoint are retained, the application reports a stable diagnostic, and ordinary world editing is not wired to that file. A process interruption between migration transactions may leave an earlier contiguous schema version; the next launch revalidates the ledger and checkpoints again before retrying. Disk-space exhaustion aborts the checkpoint or migration and is reported without claiming the upgrade succeeded. Safety files are intentionally not automatically pruned; the Player may remove older verified checkpoints after making their own copy.

If the persistent store cannot open, startup may offer a clearly labelled temporary in-memory world. Its warning states that changes will not be saved. This fallback never replaces the database. The failed persistent store remains isolated for recovery; where its regular SQLite file is still readable (for example, an interrupted migration), the Player can inspect a verified backup and run the normal guarded restore flow, then reload. An empty, malformed, inaccessible, or otherwise unreadable file is not overwritten by this recovery-only route: preserve the original and use a separately verified backup or seek assistance rather than treating the temporary world as a repaired database.

### Portable backups and explicit restore

A `.liferpg-backup` is a versioned, bounded envelope: magic and format version, a length-delimited strict JSON manifest, then a SQLite database image. The manifest identifies Life RPG, format and schema versions, UTC creation time, stable Player IDs and display names, database length, and SHA-256. It includes no machine-local database path or credentials. Backup bytes are obtained through SQLite's online backup API, so a committed WAL transaction is part of the snapshot. Publication uses a new destination and does not overwrite an earlier backup. Before success, the archive is reopened and checked for format, bounds, checksum, schema ledger, SQLite integrity, required tables, foreign keys, Player ownership invariants, and essential configuration.

Restore is initiated by the Player. The selected archive is validated without changing it; compatibility distinguishes same-schema, older supported (staged through the ordinary migrations), newer unsupported, and malformed backups. A different stable Player-ID set requires reviewing the Player names and typing `RESTORE`. A verified safety backup of the current file-backed world is created before replacement. Replacement and post-copy validation must both succeed to return success. If that phase errors, the application attempts to restore and validate the safety snapshot; it reports a rolled-back failure instead of claiming success. If both replacement and rollback fail, the store becomes fail-closed (`LR-RESTORE-01`) and refuses subsequent reads/writes until recovery; the preserved safety archive is the recovery point. Process termination or filesystem failure can prevent cleanup code from running, so the next launch still validates the database and never resets it automatically. A successful restore makes the existing UI stale; the application makes it inert and requires reload before editing the restored world.

The Status screen exposes backup creation, non-mutating archive inspection, compatibility/identity preview, explicit restore, and a bounded read-only integrity check. It is not a general repair tool. Integrity findings are reported, never silently rewritten.

### Release diagnostics and boundary

Player-facing storage failures use stable diagnostic identifiers and omit SQL, stack traces, raw operating-system errors, and machine-specific paths. Backup format DTOs have typed Rust and TypeScript mirrors. These operations preserve the existing domain model: backups and safety checkpoints are persistence mechanisms, not portable Workspace exports, new game events, or additional world entities. Workspace transfer/import semantics remain unchanged and are still separate from full-world backup/restore.

For package and install expectations, see the README: Linux `.deb`, AppImage, and executable targets are the current configured targets; manual upgrades preserve the existing database and run normal migrations. No automatic updater, synchronization, or additional platform promise is implied.
