# Phase 6.1.1 — Final Effect Lifecycle Correction

**Date:** 2026-09-28  
**Branch:** `phase-6.1.1-effect-lifecycle-final`  
**Base commit:** `2dab9aad1b32cf02d920953446f3c27604114b12`  
**Schema:** 12 (the checked-out base contains Phase 6.1 Migration 0012)

## Scope and decision

This is a narrow correctness pass over Phase 6.1. It does not redesign Effects, Sessions, or history. The supplied starting-point note said schema 11, but the authoritative branch/commit already contains Migration 0012 and schema version 12; this work retains that actual repository state, does not edit migrations, and adds no migration.

Manual deactivation is accepted only while the Effect is active at the explicit operation time. The domain rejects an Effect whose `expires_at <= now`, including an exact tie. The application service delegates to that domain invariant before writing, so a rejected request does not set `deactivated_at`, add a `manually_deactivated` event, or record a Session link for the attempted action. A scheduled Effect also remains non-deactivatable before its start. Indefinite Effects and Effects whose expiry is still in the future remain explicitly manageable.

## Lifecycle and expiry editing

- `expires_at = NULL` means no recorded expiry. After its start, the Effect remains active until an explicit manual-off action or a deliberate authored edit changes its facts.
- `expires_at <= current time` derives `expired`; time passing is observational and creates no database write or synthetic `expired` history event.
- The Player Hub and Effects manager show lifecycle state and offer manual deactivation only for currently active Effects. Expired records remain visible/editable and their history remains available.
- Descriptive edits to an expired Effect preserve its expiry and it remains expired.
- The Player may deliberately extend an expired Effect's expiry into the future or clear it to `NULL`. The existing `expiry_changed` event stores before/after snapshots. The resulting lifecycle is derived from the edited facts (and can become active); there is no synthetic `reactivated` event. The Effects manager explains this before the edit is saved.
- An Effect manually deactivated while still active remains distinguishable from later expiry. Scheduled Effects cannot be manually deactivated before their start.

## History and Session relationships

Effect history remains append-only for authored actions: creation, details changes, expiry changes, manual deactivation, and Session link/unlink. Rejected deactivation after expiry writes no event. Expiry itself never synthesizes history. Existing explicit typed `SessionEffect` relationships are not changed by expiry, editing, or manual deactivation; they remain inspectable as historical context until the Player explicitly removes the relationship. The expired-Effect regression verifies its Session link remains available after close/reopen.

## Implementation

- Added the expired-expiry guard to `Effect::deactivate`, making it authoritative for all callers, including Tauri IPC.
- Gated manual-deactivation actions in both `EffectsScreen` and the Player Hub on the derived `active` state; other lifecycle labels, Effect history and Session links remain visible.
- Kept descriptive editing for expired Effects and added an explanatory note for intentional expiry edits.
- Extended domain and file-backed application/persistence coverage for expired rejection/no history, future-expiry and indefinite manual-off, expiry edits/history snapshots, explicit Session retention, and lifecycle/history behavior after reopening SQLite.
- Extended UI tests for lifecycle/action visibility, editing guidance, and an expired Effect's visible Session link.

## Verification

| Check | Result |
|---|---|
| Unchanged baseline before edits: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace` | **Passed** (recorded before changes) |
| Unchanged baseline before edits: `npm run typecheck`, `npm test`, `npm run build:vite` | **Passed** — 94 frontend tests across 15 files |
| Focused domain/persistence regressions | **Passed** — domain expiry rejection and file-backed lifecycle/Session/history close-reopen case |
| Focused UI regressions | **Passed** — Effects manager, Player Hub, Session-effects panel (20 tests across 3 files) |
| Final `cargo fmt --all -- --check` | **Passed** |
| Final `cargo check --workspace` | **Passed** |
| Final `cargo test --workspace` | **Passed** — 97 Rust tests; all doc-test targets completed |
| Final `npm run typecheck` | **Passed** |
| Final `npm test` | **Passed** — 96 tests across 15 files |
| Final `npm run build:vite` | **Passed** — production bundle built |
| `git diff --check`; migration scope audit | **Passed** — no migration/schema file changed |
| Fresh native Tauri launch | **Passed** — fresh local database applied migrations 0012/schema 12; UI rendered and a temporary Player plus indefinite Effects were created through the native UI |
| Native database reopen | **Passed** — reopening the same isolated app-data directory preserved the Player, indefinite Effect rows, and their `created` history; the file-backed Rust close/reopen test additionally verifies manual deactivation, history, and Session links |

## Known limitations

Automated tests are the reliable verification for native lifecycle actions. In this Xvfb/WebKit run, the native Effect form was opened and the Never/no-expiry Effect was created and shown as active, but synthetic interaction did not reliably complete the platform date picker for future/past expiry, activate the manual-deactivation card control, or complete the Session-link workflow. Those interaction paths were therefore not claimed as native-UI-verified. Rust domain/service/storage tests verify expired deactivation rejection with no new event, successful manual-off for future and indefinite Effects, intentional expiry edits and history, Session relationship preservation, and close/reopen persistence; frontend tests verify visible action gating, edit guidance and expired-link presentation.

No schema migration, scheduler, polling, background daemon, or remote service was added. Phase 7 was not started.
