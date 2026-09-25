# Life RPG

A **local-first desktop application** that models your real life as a persistent
RPG world: state, history, progression, quests, skills, effects and transactions —
stored in SQLite on your own machine.

No cloud. No account. No server. Works with the network cable unplugged.

> **Status: Phase 1 — Foundation.** This build is a genuinely runnable
> application, not an architecture document. It launches a desktop window,
> opens a real SQLite database, migrates it to a versioned schema, performs a
> transaction-wrapped write/read round trip, and reports all of it on a status
> screen. The Player / Quest / Skill domain is deliberately **not** implemented
> yet — see [What Phase 1 does *not* do](#what-phase-1-does-not-do).

---

## Table of contents

- [Architecture at a glance](#architecture-at-a-glance)
- [Why this stack](#why-this-stack)
- [Repository layout](#repository-layout)
- [Install dependencies](#install-dependencies)
- [Run in development mode](#run-in-development-mode)
- [Run the tests](#run-the-tests)
- [Build the application](#build-the-application)
- [Where your data lives](#where-your-data-lives)
- [Database approach](#database-approach)
- [What Phase 1 does *not* do](#what-phase-1-does-not-do)
- [Troubleshooting](#troubleshooting)

---

## Architecture at a glance

Seven responsibilities, kept separate on purpose. Each row is a *question*, and
exactly one layer is allowed to answer it:

| Layer | Question it answers | Crate / directory |
|---|---|---|
| **Domain** | What exists in the world? | `crates/lr-domain` |
| **State** | What condition is it in? | *(Phase 2 — cached fields on aggregates)* |
| **History** | What happened in the past? | *(Phase 2 — transaction ledger, snapshots)* |
| **Rules** | How does the world change? | *(Phase 3 — conditions/triggers/actions)* |
| **Application** | What commands/queries operate on the world? | `crates/lr-application` |
| **Persistence** | How is everything saved? | `crates/lr-persistence` |
| **Presentation** | How should a thing look? | `src/presentation` |
| **UI state / Workspace** | How does the user want the interface arranged? | *(Phase 4 — persisted UI state)* |

The **IPC contract** (`crates/lr-contracts` ⇄ `src/domain`) is a boundary, not a
layer: it exists so neither side has to import the other's types.

Dependency direction is strictly one-way and enforced by `Cargo.toml`, not by
convention:

```text
        lr-domain          (no dependencies except thiserror/chrono)
            ▲
            │
      lr-application       (ports + use cases; no SQL, no Tauri)
            ▲
            │
      lr-persistence       (rusqlite adapter; the ONLY crate that knows SQL)
            ▲
            │
        src-tauri          (desktop shell: wires ports, exposes commands)
            ▲
            │  typed IPC (in-process, no HTTP)
            │
        src/ (React)       (never sees SQL, never sees a Rust type)
```

`lr-domain` has no `rusqlite` and no `tauri` in its dependency list — so
`cargo test -p lr-domain` compiles and runs with zero infrastructure.

---

## Why this stack

| Choice | Reason |
|---|---|
| **Tauri v2** | Produces a real native executable. The Rust core and the webview communicate over an **in-process IPC channel**, so there is no port, no socket, no CORS and nothing reachable from another machine. This is the "local/native application boundary" the design calls for. |
| **React + TypeScript** | The UI is going to become a dynamic, data-driven presentation layer (variants, density, workspaces). A component model with strong typing is the right fit; `strict` mode plus a hand-checked contract keeps the boundary honest. |
| **Rust** | The core is a long-lived local process holding a database and (later) a rules engine. Memory safety without a GC, a first-class test story, and single-binary distribution. |
| **SQLite (`rusqlite`, `bundled`)** | The canonical local-first database. `bundled` compiles the C library into the binary, so there is **no system SQLite to install** and no version drift between machines. |
| **Cargo workspace** | Makes the layer boundaries *compile-time* facts. `lr-domain` cannot accidentally `use rusqlite` if `rusqlite` is not in its manifest. |

### Why there is no HTTP server

Requirements: one user, offline-first, no LAN, no remote clients. An HTTP server
would add an open port, a startup race, CORS/auth surface and a serialization hop
in exchange for **nothing** — because the only client already lives in the same
process tree. Tauri's command bridge gives the same request/response ergonomics
with none of the cost. No Django, no FastAPI, no REST, no GraphQL, no WebSockets.

---

## Repository layout

```text
life-rpg/
├─ Cargo.toml                     # workspace: 5 members, shared dependency versions
├─ package.json                   # frontend + orchestration scripts
├─ vite.config.ts                 # dev port pinned to 1420 (Tauri's devUrl)
├─ tsconfig.json                  # strict: true, noUncheckedIndexedAccess: true
├─ index.html
│
├─ crates/
│  ├─ lr-domain/                  # DOMAIN — what exists in the world
│  │  └─ src/
│  │     ├─ lib.rs                #   layer contract + roadmap table
│  │     ├─ error.rs              #   rule-violation vocabulary
│  │     └─ value.rs              #   EntityId, SchemaVersion, Iso8601Timestamp
│  │
│  ├─ lr-application/             # APPLICATION — use cases + ports
│  │  └─ src/
│  │     ├─ lib.rs
│  │     ├─ error.rs              #   StorageError (port) / AppError (command)
│  │     ├─ ports.rs              #   HealthStore, Clock — traits, no SQL
│  │     └─ services/
│  │        └─ health.rs          #   HealthService + 7 unit tests
│  │
│  ├─ lr-persistence/             # PERSISTENCE — the only SQLite-aware crate
│  │  └─ src/
│  │     ├─ lib.rs
│  │     ├─ error.rs             #   PersistenceError -> StorageError translation
│  │     ├─ pragma.rs            #   WAL / foreign_keys / synchronous / busy_timeout
│  │     ├─ migrations.rs        #   versioned runner + ledger + 8 tests
│  │     ├─ sqlite_store.rs      #   SqliteHealthStore + 7 tests
│  │     └─ migrations/
│  │        ├─ 0001_core_ledger.sql
│  │        ├─ 0002_health_probe.sql
│  │        └─ 0003_type_definition_registry.sql
│  │
│  └─ lr-contracts/               # IPC BOUNDARY — DTOs shared with the frontend
│     └─ src/lib.rs               #   + contract-drift test pinning JSON keys
│
├─ src-tauri/                     # DESKTOP SHELL — the composition root
│  ├─ Cargo.toml
│  ├─ build.rs
│  ├─ tauri.conf.json             # window, CSP, bundle targets (deb, appimage)
│  ├─ capabilities/default.json   # Phase 1 permissions: core:default only
│  └─ src/
│     ├─ main.rs                  # thin entry point
│     ├─ lib.rs                   # bootstrap + run() + 5 tests
│     ├─ state.rs                 # AppState (DI container)
│     └─ commands/
│        ├─ mod.rs
│        └─ status.rs             # get_status / get_world_location / ping
│
├─ src/                           # FRONTEND (React + TS)
│  ├─ main.tsx                    # entry: imports tokens.css then global.css
│  ├─ App.tsx                     # renders one screen: the status screen
│  ├─ app/
│  │  ├─ AppShell.tsx             # outer chrome (title band + content slot)
│  │  └─ AppShell.css
│  ├─ domain/
│  │  ├─ health.ts                # hand-mirrored contract types + helpers
│  │  └─ ipc.ts                   # CoreClient — the ONE place invoke is called
│  ├─ presentation/
│  │  └─ spec.ts                  # controlled style schema + its interpreter
│  ├─ features/status/
│  │  ├─ StatusScreen.tsx         # the Phase 1 screen
│  │  ├─ StatusScreen.css
│  │  ├─ StatusPill.tsx
│  │  ├─ InfoCard.tsx
│  │  ├─ MigrationLedger.tsx
│  │  └─ useHealthReport.ts       # use-case hook (loading/error/refresh)
│  ├─ styles/
│  │  ├─ tokens.css               # design tokens (the only place colors exist)
│  │  └─ global.css
│  └─ test/                       # setup, fixtures, 3 test suites
│
├─ scripts/gen_icons.py           # regenerates src-tauri/icons (stdlib only)
└─ docs/
   ├─ ARCHITECTURE.md             # the full design rationale
   └─ PHASE-1-REPORT.md           # what was built, changed, tested, and left over
```

---

## Install dependencies

### 1. Prerequisites

| Tool | Version used | Purpose |
|---|---|---|
| Node.js | 22.x | frontend build + tests |
| npm | 10.x | package management |
| Rust (stable) | 1.98+ | core + desktop shell |
| Python 3 | 3.12 (optional) | regenerate app icons |

```bash
# Rust, if you do not have it
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Linux system libraries** (Tauri needs a webview; these are install-time only):

```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev libayatana-appindicator3-dev patchelf
```

macOS and Windows need no extra system packages beyond Xcode CLT / MSVC build
tools.

SQLite itself is **not** a prerequisite: `rusqlite` is built with the `bundled`
feature, so the engine is compiled into the binary.

### 2. Project dependencies

```bash
npm install
```

That is the only install step — Rust dependencies are fetched automatically by
Cargo on first build.

---

## Run in development mode

```bash
npm run dev
```

This launches Tauri, which (a) starts Vite on `http://localhost:1420`, (b) builds
the Rust core, (c) opens a native window pointed at the dev server. Hot reload
works for the frontend; changing Rust files triggers a core rebuild.

Prefer a browser-only loop while working purely on styles?

```bash
npm run dev:vite     # Vite only — the status screen will show a connection error,
                     # because there is no core and no HTTP fallback. That is by design.
```

---

## Run the tests

```bash
# Rust: domain + application + persistence + shell (no display, no DB server)
cargo test --workspace

# Frontend: contract, IPC boundary, presentation interpreter, status screen
npm test

# Type-check only
npm run typecheck
npm run rust:check
```

`npm test` runs Vitest in watch-free mode; use `npm run test:watch` while
iterating.

Rust tests never touch your real world database: they use in-memory SQLite or a
`tempfile` directory that is deleted afterwards.

---

## Build the application

```bash
npm run build          # production bundle for the current OS
```

Artifacts land in `target/release/bundle/`:

- **Linux** — `deb/life-rpg_0.1.0_amd64.deb` and `appimage/life-rpg_0.1.0_amd64.AppImage`
- **macOS** — `.app` / `.dmg`
- **Windows** — `.msi` / `.exe`

The bare executable is also at `target/release/life-rpg`, which is handy for a
smoke test:

```bash
./target/release/life-rpg
```

To regenerate the icons:

```bash
python3 scripts/gen_icons.py
```

---

## Where your data lives

The database sits in Tauri's per-user app data directory, under the identifier
`com.liferpg.desktop`:

| OS | Path |
|---|---|
| Linux | `~/.local/share/com.liferpg.desktop/life-rpg.sqlite3` |
| macOS | `~/Library/Application Support/com.liferpg.desktop/life-rpg.sqlite3` |
| Windows | `%APPDATA%\com.liferpg.desktop\life-rpg.sqlite3` |

The status screen prints the exact resolved path, so you never have to guess.

Backing up is copying that one file (plus its `-wal` sibling while the app runs).
Deleting it starts a fresh world on next launch.

---

## Database approach

**SQLite via `rusqlite` with the `bundled` feature.** Rationale, and the
alternatives that were rejected:

- **Not a pile of JSON files.** JSON files give you no transactions, no
  referential integrity, no concurrency story, and rewriting a 40 MB file on
  every XP tick is a bug waiting to happen. The requirements explicitly reject
  this, and rightly.
- **Not `tauri-plugin-sql` from the frontend.** That would make the UI the place
  where SQL is written — the opposite of the layering this project is built on.
  SQL lives in `lr-persistence`; the frontend sends *intents*.
- **Not disk-cache files as IPC.** The cache is not the source of truth, and
  using it as a channel makes history and state indistinguishable.

### Pragmas (set on every connection)

| Pragma | Value | Why |
|---|---|---|
| `journal_mode` | `WAL` | Readers don't block the writer; the database survives a hard crash mid-write. |
| `foreign_keys` | `ON` | Off by default in SQLite. Referential integrity should be real from day one, because retrofitting it later means auditing every row. |
| `synchronous` | `NORMAL` | Durable under WAL while keeping per-write latency low on an SSD. |
| `busy_timeout` | 5000 ms | Tolerate momentary contention instead of erroring out. |
| `temp_store` | `MEMORY` | Keep scratch work off the disk. |

### Migrations

Embedded as `&'static str` via `include_str!`, applied by a small runner, and
recorded in a **`schema_migrations` ledger table**.

Why a ledger table instead of only `PRAGMA user_version`? Because this
application is *about* history: a ledger records each migration's version,
**name** and **applied-at timestamp**, so the status screen can show when each
piece of the schema appeared. A bare integer cannot answer that. Version numbers
remain monotonic, so the downgrade guard still works (a store written by a
newer build is refused, never silently corrupted).

Guarantees the runner provides, all covered by tests:

1. **Idempotent** — already-applied versions are skipped, so it is safe to call
   on every launch.
2. **Atomic per step** — each migration and its ledger insert commit together in
   one transaction; a crash can never mark a half-applied step as done.
3. **Resumable** — a store that only got as far as v1 continues from v2.
4. **Non-destructive** — applying migrations to an existing store never wipes
   rows.
5. **Downgrade-guarded** — refuses a store whose schema is newer than the build.

**Rule for contributors: never edit an applied migration. Add a new one.**

### Type definitions are data, not code

`0003_type_definition_registry.sql` creates `type_definitions` and seeds the
vocabulary named in the requirements (quest: main/side/daily/long_term/challenge;
skill_tree: programming/fitness/education/life; effect: buff/debuff/condition/
temporary_modifier; plus transaction, narrative_entry and comment_target
namespaces).

Adding a new conceptual type — say a `raid` quest type — is an `INSERT`:

```sql
INSERT INTO type_definitions (namespace, code, label) VALUES ('quest', 'raid', 'Raid');
```

No migration, no recompile. A test proves exactly this.

What it is *not*: the shared, structural fields (namespace, code, label,
ordering, active flag) are real typed indexed columns. Only genuinely open-ended
extras go in `metadata_json`. Types have a shape; their metadata does not. This
is "database-driven types", not "everything is JSON".

### Presentation is a controlled schema, not CSS

`src/presentation/spec.ts` defines closed unions for `surface`, `radius`,
`shadow`, `accent` and text sizing, plus an interpreter that maps tokens to CSS
custom properties. A stored style record can only ever produce `var(--…)`
references or literals from the radius table — there is no code path that pushes
a stored string into CSS, and no path that evaluates stored JavaScript. A test
asserts this.

### Current state vs. history

Kept deliberately distinct, and this separation is enforced by the schema shape:

- **Current state** — cached fields on aggregates (`Player.total_xp`), fast to read. *(Phase 2)*
- **History** — an append-only transaction ledger; the authoritative record of
  change. *(Phase 2)*
- **Snapshots** — immutable per-day records of what the world looked like. *(Phase 2)*

Phase 1 already demonstrates the pattern in miniature: `health_probe` rows are
append-only and never updated, while the report's `probeRows` is a derived count.

---

## What Phase 1 does *not* do

By explicit instruction, and listed here so nobody mistakes absence for
oversight:

- No Player, Quest, Skill, Skill Tree, Effect, Transaction, Comment or Narrative
  Entry aggregates. Phase 1 ships the *primitives* those will be built from
  (`EntityId`, `Iso8601Timestamp`, `SchemaVersion`) plus their type vocabulary.
- No rules engine (condition → trigger → action).
- No dynamic UI system, workspace layout, or style sandbox.
- No persisted UI state.
- No character sheet, quest board, or skill tree screens.

The migrations create only `app_meta`, `health_probe` and `type_definitions` —
deliberately little, because later phases should add tables that encode real,
understood invariants rather than guesses made today.

---

## Troubleshooting

**`failed to find a `wry` …` / no `webkit2gtk-4.1` found**
Install the Linux system libraries listed under [prerequisites](#1-prerequisites).

**The window opens but the status screen says `failed`**
That is the app working correctly: the core opened a window and is telling you
storage is unreachable. Read the *Problems* list — it names the exact cause
(usually an unwritable data directory). The app deliberately falls back to an
in-memory world rather than refusing to start, and the screen says so.

**"in-memory (not persisted to disk)" appears in the Store row**
You are either in a test, or the file-backed open failed. Check the data
directory's permissions.

**`npm run dev:vite` shows a connection error**
Expected — a browser tab has no Rust core, and there is no HTTP fallback by
design. Use `npm run dev`.

**Port 1420 already in use**
Vite is configured with `strictPort: true` so it fails loudly instead of
silently moving. Stop the other process.
