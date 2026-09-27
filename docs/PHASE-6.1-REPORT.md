# Phase 6.1 — Effect Lifecycle & Hub Corrections

**Date:** 2026-09-28  
**Branch:** `phase-6.1-effect-lifecycle`  
**Schema:** 12

## Summary

Phase 6.1 completes the Effect lifecycle and Player Hub corrections started after Phase 6. Effects can now be indefinite or have a recorded expiry; lifecycle status is derived for display without time-driven database writes; edits, manual deactivation, and Session associations are recorded in durable append-only history; and Sessions can be explicitly associated with Effects using typed roles. The work remains local-first and offline-only, with no background process or remote service added.

## Effect lifecycle semantics

- `expires_at = NULL` means **no expiry**. An Effect remains active until its owner explicitly records manual deactivation or edits its details.
- A stored expiry is a recorded timestamp, not a scheduled database mutation. Passing that timestamp never writes `deactivated_at`, inserts a lifecycle event, or changes the Effect row.
- The display lifecycle is derived from the recorded start, expiry, and manual-deactivation timestamps at read/render time:
  - `scheduled` while a valid `started_at` lies in the future;
  - otherwise `active` while no applicable expiry or manual-off fact has taken effect;
  - `expired` when the recorded expiry has passed;
  - `manually_deactivated` when an explicit manual-off timestamp has taken effect.
- If both the expiry and manual-off timestamps have taken effect, the earlier recorded fact determines the displayed state; expiry wins an exact tie. A future manual-off timestamp does not make the Effect inactive early.
- The shared frontend lifecycle helper is display-only. Rust domain/application validation remains authoritative for Effect writes.

## History and persistence

Migration 0012 adds `effect_history` and `session_effects` to schema version 12. Effect history entries record event kind, time, optional Session context, and explicit current and (where applicable) previous JSON snapshots. Supported event kinds are `created`, `details_changed`, `expiry_changed`, `manually_deactivated`, `session_linked`, and `session_unlinked`.

The persistence operations insert/update Effect state, history, and any Session relationship in a single transaction. History is append-only; expiry itself does not create an event simply because time passed. Explicit changes to an Effect’s expiry are recorded separately from detail changes. Reads do not synthesize history or mutate Effects.

## Effect ↔ Session integration

`SessionEffect` is a typed relationship, separate from either source record. Its roles are `relevant`, `applied`, `removed`, and `observed`; each link stores its Player, Session, Effect, role, `added_at`, and optional `removed_at`. Migration constraints/triggers and service checks keep the relationship within one Player world.

Relationships are never inferred from time, Quest context, or Effect state. The user explicitly selects a Session when creating/linking an Effect or choosing Session context for a manual deactivation. Removing a relationship only records `removed_at` and a `session_unlinked` snapshot; it does not delete or deactivate the Effect or alter the Session. Re-adding a previously removed role revives the unique current relationship row, while its earlier link/unlink facts remain in history. Finishing a Session preserves its links.

The Effects manager supports create, edit, manual deactivation, optional Session context, and history inspection. The Session detail panel supports explicit existing-Effect links, role choice, link removal, and atomic creation as `applied`.

## Hub and integration corrections

- Removed the internal `3.6 · world semantics` branding from the visible footer; the app now uses neutral product branding.
- Replaced the Learning template’s “Skills in practice” panel label with “Skills tracked”; the active Skill dashboard metric also describes tracked records instead of implying a practice metric that is not measured.
- Session context presentation resolves recorded Quest, Stage, Branch, Skill, and Concept anchors by their names when available; Stage/Branch-only context no longer falls through to an inaccurate empty-Quest message.
- Corrected Quest/Session status literal mismatches and bounded Explorer search sizes to the native maximum (200), avoiding invalid startup/search IPC calls.
- Aligned the JavaScript Tauri API package to exact `@tauri-apps/api@2.11.1`, matching the Rust Tauri `2.11` minor. No other dependency or package-lock entries were changed for this alignment.

## Verification

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace` | Passed — 96 Rust unit/integration tests across the workspace; doc-test targets also completed |
| Effect close/reopen persistence regression | Passed — file-backed SQLite is closed and reopened; no-expiry state, manual deactivation, typed Session links, and lifecycle history remain present |
| `npm run typecheck` | Passed |
| Full frontend suite | Passed — 94 tests across 15 files |
| `npm run build:vite` | Passed — production frontend bundle built |
| `git diff --check` and targeted UI wording audit | Passed |
| Native Tauri under Xvfb | Launched successfully. Created a temporary Player, Quest, active Session, and indefinite Effect through the UI with an explicit `applied` Session relationship. Confirmed `created` and `session_linked` snapshots in the local database, restarted the native app with the same isolated data directory, and verified the Effect/link/history remained visible and persisted. |

## Known limitations

Synthetic Xvfb input did not reliably complete the WebKit native `datetime-local` picker or trigger the Effect-card manual-deactivation button during the interactive walkthrough. Thus finite-expiry creation and manual deactivation were **not** verified through native UI clicks in that walkthrough. They are covered by the Rust lifecycle/service/storage tests, including a file-backed close/reopen regression, and the frontend tests cover the optional-expiry controls and explicit manual-deactivation behavior.

The first native launch emitted a package-version mismatch warning. After aligning `@tauri-apps/api` with the Rust Tauri 2.11 minor, the native app was launched again under Xvfb without that warning and reopened the persisted Player Hub and Effect.

`npm install` reported five package audit advisories in the existing dependency graph (three moderate, one high, one critical). No unrelated dependency upgrades or audit-fix operations were performed in this phase.

## References

- [Tauri dependency updates and npm/Cargo minor-version synchronization](https://v2.tauri.app/develop/updating-dependencies/)
- [Published `@tauri-apps/api@2.11.1` package metadata](https://www.npmjs.com/package/@tauri-apps/api/v/2.11.1)
