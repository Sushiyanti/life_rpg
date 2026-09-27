# Phase 5.2 — Workspace Transfer Integrity

**Branch:** `phase-5.2-workspace-transfer-integrity` (from `phase-5.1-workspace-capabilities`)
**Implementation commit:** `c7096d07627a78ccfb6646a54e3879e54ce65d9c`
**Schema:** 11 (forward-only migration `0011_phase52_concept_transfer_keys.sql`)
**Boundary:** Transfers declarative workspace presentation only; they do not export or clone Player/world records.

## Transfer contract and versioning

The current format is strict JSON with `format: "life-rpg-workspace"` and `version: 2`. It includes the workspace name/template and an ordered list of explicit panel presentation values and bounded filters. Concept-based filters use a `relatedConcept` reference with a stable Concept-only `key`, human-readable `name`, and `typeCode`. Transfer files contain no Player IDs, workspace/panel database IDs, Concept local IDs, timestamps, executable content, or canonical world records.

Exports are reconstructed from a fixed allowlist, validated before return, and sorted by panel order with a locale-independent stable tie-break. Re-exporting unchanged workspace configuration therefore produces semantically deterministic JSON. Runtime selection and local storage are not included.

Version 1 remains supported through a deliberate migration path. It discards every imported `filterConceptId` without looking up or trusting it, reports affected panels as unresolved, and normalizes the historical Quest (`in_progress` → `active`, `pending` → `open`) and Activity (`active` → `in_progress`) status aliases before current validation. Unknown fields, malformed configurations, unsupported formats, and unsupported versions fail closed; no imported strings are executed as code or SQL.

## Concept identity and reference resolution

Migration 11 adds an immutable, unique `transfer_key` to Concepts. Existing Concepts are backfilled; new Concepts receive a UUID-backed `concept-ref-v1-…` key. This is deliberately limited to Concepts needed by workspace filters—not a universal entity identity, sync, or merge system.

- **Same world:** A unique exact key and matching type resolve to the already-existing destination Concept, preserving the Concept filter without duplication.
- **Different world:** Independently created Concepts have different keys. Matching type/name entries are suggestions only and require an explicit selection; the application never silently selects a same-name Concept.
- **Unresolved or ambiguous:** The preview shows the unresolved reference and candidates. The player may explicitly choose a destination Concept or leave the panel unfiltered. Import feedback reports how many references resolved and how many were left unfiltered.
- **Forged/local IDs:** The file cannot supply a destination database ID in v2. The only IDs submitted after preview are from the active Player's Concept list; the application service and SQLite store recheck ownership. Legacy v1 IDs are discarded.

## Preview, validation, ownership and atomicity

The Workspace Builder validates the selected file before showing an import preview, displays the workspace name/panel count and every Concept resolution state, and waits for an explicit “Import reviewed workspace” action. A successful action calls one typed native `import_workspace` command. The application validates the workspace/panels and destination Player ownership; SQLite revalidates panel structure, workspace ownership, and each Concept owner inside one transaction. Any failure rolls back the workspace and all panels. An insert-time duplicate-key failure is tested after earlier rows have been inserted, confirming actual transaction rollback.

Validation covers the exact format/version and field sets, workspace name/template, maximum 100 panels, registered panel sources/variants/sorts/statuses, filter capability/value, type-code syntax, Concept reference shape, density, booleans, result limit (1–50), recent window (1–365 days), title length, sort order (0–999), and grid span (1–2). Unknown and executable-like fields are rejected. Duplicate workspace names are rejected by the existing uniqueness constraint; no partial state remains, and the user can retry with a different name.

Import creates only a Workspace and its Workspace Panels. It does not create Concepts, Quests, Skills, Sessions, Transactions, History, or other world entities.

## Migration and persistence verification

Migration 11 is appended after migrations 1–10; no applied migration was edited. Tests verify a fresh store reaches schema 11, migration ledger/idempotency behavior, migration 10→11 upgrades and backfills unique keys, new Concept key assignment, key immutability, and workspace/Concept key persistence across a database reopen.

## Verification results

All checks below passed on the final implementation commit:

- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo test --workspace` — **94 tests passed** (3 Tauri, 13 application, 11 contracts, 18 domain, 49 persistence; no failures).
- `npm run typecheck`
- `npm test` — **65 tests passed across 9 files**, including deterministic export, same-world and cross-world resolution, v1 normalization/ID discard, malformed/forged input, atomic IPC payload, preview-before-confirm, and unresolved-filter UI feedback.
- `npm run build:vite` — production build passed.
- `git diff --check` — passed before commit.
- Native smoke test — built `life-rpg` and launched it under Xvfb with isolated temporary data; it remained open until the 15-second smoke-test timeout. Only non-fatal accessibility-bus and software-rendering warnings were emitted.

## Known limitations and deferred work

- A Concept created independently in another world will not have the same random stable key. Cross-world matches therefore require explicit user resolution; names are never treated as globally unique.
- If no Concept is selected, the filter is deliberately neutralized. The semantic reference is not stored as a dangling panel filter after import.
- Version 1 files cannot preserve Concept-filter meaning because their IDs are local and untrusted; affected filters are reported and left unfiltered.
- A duplicate workspace name is rejected rather than silently renamed.
- Workspace transfer does not clone world state, implement cloud sync, merge worlds, or establish global identity for Quests/Skills/other entities. Phase 6 and broader workspace redesign remain out of scope.
