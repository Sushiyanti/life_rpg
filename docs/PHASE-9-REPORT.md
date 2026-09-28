# Phase 9 — Timeline in Declarative Workspaces

**Status:** Complete  
**Date:** 2026-09-28  
**Branch:** `phase-9-workspaces-presentation`

## Summary

Phase 9 integrates Timeline as a reusable, read-only Workspace panel while preserving the boundary between presentation settings and canonical world records.

- Added the `timeline` panel source to the closed Workspace registry and rendered it through a bounded Timeline query component.
- Added per-panel category, entity-kind, exact entity-ID, Concept, and inclusive date-range filters. Timeline panels retain their own sort, limit, visibility, collapse, pin, grid-span, and order settings.
- Added Migration 15. It rebuilds and copies the existing Workspace panel rows, preserves prior panel configuration, admits Timeline as a panel type, stores the Timeline filters, constrains the accepted values, enforces ordered date bounds and the entity-kind/identity pairing, and keeps Timeline-only filters off other panel types.
- Extended Timeline coverage with real creation facts for Concepts, Skill Trees, Quest Stages, and Quest Branches. Relationship-history rows now include related Content title, role, attachment/removal timestamps, and explicit navigation to the Content record and target record.
- Added Overview and Review template Timeline panels.
- Updated Workspace transfer to strict format **v3**, with explicit handling of legacy v1/v2 payloads. Exact Timeline identities remain source-local: transfer does not disclose the local entity ID and import warns that the identity must be selected again in the destination world.
- Updated `docs/ARCHITECTURE.md` and `docs/DOMAIN-DESIGN-CODEX.md` with Phase 9 decisions and invariants.

## Verification

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo check --workspace` | Passed |
| `cargo test --workspace` | **116 passed, 0 failed, 0 ignored**; doc-test suites also passed (0 doc tests) |
| `npm run typecheck` | Passed |
| `npm test -- --run` | **111 passed** across 18 test files |
| `npm run build:vite` | Passed |
| `npm run build` | Passed; release executable, Debian package, and AppImage produced |
| `git diff --check` | Passed |

Regression coverage includes Migration 15 upgrade/preservation and constraints, source/category/entity/Concept/date filtering, exact identity selection, relationship context, panel transfer v1/v2/v3 behavior, source-local identity privacy, and Workspace Builder/panel behavior. Date comparisons are checked inclusively at both bounds in the persistence suite.

## Fresh-data native smoke test

The release Tauri executable was launched under Xvfb with an isolated temporary `XDG_DATA_HOME`; no existing user database was opened or modified.

1. The app started and created a fresh SQLite database. Migrations 1–15 applied, with schema version 15.
2. Created a Player through the native UI. Its default Overview Workspace included a Timeline panel.
3. Created a second Workspace from the Review template and switched between it and Overview using the native Workspace selector.
4. Added a second Timeline panel, then removed it through the Builder. The original Timeline panel and source records remained.
5. Hid and restored the original Timeline through the Builder. Reordered it one position and verified its new order.
6. Configured and saved the Timeline category, entity kind, and exact Quest ID. The panel displayed the exact Quest creation fact.
7. Created a Quest and authored Content through the native UI, then attached that Content to the Quest with the Guidance role. The Timeline relationship-history row displayed the Content title and role; expanding it exposed both **Open Content** and **Open target** actions. Each action navigated to the exact corresponding record.
8. Closed and relaunched the app against the same temporary data directory. Schema version, both Workspaces, panel visibility/order, Timeline category/kind/exact ID, and the source rows were retained.
9. Source-row counts after panel addition, removal, hide/restore, reorder, switching, and restart were unchanged: one Player, one Quest, one Content entry, and one Content attachment. The test records themselves were authored deliberately during the smoke test.

The native process emitted non-fatal accessibility-bus and DRI3 warnings from the virtual display. They did not block startup or any of the exercised UI flows. Both Linux bundles were generated successfully; the packaged `.deb` and AppImage were not separately installed/launched.

## Compatibility and invariants

- Migration 15 upgrades schema 14 and preserves existing Workspace panel IDs, filters, sort order, visibility, pin/collapse state, and layout settings.
- Workspace transfer format v3 is current; valid v1/v2 inputs retain explicit upgrade paths.
- Timeline exact entity IDs are local references, not portable world data.
- Workspace configuration is presentation-only; adding, moving, hiding, or removing panels does not create or delete world facts.
- Timeline remains a read-only projection over canonical records, not a second event store.

Phase 8 and earlier branches were not modified. This phase is based on the Phase 8 final commit and is isolated on `phase-9-workspaces-presentation`.
