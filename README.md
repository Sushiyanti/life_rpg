# Life RPG

A **local-first desktop application** that models your real life as a persistent
RPG world: state, history, progression, quests, skills, effects and transactions —
stored in SQLite on your own machine.

No cloud. No account. No server. Works with the network cable unplugged.

> **Status: v1.0.0 — first stable release.** Release scope is Linux x86_64 on
> Ubuntu 22.04 (Jammy) or a compatible newer GTK 3/WebKitGTK 4.1 system. See the
> [v1.0.0 release notes](V1.0.0-RELEASE-NOTES.md), [Phase 13 validation report](docs/PHASE-13-REPORT.md),
> [Phase 12 report](docs/PHASE-12-REPORT.md), [Domain Design Codex](docs/DOMAIN-DESIGN-CODEX.md),
> and [Architecture](docs/ARCHITECTURE.md).

---

## Table of contents

- [Architecture at a glance](#architecture-at-a-glance)
- [Download v1.0.0](#download-v100)
- [Why this stack](#why-this-stack)
- [Repository layout](#repository-layout)
- [Install dependencies](#install-dependencies)
- [Run in development mode](#run-in-development-mode)
- [Run the tests](#run-the-tests)
- [Build the application](#build-the-application)
- [Where your data lives](#where-your-data-lives)
- [Database approach](#database-approach)
- [Release status and deferred scope](#release-status-and-deferred-scope)
- [Troubleshooting](#troubleshooting)

---

## Download v1.0.0

The first stable release provides Linux x86_64 `.deb` and AppImage packages for
Ubuntu 22.04 (Jammy) or compatible newer systems with GTK 3 and WebKitGTK 4.1.
Download the packages and `SHA256SUMS` from the
[Life RPG v1.0.0 GitHub Release](https://github.com/Sushiyanti/life_rpg/releases/tag/v1.0.0).
After downloading all three files into one directory, verify the binaries with
`sha256sum --check SHA256SUMS`.

This release is not validated for Windows, macOS, older Ubuntu releases, or other
CPU architectures. Backup files are local and unencrypted; treat them as sensitive
data. The archive checksum detects corruption but does not authenticate who made
the backup.

---

## Architecture at a glance

Seven responsibilities, kept separate on purpose. Each row is a *question*, and
exactly one layer is allowed to answer it:

| Layer | Question it answers | Crate / directory |
|---|---|---|
| **Domain** | What exists in the world? | `crates/lr-domain` |
| **State** | What condition is it in? | Structured aggregate/current-value tables |
| **History** | What happened in the past? | Append-only transactions, lifecycle/revision history, and immutable snapshots |
| **Rules** | How does the world change? | `lr-application` — closed declarative conditions, event triggers, and actions |
| **Application** | What commands/queries operate on the world? | `crates/lr-application` |
| **Persistence** | How is everything saved? | `crates/lr-persistence` |
| **Presentation** | How should a thing look? | `src/presentation` |
| **UI state / Workspace** | How does the user want the interface arranged? | `src/App.tsx`, `src/features/workspaces` — persisted, Player-scoped panels and preferences |

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

### Phase 3 rule flow

```text
typed domain event → enabled rules (priority DESC, Rule ID ASC)
  → typed condition evaluation → ordered action planning
  → domain invariant checks → bounded FIFO follow-on events
  → one application-port transaction for root changes + XP ledger + audit
```

The first event set is `quest_completed`, `player_xp_changed`, and `stat_changed`.
Version 1 conditions are explicit numeric/text comparisons and `ALL`/`ANY`/`NOT`;
actions are XP award/penalty, valid Quest completion, and bounded Player Stat set/
modify. Rules are data only—there is no expression parser, `eval`, scripting, or
scheduler. Chain limits and exact supported fields are documented in the [Codex](docs/DOMAIN-DESIGN-CODEX.md).

Phase 3.5 adds `concept_progress_changed` to the same closed event system, with
bounded `set_concept_progress` actions. Concepts remain meaningful world subjects;
Quests, Skills, Narratives, Comments, Transactions, and Effects remain their own
typed entities and may be linked to a Concept. Progress tracks explicitly
distinguish numeric, percentage, experience, level, and mastery semantics.
Global search is an application query over a compact SQLite FTS5 text projection,
not a second copy of the world.

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
│  │     ├─ lib.rs                #   domain exports
│  │     ├─ error.rs              #   domain-violation vocabulary
│  │     ├─ value.rs              #   validated identity/date/time/version values
│  │     ├─ player.rs             #   Player progression and XP floor
│  │     ├─ quest.rs / skill.rs   #   current Quest and Skill lifecycles
│  │     ├─ concept.rs            #   typed Concept, links, and progress semantics
│  │     └─ stat.rs               #   data-defined Player Stat values
│  │
│  ├─ lr-application/             # APPLICATION — use cases + ports
│  │  └─ src/
│  │     ├─ lib.rs
│  │     ├─ error.rs              #   storage, domain, and rule-chain errors
│  │     ├─ ports.rs              #   focused world, Concept, and search ports — no SQL
│  │     ├─ rules.rs              #   versioned event/condition/action data model
│  │     ├─ search.rs             #   typed global search query/results
│  │     └─ services/
│  │        ├─ health.rs          #   operational health use case
│  │        ├─ world.rs           #   Player/world use cases
│  │        ├─ concepts.rs        #   Concept use cases and detail query
│  │        └─ rule_engine.rs     #   trusted, bounded deterministic interpreter
│  │
│  ├─ lr-persistence/             # PERSISTENCE — the only SQLite-aware crate
│  │  └─ src/
│  │     ├─ lib.rs
│  │     ├─ error.rs             #   PersistenceError -> StorageError translation
│  │     ├─ pragma.rs            #   WAL / foreign_keys / synchronous / busy_timeout
│  │     ├─ migrations.rs        #   schema versions 1–17 + upgrade tests
│  │     ├─ backup.rs            #   online snapshots, integrity, staged restore
│  │     ├─ sqlite_store.rs      #   health/connection and migration-checkpoint adapter
│  │     ├─ world_store.rs       #   atomic world/rule persistence + tests
│  │     ├─ concept_store.rs     #   Concept/progress/search adapters + tests
│  │     └─ semantics_store.rs   #   activity/recovery/preferences/suggestions
│  │     └─ migrations/
│  │        ├─ 0001_core_ledger.sql … 0003_type_definition_registry.sql
│  │        ├─ 0004_phase2_domain.sql
│  │        ├─ 0005_phase21_integrity.sql
│  │        ├─ 0006_phase3_rules.sql
│  │        ├─ 0007_phase35_concepts.sql
│  │        ├─ 0008_phase36_world_semantics.sql
│  │        ├─ 0009_phase5_workspaces.sql … 0013_phase7_content_guidance.sql
│  │        ├─ 0014_phase8_timeline_indexes.sql … 0016_phase10_gameplay_rules_progression.sql
│  │        └─ 0017_phase11_tags.sql
│  │
│  └─ lr-contracts/               # IPC BOUNDARY — DTOs shared with the frontend
│     └─ src/world.rs             #   world and Rule DTOs; lib.rs pins wire contracts
│
├─ src-tauri/                     # DESKTOP SHELL — the composition root
│  ├─ Cargo.toml
│  ├─ build.rs
│  ├─ tauri.conf.json             # window, CSP, bundle targets (deb, appimage)
│  ├─ capabilities/default.json   # scoped IPC plus native backup open/save pickers
│  └─ src/
│     ├─ main.rs                  # thin entry point
│     ├─ lib.rs                   # bootstrap + invoke-handler registration
│     ├─ state.rs                 # AppState (DI container)
│     └─ commands/
│        ├─ mod.rs
│        ├─ status.rs             # health/location/liveness adapters
│        ├─ backup.rs             # backup, validation, restore, integrity commands
│        └─ world.rs              # thin world and Rule command adapters
│
├─ src/                           # FRONTEND (React + TS)
│  ├─ main.tsx                    # entry: imports tokens.css then global.css
│  ├─ App.tsx                     # coordinates route/world state and typed data
│  ├─ app/
│  │  ├─ AppShell.tsx             # persistent navigation, world selector, actions
│  │  └─ AppShell.css
│  ├─ domain/
│  │  ├─ health.ts                # hand-mirrored health contract types + helpers
│  │  ├─ backup.ts                # hand-mirrored backup and integrity DTOs
│  │  └─ ipc.ts                   # CoreClient — the ONE place invoke is called
│  ├─ presentation/
│  │  └─ spec.ts                  # controlled style schema + its interpreter
│  ├─ features/status/
│  │  ├─ StatusScreen.tsx         # app health, recovery, integrity and backup
│  │  ├─ BackupRestorePanel.tsx   # explicit local backup/restore workflow
│  │  ├─ StatusScreen.css
│  │  ├─ StatusPill.tsx
│  │  ├─ InfoCard.tsx
│  │  ├─ MigrationLedger.tsx
│  │  └─ useHealthReport.ts       # use-case hook (loading/error/refresh)
│  ├─ features/world/
│  │  ├─ WorldWorkspace.tsx       # structured panels and major world screens
│  │  ├─ WorldExplorer.tsx        # search, typed detail, lifecycle and history
│  │  ├─ PlayerCharacter.tsx      # Player progression, stats and snapshots
│  │  ├─ EffectsScreen.tsx        # Effect status and contextual visibility
│  │  └─ RulePanel.tsx            # typed Rule authoring and audit view
│  ├─ styles/
│  │  ├─ tokens.css               # design tokens (the only place colors exist)
│  │  └─ global.css
│  └─ test/                       # Rust/IPC/UI regression coverage
│
├─ scripts/gen_icons.py           # regenerates src-tauri/icons (stdlib only)
└─ docs/
   ├─ ARCHITECTURE.md             # current design and release-safety guarantees
   ├─ DOMAIN-DESIGN-CODEX.md      # canonical world semantics
   └─ PHASE-12-REPORT.md          # hardening evidence and known limitations
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

This repository currently configures and validates Linux packaging only. It does
not claim macOS or Windows installer support.

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
npm run dev:vite     # Vite only — the shell renders, but world operations show the
                     # expected connection warning because no Tauri core is attached.
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
npm run build          # production bundle for the configured Linux targets
```

Artifacts land in `target/release/bundle/`. The configured Tauri bundle targets
are Linux `.deb` and AppImage only; this repository does not configure or claim
macOS or Windows release packaging. Ready-to-install packages are published on
the [GitHub Releases page](https://github.com/Sushiyanti/life_rpg/releases):

- **Linux x86_64** — `.deb` and AppImage packages for v1.0.0.

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

Paths and packaging on other operating systems are outside the v1.0.0 supported
release scope.

The status screen reports a safe location hint rather than printing an absolute
machine path. The built-in **System health → Backups & recovery** area uses
SQLite's online backup API, so committed WAL state is included even while the
application is running. Choose a `.liferpg-backup` file through the system save
dialog; the archive contains a versioned manifest, Player identity descriptors,
schema/creation metadata, and a SHA-256 database checksum, but not the source
installation path. The archive is validated before success is reported, and
existing files are never overwritten.

Do **not** copy only the live `.sqlite3` file as a backup while the app is open;
committed transactions can still reside in its WAL. Do not delete the world file
to reset the app. An existing empty, malformed, inconsistent, or newer-schema
database is left in place and the app fails safe rather than silently creating
a replacement world. Use a verified backup or preserve the original file before
any manual recovery.

When manually installing a newer build, Life RPG keeps using this same app-data
database and applies only the next forward migrations after writing a verified
pre-upgrade checkpoint. A failed checkpoint or migration leaves the existing
world in place and opens a clearly marked temporary, non-persistent fallback;
it never resets to a new world. A newer unsupported schema is refused, not
downgraded. After a verified restore, reload the app before editing so no stale
view can write against the replaced world.

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

- **Current state** — cached fields on aggregates (`Player.total_xp`), fast to read.
- **History** — an append-only transaction ledger; the authoritative record of
  change.
- **Snapshots** — immutable per-day records of what the world looked like.

Phase 1 already demonstrates the pattern in miniature: `health_probe` rows are
append-only and never updated, while the report's `probeRows` is a derived count.

---

## Release status and deferred scope

- **Released:** v1.0.0; Phases 1–13 are complete and the SQLite schema is 17.
- **Supported release scope:** Linux x86_64 on Ubuntu 22.04 (Jammy) or compatible
  newer GTK 3/WebKitGTK 4.1 systems, as documented in the [release notes](V1.0.0-RELEASE-NOTES.md).
- **Deferred beyond v1.0:** cloud synchronization, accounts/authentication,
  multiplayer, mobile support, AI, scheduling/background processing, arbitrary
  scripting, universal graph/event-sourcing rewrites, unrestricted page authoring,
  rich text, and any other feature deliberately listed as deferred in the
  [Domain Design Codex](docs/DOMAIN-DESIGN-CODEX.md).

Phase 12 and Phase 13 reports remain historical validation records; current
scope and safety behavior live in the [Architecture](docs/ARCHITECTURE.md),
[Domain Design Codex](docs/DOMAIN-DESIGN-CODEX.md), and [v1.0.0 release notes](V1.0.0-RELEASE-NOTES.md).

---

## Troubleshooting

**`failed to find a `wry` …` / no `webkit2gtk-4.1` found**
Install the Linux system libraries listed under [prerequisites](#1-prerequisites).

**The app shows a storage or non-persistent-world warning**
Read the safe diagnostic code and recovery guidance on System health. If the
file-backed database cannot be opened, the app may offer a temporary in-memory
world so it remains usable, but a persistent warning stays visible because
changes to that temporary world will not survive a restart. Fix directory
permissions or restore a verified backup before treating new data as durable.
Absolute local paths and raw operating-system error text are not shown.

**"in-memory (not persisted to disk)" appears in the Store row**
The file-backed open failed or this is a test. Do not assume edits are durable;
check storage permissions and the displayed diagnostic code before continuing.

**`npm run dev:vite` shows a connection error**
Expected — a browser tab has no Rust core, and there is no HTTP fallback by
design. Use `npm run dev`.

**Port 1420 already in use**
Vite is configured with `strictPort: true` so it fails loudly instead of
silently moving. Stop the other process.
