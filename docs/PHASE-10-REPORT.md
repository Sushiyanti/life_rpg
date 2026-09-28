# Phase 10 — Explicit Gameplay Rules & Progression Automation

**Status:** Complete
**Date:** 2026-09-28
**Branch:** `phase-10-gameplay-rules-progression`
**Base:** Phase 9 commit `55b9b0bd75337f3c45ac8b36779caa64eeb66161`

## Summary

Phase 10 extends the existing closed, declarative Rule system to explicit Skill progression and immediate Effect operations. Player-owned canonical state remains authoritative: automation only performs bounded actions explicitly enabled by the Player, and every root change and successful follow-on Rule operation is committed as one world mutation.

## Skill progression and authority

- Skill XP is a nonnegative whole-number measure independent of Player XP, invested time, lifecycle, and manually authored Skill level. Signed adjustments preserve the zero floor and record requested/applied delta in an immutable `skill_xp` Transaction with Skill identity, before/after values, source, reason, and timestamps.
- XP changes never alter `level`, `level_name`, or `progression_label`. Skill levels remain manually controlled; there is no XP-to-level formula or automatic leveling.
- Availability (`locked`/`available`) is independent of lifecycle (`active`/`paused`/`completed`/`archived`). Per-Skill authority is `manual` or `rule_controlled`. A Player's explicit lock/unlock reclaims manual authority. A Rule can unlock only a Skill explicitly delegated to Rule control; a manually controlled lock aborts the chain. Unlocking an already available Skill is a deterministic successful no-op without duplicate history or event.
- Append-only `skill_history` records real availability and authority transitions with previous/current state and source. XP changes remain in the Transaction ledger rather than being duplicated there. Skill hierarchy is not a prerequisite graph.

## Rule vocabulary and execution

Phase 10 adds closed typed triggers for `skill_xp_changed`, `skill_unlocked`, `session_started`, `session_finished`, `effect_created`, and `effect_deactivated`. Conditions read only the validated fields available on the matching event; they do not inspect arbitrary Skill, Session, or Effect JSON or query arbitrary live state. Session events come from explicit Player commands and use only recorded Session values; no missing outcome or duration is inferred.

New bounded actions are:

- `award_skill_xp`: adjust one owned Skill by a nonzero signed delta, with an optional reason.
- `unlock_skill`: unlock one owned Skill only when its Rule authority permits the transition.
- `apply_effect`: create a normal Player-owned Effect from a registered type, with an optional same-Player Concept target and bounded expiry.
- `deactivate_effect`: explicitly deactivate one actionable Effect through the Rule-specific lifecycle path.

Rule-created Effects use the application identity and Clock, persist ordinary Effect creation history, and do not invent a Session relationship. Effect deactivation provenance distinguishes Player `manually_deactivated`, Rule `rule_deactivated`, and derived `expired`. Expiration is read-derived: it writes no history, emits no Rule event, and cannot be explicitly deactivated after expiry. Scheduled Effects remain protected where current semantics do not allow explicit deactivation.

The existing deterministic limits and order remain in force: depth 8, 16 actions per Rule, 32 actions and 64 evaluations per chain, condition bounds, priority-descending/stable-ID Rule ordering, FIFO follow-on events, and repeated-event detection. No scheduler, daemon, clock-triggered Rule, arbitrary code, HTTP action, or synthetic activity was introduced.

## Atomicity, audit, and Timeline

Root operations, Skill XP Transactions, availability changes, Effect lifecycle history, follow-on events, and successful execution audit are planned and persisted through the existing `WorldStore::apply_rule_chain` transaction. A later action failure rolls back the root and every derived state change; failure audit follows the existing separate rollback policy. A manually controlled Skill lock and an XP self-trigger cycle are covered by persistence tests demonstrating rollback/guard behavior.

The existing Timeline projection now composes persisted Skill XP Transactions, real Skill availability history, and Rule-caused Effect lifecycle facts. These remain projections over canonical records, not a new event store. Derived Effect expiration and synthetic events are excluded.

## Migration and compatibility

Migration **16** adds Skill availability/control with backward-compatible defaults (`available`/`manual`), append-only Skill history, Effect deactivation provenance, and the schema constraints needed for the new closed Rule vocabulary. Historical explicit Effect deactivations are attributed to manual provenance. The shared Transaction table is rebuilt losslessly to allow `applied_amount` for both Player XP and Skill XP while preserving prior rows, IDs, timestamps, indexes, search rows, Concept links, and integrity guards.

The schema-15 upgrade test verifies preservation of Skills and their levels/XP, Effects and lifecycle history, legacy Rule definitions and execution audit, and the expected defaults. New gameplay Rule event kinds are accepted after migration. Fresh schema creation and migration integrity are also exercised by the persistence suite.

## Verification

All required checks completed successfully:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo check --workspace` | Passed |
| `cargo test --workspace` | **124 passed, 0 failed, 0 ignored**; doc-test suites passed (0 doc tests) |
| `npm run typecheck` | Passed |
| `npm test` | **113 passed** across 18 test files |
| `npm run build:vite` | Passed |
| `npm run build` | Passed; release executable, Debian package, and AppImage produced |
| `git diff --check` | Passed |

Focused coverage includes Skill XP bounds and level independence, manual versus Rule unlock authority, XP→unlock→Effect chains, failure rollback for manually controlled locks, XP-cycle depth protection, explicit Session-finish Effect deactivation, Timeline projections, and the schema-15-to-16 compatibility path.

## Native smoke test

The release app was run under Xvfb using a fresh isolated `XDG_DATA_HOME`; no pre-existing user database was opened. In the native UI, two Player-authored Rules were saved: `skill_xp_changed` unlocks a specific Rule-authorized Skill at the threshold, then `skill_unlocked` for that Skill creates the `Unlocked Focus` Buff.

A real manual +10 Skill XP action triggered the chain. SQLite verification showed Skill XP 10, availability `available` with Rule control, the manually authored level still 1, one Skill XP Transaction, the Rule-sourced Buff, and two successful audited Rule executions at depths 0 and 1. The app was closed and restarted against the same isolated data directory; Skill XP, availability, Effect, Transaction, Skill history, and Rule audit remained present without duplicate synthetic records. Automated Timeline tests verify that its rows reflect those persisted facts.

## Intentionally deferred

Automatic Skill leveling, XP-to-level formulas, inferred Skill prerequisites/graphs, scheduled or time-triggered Rules, Effect-expiration events, synthetic activity, executable/user-authored code, arbitrary SQL/HTTP automation, cloud automation, and Phase 11 work remain out of scope. Skill tree parentage does not imply a prerequisite.

## Git checkpoint

The Phase 10 branch is based on the Phase 9 final commit listed above. The Phase 9 branch and earlier phase branches were not modified. No Phase 11 work was started.
