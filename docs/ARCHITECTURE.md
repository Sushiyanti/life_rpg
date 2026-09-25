# Life RPG — Architecture

This document records *why* the system is shaped the way it is. The README
describes how to use it; this describes the reasoning, the boundaries, and the
decisions that will matter in later phases.

---

## 1. The seven responsibilities

The brief separates seven concerns. Each is a question, and exactly one layer is
allowed to answer it:

| # | Responsibility | Question | Phase 1 implementation |
|---|---|---|---|
| 1 | **Domain** | What exists in the world? | `crates/lr-domain` — value objects, error vocabulary |
| 2 | **State** | What condition is it in? | *(Phase 2)* — cached fields on aggregates |
| 3 | **History** | What happened in the past? | *(Phase 2)* — append-only ledger; Phase 1 shows the pattern via `health_probe` |
| 4 | **Rules** | How does the world change? | *(Phase 3)* — condition/trigger/action |
| 5 | **Application** | What commands/queries operate on the world? | `crates/lr-application` — ports + `HealthService` |
| 6 | **Presentation** | How should a thing look? | `src/presentation/spec.ts` — controlled style schema |
| 7 | **UI state / Workspace** | How does the user want the interface arranged? | *(Phase 4)* |
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

No `tauri`, no `rusqlite`, no `serde`. If a developer tries to add a UI concern
to the domain, they must first edit the manifest and justify it in review.
`cargo test -p lr-domain` compiles with zero infrastructure, which is the
practical payoff.

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

Commands are three-line adapters: take `State`, call one use case, convert to a
DTO. If a command grows an `if` that encodes a game rule, that rule is in the
wrong layer.

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

## 6. Roadmap

| Phase | Scope | Status |
|---|---|---|
| **1** | Foundation: workspace, shell, SQLite, migrations, layering, health screen, tests | **complete** |
| 2 | Domain aggregates: Player, Quest, Skill, Skill Tree, Effect, Transaction, Comment, Narrative Entry; repositories; snapshots; type-definition repository | next |
| 3 | Rules engine: condition → trigger → action; reusable penalties | planned |
| 4 | Dynamic presentation: stored presentation records, UI-state persistence, workspace layout | planned |
| 5 | Style sandbox/editor and workspace customization | planned |
| 6 | Packaging polish, backup/restore, export | planned |

### Phase 2 preview — what the foundation already decided

- Aggregates reference `type_definitions` by `(namespace, code)`; adding a quest
  type stays a data operation.
- Repositories implement port traits in `lr-application`, the same shape as
  `HealthStore`, and live in `lr-persistence::repositories`.
- `EntityId` is the primary key type; `Iso8601Timestamp` is the time type.
- Every mutation that writes more than one row goes through
  `Connection::transaction`.
- `player_state_snapshot` gets `UNIQUE (player_id, snapshot_date)` — the brief's
  uniqueness rule — with corrections handled by an explicit correction
  mechanism, not by overwriting history.
