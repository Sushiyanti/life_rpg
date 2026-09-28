# Phase 8 Report — Unified Read-Only Timeline

- **Status:** Complete
- **Branch:** `phase-8-timeline`
- **Base:** `phase-7-content-guidance-final` (`7d34989`)
- **Scope:** First-class chronological view over existing Player-owned world and history records.

## Purpose and invariant

The Timeline answers “what is already recorded in this Player world, and when?” It is a **read-only chronological projection**, not a new canonical event store. It preserves source identity and the meaning of each persisted timestamp. Reading, filtering, paging, and navigating the Timeline do not write or backfill world data.

It remains distinct from both the Player Hub’s concise activity panel (which now reuses the same bounded query) and **History & recovery** (which continues to own revisions, snapshots, and recovery workflows). It does not infer activity from elapsed time, prose, current state, expiry, scheduled dates, or missing days.

## Persisted sources and timestamp semantics

All supported categories and entity kinds are closed typed vocabularies shared across the application, Rust IPC contracts, Tauri, and TypeScript. Each result includes a stable source ID, Player ID, category, exact source/entity identity, primary timestamp and semantic label, optional secondary timestamp and label, title/summary, and only explicitly supported Concept/type/state context.

| Category | Existing source | Primary timestamp and treatment |
|---|---|---|
| Record changes | Player, Quest, and Skill records | Only persisted creation, Quest/Skill start, and Quest/Skill completion times are projected; no timestamp is supplied for an unrecorded transition. |
| Sessions | Quest Sessions | One item per Session, ordered and date-filtered by persisted `started_at` (“Started”); persisted `ended_at`, when present, is optional “Ended” detail on that same item. It is not a second completion event. |
| Transactions | Player Transactions | Persisted `occurred_at` is primary. `captured_at` is optional detail; legacy `NULL` capture times remain unknown rather than being invented. |
| Effect history | Effect history rows | Each explicit history row uses `recorded_at`. Expiry or passage of time never synthesizes an Effect history item. |
| Content | Canonical NarrativeEntry/Content | A Content record can contribute its persisted creation and, if later, update fact. Reusing/attaching Content does not duplicate these items. |
| Comments | Authored comments | Uses `created_at` and navigates to the exact supported target, rather than treating the comment as a copy of that target. |
| Concept progress | Concept progress history | `occurred_at` is primary; persisted `captured_at` remains separately labeled detail. |
| Revisions | Entity revisions | Uses the revision’s `recorded_at`; it remains distinct from the current entity record. |
| Snapshots | Player, Skill, and Concept snapshots | Uses the source snapshot’s persisted creation/capture timestamp and keeps snapshot state separate from events. |
| Lifecycle | Lifecycle audit facts | Uses `occurred_at`; optional `captured_at` is secondary detail, and before/after state remains explicit. |
| Relationship history | Content attachment facts | Explicit `created_at` and non-null `removed_at` facts are separate from Content records and lifecycle state. Removed links remain history, not active links. |

Concept context is included only through persisted explicit Concept relationships, associations, targets, or content attachments—not by searching narrative text. Player ownership is mandatory for every query, including comments and related records.

## Query, ordering, and performance

`TimelineService` is a dedicated application query over the `TimelineStore` port. The SQLite adapter composes existing source tables with `UNION ALL`; it does not copy them into a presentation table. Player ownership, primary-timestamp range predicates, and selected category branches are applied in SQL. Optional entity-kind and explicit Concept filters are also bounded and typed.

The query accepts inclusive `from`/`through` bounds, newest/oldest order, and offset pagination. Validation rejects reversed ranges, zero or greater-than-200 limits, and offsets above 100,000. The UI requests pages of 50. Sorting is by primary timestamp in the requested direction, then category, stable source ID, entity kind, and entity ID in binary ascending order to make ties and pages deterministic.

Forward migration **0014** adds 14 timestamp indexes over the existing source tables for Player/entity-scoped chronological scans. Migration 13 is unchanged. Migration 14 creates no table, copied timeline state, backfill, or event log. Tests verify fresh schema creation, upgrade from schema 13 with existing rows preserved, required index presence, and use of the Effect player/time index in a query plan.

## UI behavior

- Adds a dedicated Timeline navigation route; Explorer and History & recovery remain separate.
- Groups items by their primary timestamp’s local calendar date and supports category, entity kind, Concept, inclusive date range, newest/oldest, and bounded “Load more” controls.
- Labels the timestamp meaning on each item; Session start and end are explicitly shown as “Started” and “Ended”.
- Keeps source identity available on the exact-record navigation control. Long summaries can be expanded without losing access to the source link.
- Announces loading and errors; an empty result states that no recorded events match the range rather than claiming that nothing happened.
- The Player Hub keeps its contextual short activity list and existing visibility preferences, but now uses the same bounded server-side query. “View full timeline” opens the new route; History & recovery remains available as before.
- Timeline source navigation delegates to the existing Player-scoped Explorer/detail route. No parallel details or mutation path was added.

## Verification

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo check --workspace` | Pass |
| `cargo test --workspace` | **114 passed, 0 failed** (3 Tauri library, 17 application, 15 contracts, 20 domain, 59 persistence) |
| `npm run typecheck` | Pass |
| `npm test` | **112 passed, 0 failed** across 17 test files |
| `npm run build:vite` | Pass |
| `npm run build` | Pass; native release plus Debian and AppImage bundles produced |

Coverage includes closed DTO/timestamp semantics, query validation and bounds, deterministic ordering and tie-breaking, inclusive ranges, category/entity/Concept filters, Player isolation, real multi-source SQL projection, pagination, legacy Transactions with null capture time, no synthesized Effect expiry events, Content reuse, distinct relationship history, and read-only behavior. IPC tests cover command spelling, request/response typing, filters, and pagination. UI tests cover loading/error/empty states, chronology/date groups, filters, navigation, long content, pagination, and the Player Hub full-Timeline link.

### Native smoke

Ran the release application in an actual Xvfb-displayed Tauri window against a **fresh disposable XDG data directory**. Fresh startup applied schema 14 and the native UI created a Player. The isolated database was then populated with supported persisted fixture rows (including Sessions, Transactions, Effect history, Content, Comment, and Concept progress), and the application was reopened against that same database.

The reopened app displayed multiple categories and the single-row Session start/end presentation. Native mouse interaction verified the Transactions category filter, a future one-day date range and its honest empty state, and navigation from a specific Session item to the exact Session detail in World Explorer. A second-world Transaction existed but did not appear in the active Player’s filtered Timeline. Before and after native viewing/filtering/navigation, canonical source row counts were unchanged; schema version remained 14 and no `timeline_events` table existed. The smoke database lived only under `/tmp/phase8-native-smoke` and is not repository or user data. Headless Xvfb emitted accessibility-bus/GPU-acceleration warnings, but the app window rendered and the interactions above completed.

## Migration and scope

The only schema change is the forward-only index migration 0014; no data is copied or transformed. Migration 13 remains untouched. The Timeline does not add a generic event-sourcing system, automatic activity inference, AI history, notifications, scheduling, background polling, cloud sync, accounts, rich text, arbitrary pages, new Effect automation, automatic Session creation, automatic progression, or XP-to-level conversion. Phase 9 is not started.

## Known limitations

- Only timestamped facts currently persisted by the application can appear; current-state-only data and genuinely missing timestamps remain absent.
- Date ranges apply to each item’s primary timestamp. A Session is ranged by its persisted start; its persisted end is detail on the same item.
- The local-first Timeline is Player-scoped and bounded to 200 results per page with a validated maximum offset; it is not a global or unbounded audit export.
- Exact-record destination capabilities remain those of the existing Explorer/detail surfaces.

> **Final invariant:** The Timeline is a read-oriented chronological projection of real persisted world/history records. It never invents activity, mutates canonical world state, or replaces the specialized history and current-state models underneath it.
