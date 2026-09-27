# Phase 7 — Reusable Content & Guidance Report

**Baseline:** `phase-6.1.1-effect-lifecycle-final` (`d153dba240f2ceecf45607813f844f7725223a4c`)  
**Implementation branch:** `phase-7-content-guidance`  
**Schema version:** **13** — `0013_phase7_content_guidance`

## Outcome

Phase 7 makes reusable authored content a first-class, Player-owned world capability without adding a CMS, Page aggregate, generic graph, scheduler, rich-text system, or automatic progression behavior.

The established `NarrativeEntry` aggregate is the canonical Content record. One content entry can be explicitly attached to many supported same-world records; the content body is never copied merely to use it in more than one context.

## Domain semantics

| Concern | Final behavior |
|---|---|
| Canonical record | `NarrativeEntry` remains the persisted/content identity. It carries Player ownership, data-defined kind, title, plain-text body, optional author/source fields, metadata, timestamps, and `is_active`. |
| Content types | Types remain `type_definitions` rows under `narrative_entry`, not a Rust enum. Existing types are preserved; migration 13 adds `introduction`. Existing `note`, `guide`, `guidance`, `briefing`, `story`, `lore`, `instruction`, `reading`, `todo`, `reference`, `summary`, `reminder`, `reflection`, and `journal` remain valid. |
| Content edit | `NarrativeEntry::edit` validates the content namespace, nonblank bounded title/body, and paired source fields. Updates use the existing before-image revision and FTS triggers. |
| Content lifecycle | Content uses the existing active/archive/trash/recovery model. Lifecycle, an `is_active` content field, presentation visibility, and a target’s lifecycle remain separate axes. |
| Editor | The Guidebook uses plain text with preserved line breaks. No stored HTML, JavaScript, custom CSS, iframe, rich-text, or markdown execution was added. |

## Explicit relationship semantics

`content_attachments` now represents a historical **Content relationship fact**, rather than a mutable current-only join:

| Structured field | Meaning |
|---|---|
| `id` | Stable relationship identity. |
| `content_id`, `player_id` | Canonical content and owning Player world. |
| `target_kind`, `target_id` | Closed supported target: Player, Quest, Quest Stage, Quest Branch, Session, Skill, Skill Tree, Concept, or Effect. |
| `role_code` | Data-defined bounded usage vocabulary such as `guidance`, `about`, `instruction`, `reference`, `context`, `introduction`, `result`, `summary`, and `reminder`. |
| `sort_order` | Deterministic per-target display ordering only; it does not change world semantics. |
| `created_at`, `removed_at`, `updated_at` | Timestamped relationship history. Removal retains the fact and does not delete the Content or target. |

The application service resolves both Content and target in the Player world before writing. SQLite independently checks the same ownership/existence invariant on inserts and relevant updates. A partial unique index permits at most one active fact for the same content/target/role; removing it makes a later attachment a new historical fact rather than overwriting the prior removal. If a legacy physical target deletion occurs, an active polymorphic relationship is timestamped removed rather than erased; ordinary archive/trash and Effect lifecycle transitions preserve links.

## Migration 13 and compatibility

`0013_phase7_content_guidance.sql` is a single forward-only migration. It:

1. Adds the one missing `introduction` content type and additive relationship roles.
2. Rebuilds the legacy current-only attachment table into timestamped relationship facts.
3. Preserves every legacy relationship’s content, Player, target, role, creation and update timestamps. Active legacy rows remain active; legacy inactive rows map their existing `updated_at` to `removed_at` rather than inventing a timestamp.
4. Adds active-relationship uniqueness, reverse/content and target lookup indexes, ownership update guards, and polymorphic target-removal triggers that preserve historical rows.

No applied migration was modified. Existing Narrative Entry records stay readable, searchable, and attachable. Existing `source_kind`/`source_id` fields remain **provenance**, not implicit relationships; they are not silently converted into targets because that would conflate a source reference with the explicit association semantics introduced here.

## Search, history, and recovery

- The existing single-row Narrative FTS projection remains authoritative. Content title/body/type, Player, lifecycle, active state, timestamps, explicit Concept attachment, and optional associated target kind are queryable.
- Search uses `EXISTS` against active content relationships, so reused Content yields one typed hit rather than one FTS document per attachment.
- Active Content→Concept relationships participate in explicit Concept filtering.
- Editing Content performs the normal Narrative update, producing an existing before-image revision and synchronizing FTS. Attachment creation/removal has its own timestamped relationship history and does not manufacture Narrative revisions.
- Trashing/recovering Content is still an existing lifecycle mutation; it neither alters targets nor removes relationship facts. New attachments to archived/trashed Content are rejected.

## UI and IPC

### Content Guidebook

The former Journal route now opens a dedicated **Content Guidebook**. It provides:

- browse, plain-text search, and type filtering;
- create, inspect, and edit content including optional author/source fields;
- lifecycle/history navigation through the existing Explorer/recovery surface;
- relationship history inspection, including removed facts;
- attaching the selected canonical Content to Player, Quest, Skill Tree, Skill, Concept, or Effect targets; and
- timestamped relationship removal without deleting either side.

World Explorer retains contextual attachment panels for Player, Quest, Stage, Branch, Session, Skill, Skill Tree, Concept, and Effect details. It exposes active attached Content and relationship removal. Explorer also accepts a content target-kind filter. The typed `CoreClient` and Tauri commands cover Content create/get/list/update plus attach/detach/list-for-target/list-for-content/list-role intents; React never accesses SQLite.

## Focused tests added

- Domain validation of Content edits and source pair invariants.
- Migration v12→v13 upgrade preservation, legacy inactive timestamp mapping, new type seed, same-world direct-SQL update rejection, and target-removal history preservation.
- Persistence/service coverage for one Content record attached to Quest, Session, Concept, and Effect; edits that produce a Narrative revision and FTS update; cross-Player/missing-target rejection; removal timestamps; reattach-as-new-fact; Effect lifecycle preservation; and content/relationship durability across a file-store reopen.
- Frontend Guidebook browse/filter/inspect/lifecycle navigation/create/attach/detach behavior.
- Typed IPC command and payload tests for Content management and relationships.

## Verification

Completed after the final implementation update:

```bash
cargo fmt --all -- --check        # passed
cargo check --workspace           # passed
cargo test --workspace            # passed: 102 Rust tests (plus doc tests)
npm run typecheck                 # passed
npm test                          # passed: 99 frontend tests across 16 files
npm run build:vite                # passed
npm run build                     # passed: Linux .deb and AppImage bundles
```

A fresh-data native smoke test launched the final `target/release/life-rpg` under `xvfb-run` for 10 seconds. It remained running until the expected test timeout, created a new database, applied schema version `13` / `0013_phase7_content_guidance`, and created the `content_attachments` table. The production Tauri bundle also completed successfully, yielding the Linux `.deb` and AppImage.

The headless WebKit environment did not drive visible Guidebook controls end-to-end; this report does **not** claim GUI click automation for that path. The create/edit/attach/detach/reopen/search/recovery behavior is instead covered by the typed Rust persistence/service tests and the React Guidebook tests above.

## Intentional limitations / preserved decisions

- No rich text/markdown/HTML execution, attachment files, tags, pages, generic graph, cloud sync, account, HTTP API, scheduling, notification, AI generation, inferred activity, or automatic progression.
- No automatic propagation from Quest/Skill/Concept Content to Sessions; every Session relationship is explicit.
- Content attached to an Effect is explanatory/contextual only. Effect expiry, activation, deactivation, or editing never creates/removes content links.
- `sort_order` is present for deterministic presentation but no page-builder/reordering UI was introduced.
- Existing temporary `is_active` Content state remains distinct from archive/trash and contextual visibility; no permanent Content deletion is introduced.
