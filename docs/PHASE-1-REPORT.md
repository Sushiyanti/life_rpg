# Phase 1 report — Foundation

Companion to [`ARCHITECTURE.md`](./ARCHITECTURE.md). This file records what was
built, what changed, what was verified, and what is deliberately missing.

---

## 1. Architecture chosen

**Tauri v2 desktop application: Rust core (`lr-*` Cargo workspace) + React 18 /
TypeScript frontend + SQLite (`rusqlite`, `bundled`), communicating over Tauri's
in-process command bridge.**

```text
lr-domain  ──▶  lr-application  ──▶  lr-persistence
   (pure)          (ports/use cases)      (SQLite adapter)
                          ▲
                          │
                    src-tauri (composition root + #[tauri::command])
                          ▲
                          │  typed IPC — no HTTP
                          │
                    src/ (React + TS)
```

## 2. Why it was chosen

| Requirement | How this satisfies it |
|---|---|
| Local desktop executable | Tauri produces a native binary; bundle targets `deb` + `appimage` are configured |
| Offline-first | Zero network calls anywhere in the codebase. SQLite C library is statically compiled in (`bundled`), so nothing is fetched at runtime |
| No HTTP server | Commands are in-process function calls. No port, no socket, no CORS, no auth surface |
| Proper database, not JSON files | SQLite with WAL, foreign keys on, real transactions |
| Domain independent of UI | Enforced by `Cargo.toml`: `lr-domain` has no `rusqlite` and no `tauri` dependency, so the compiler rejects a violation |
| UI not the source of truth | The frontend sends intents; SQL lives only in `lr-persistence`; `tauri-plugin-sql` is deliberately not used |
| Dynamic presentation | `src/presentation/spec.ts` defines a closed, data-only style schema with a single interpreter |
| Local IPC without HTTP | Tauri v2 `#[tauri::command]` over an in-process channel |

Alternatives rejected (Electron, a local HTTP server, Qt/GTK widgets,
`tauri-plugin-sql`) are argued in `ARCHITECTURE.md` §3.

## 3. Project structure

See the README's [repository layout](../README.md#repository-layout) for the
annotated tree. Summary of the five Rust members and the frontend:

| Member | Role | Notable contents |
|---|---|---|
| `crates/lr-domain` | Domain | `EntityId`, `Iso8601Timestamp`, `SchemaVersion`, `DomainError` |
| `crates/lr-application` | Application | `HealthStore` + `Clock` ports, `HealthService`, `AppError`/`StorageError` |
| `crates/lr-persistence` | Persistence | `SqliteHealthStore`, pragma config, migration runner, 3 embedded migrations |
| `crates/lr-contracts` | IPC boundary | `HealthReportDto`, `CommandErrorDto`, contract-drift test |
| `src-tauri` | Desktop shell | `bootstrap`, `AppState`, `get_status` / `get_world_location` / `ping` |
| `src/` | Frontend | `CoreClient` (only `invoke` call site), status screen + 4 components, tokens |

## 4. Database approach

- SQLite via `rusqlite` with **`bundled`** — no system SQLite required.
- Location: Tauri's `app_data_dir` (`com.liferpg.desktop`), file
  `life-rpg.sqlite3`.
- Pragmas: `journal_mode=WAL`, `foreign_keys=ON`, `synchronous=NORMAL`,
  `busy_timeout=5000`, `temp_store=MEMORY`.
- Migrations embedded with `include_str!`, tracked in a **`schema_migrations`
  ledger** (version, name, applied_at) rather than a bare `user_version`.
- Runner guarantees: idempotent, atomic per step, resumable, non-destructive,
  downgrade-guarded.
- Schema created in Phase 1: `schema_migrations`, `app_meta`, `health_probe`,
  `type_definitions` (+ seed vocabulary).
- `type_definitions` makes conceptual types a data concern: a new quest type is
  an `INSERT`, not a migration — proven by test.

## 5. Commands used

```bash
# environment inspection
pwd; ls -la; git status
rustc --version; cargo --version; node --version; npm --version; python3 --version
apt-cache policy libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev

# toolchain + system libraries
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
  libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libayatana-appindicator3-dev \
  patchelf xvfb

# project
npm install
python3 scripts/gen_icons.py
cargo test --workspace
npm run typecheck
npm test
npm run build:vite
cargo tauri build          # or: npm run build
```

## 6. Tests executed and results

See §7 for the actual captured output of this run.

| Suite | Tests | Covers |
|---|---|---|
| `lr-domain::value` | 7 | id validation/trim/rejection, `TryFrom`, version ordering, timestamp normalization + garbage rejection + round trip |
| `lr-application::services::health` | 7 | healthy verdict, deterministic tokens, behind-schema, too-new schema, read-back mismatch, unreachable, clock injection |
| `lr-persistence::migrations` | 8 | fresh migrate, idempotency, ledger history, partial resume, downgrade refusal, table creation, type extensibility, uniqueness |
| `lr-persistence::sqlite_store` | 7 | round trip, count accumulation, durability across reopen, WAL + FK pragmas, unavailable store, unwritable path, full seam through the service |
| `src-tauri::{lib,state}` | 6 | db path, real bootstrap, fallback, clock, durable restarts, warning state |
| `lr-contracts` | 4 | lowercase status, JSON key drift, null serialization, error code mapping |
| `src/test/ipc.test.ts` | 9 | command names vs Rust registrations, payload passthrough, error normalization, `cause` preservation, malformed input |
| `src/test/presentation-spec.test.ts` | 9 | token mapping for every closed union, spec-presence behaviour, no-out-of-set emission, JSON-only spec |
| `src/test/StatusScreen.test.tsx` | 7 | loading, healthy, pending migration, failed, core-unreachable, refresh, in-memory labelling |

## 6a. Verified GUI launch (headless sandbox)

The packaged release binary was launched against a **clean state** (data
directory deleted first) under `xvfb-run`:

```bash
rm -rf ~/.local/share/com.liferpg.desktop
xvfb-run -a ./target/release/life-rpg
```

Observed result — the application process started, stayed running until killed,
and created a real world database on disk:

```text
~/.local/share/com.liferpg.desktop/
  life-rpg.sqlite3        4096 B
  life-rpg.sqlite3-wal   70072 B      <-- WAL mode active on a real file
  life-rpg.sqlite3-shm   32768 B
  hsts-storage.sqlite, CacheStorage/, WebKitCache/, storage/, mediakeys/
```

Inspecting the created database read-only:

```text
journal_mode: wal
tables: ['app_meta', 'health_probe', 'schema_migrations', 'sqlite_sequence',
         'type_definitions']
migration ledger: [(1,'0001_core_ledger','2026-09-25T02:35:36.932827576+00:00'),
                   (2,'0002_health_probe','2026-09-25T02:35:36.932827576+00:00'),
                   (3,'0003_type_definition_registry','2026-09-25T02:35:36.932827576+00:00')]
type_definitions by namespace: [('comment_target',7),('effect',4),
                                ('narrative_entry',7),('quest',5),
                                ('skill_tree',4),('transaction',3)]
app_meta: [('store.format','life-rpg/sqlite'),('store.world_label','Primary World')]
```

So: the executable launches, SQLite opens a file on disk, WAL engages, and all
three migrations apply with real timestamps and seed the type registry.

**Unconfirmed in this environment:** the status screen was *not* confirmed
pixel-by-pixel, and no `health_probe` row was observed after the headless run —
in other words the webview's `get_status` call was not proven to complete. The
sandbox has no GPU/compositor, so WebKitGTK renders unreliably under `xvfb`
(the screenshot came back effectively blank). The command path itself is covered
by tests instead:
`end_to_end_through_the_application_service` drives the full
adapter → port → use-case seam and asserts `HealthStatus::Ok` with a matching
round trip, and `bootstrapped_world_is_durable_across_restarts` proves the row
survives a restart. Confirming the rendered screen needs a real desktop session.

### Defect found and fixed during verification

The original CSP was `default-src 'self'` with **no `connect-src`**. Tauri v2
reaches its IPC channel at `ipc:` (Linux/macOS) or `http://ipc.localhost`
(Windows), so that policy blocked every command before it reached Rust — the app
would have launched and then failed every call. Fixed by adding:

```text
connect-src ipc: http://ipc.localhost http://localhost:1420 ws://localhost:1420
```

The duplicate hand-written `<meta http-equiv="Content-Security-Policy">` in
`index.html` was also removed: two competing policies is fragile, and the meta
one was the stricter of the two. `tauri.conf.json` is now the single source.

---

## 7. Known limitations

Stated plainly, because a foundation is only trustworthy if its edges are
declared:

1. **No domain aggregates yet.** No Player, Quest, Skill, Skill Tree, Effect,
   Transaction, Comment or Narrative Entry. Phase 1 ships the primitives
   (`EntityId`, `Iso8601Timestamp`, `SchemaVersion`) and the type vocabulary the
   aggregates will be built from.
2. **No rules engine.** No condition/trigger/action machinery, as instructed.
3. **No dynamic UI system.** `presentation/spec.ts` defines the vocabulary and
   an interpreter, and `InfoCard` uses it, but nothing loads presentation
   records from the database yet.
4. **No persisted UI state and no workspaces.**
5. **One screen.** No router, no navigation.
6. **`health_probe` grows without bound.** It is a probe table, not world data,
   and re-running the status check appends a row. Phase 2 should prune it
   (keep-N or age-based) or move the probe behind a rotating token. Called out
   here so it is not mistaken for a leak.
7. **Single-connection design.** `Mutex<Connection>` is correct for one user and
   one process; it would need revisiting only if a second process ever opened the
   same file.
8. **In-memory fallback is not durable.** If the file-backed store fails to open,
   the app runs with an in-memory world and says so on the status screen. It does
   not retry the file, and nothing written in that session survives.
9. **No backup/restore or export.** Backing up currently means copying the
   `.sqlite3` file.
10. **`type_definitions` is seeded but unused by any query.** No repository reads
    it yet; the test proves the mechanism works.
11. **Bundle targets configured for Linux (`deb`, `appimage`).** macOS/Windows
    bundling needs those platforms (or CI runners) and their icons regenerated.
12. **`exactOptionalPropertyTypes` is off** in the frontend tsconfig; enabling it
    would tighten optional-field handling at some ergonomic cost.

## 8. Exact next logical phase

**Phase 2 — Domain aggregates and repositories.**

Scope, in dependency order:

1. **Repository ports** in `lr-application` (`PlayerRepository`,
   `QuestRepository`, `SkillRepository`, `SkillTreeRepository`,
   `EffectRepository`, `TransactionRepository`, `CommentRepository`,
   `NarrativeEntryRepository`, `TypeDefinitionRepository`) — same shape as
   `HealthStore`: plain DTOs + `Result<_, StorageError>`, no SQL types.
2. **Migrations 0004+** creating the corresponding tables, referencing
   `type_definitions (namespace, code)` by foreign key, with
   `UNIQUE (player_id, snapshot_date)` on `player_state_snapshot`.
3. **Aggregates** in `lr-domain` built from `EntityId` / `Iso8601Timestamp`,
   with invariant-enforcing constructors and pure methods
   (`Quest::advance_progress`, `Skill::award_xp`) that return
   `Result<_, DomainError>`.
4. **Append-only transaction ledger** with a `TransactionKind` distinguishing
   gain/loss, plus resource and source reference.
5. **Multi-write atomicity**: XP award and its transaction row commit in one
   `Connection::transaction`.
6. **Status screen stays green**; add a second screen (Character) reading real
   Player state, which is the first genuine proof the layering holds under a
   non-trivial feature.
7. **Cached-vs-derived consistency test**: a property test asserting that
   `Player.total_xp` always equals `SUM(transactions.amount)` after any
   sequence of operations.

No rules engine, no dynamic UI, and no workspace work until Phase 2 lands.
