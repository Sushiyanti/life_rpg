# Phase 3 report — Declarative rule engine

## Outcome

Phase 3 adds a first, controlled event-driven rule engine to the local-first Life RPG world. Rules are versioned **data**, evaluated by trusted Rust application code. User/database-provided executable code is neither accepted nor run: there is no expression evaluator, scripting interface, `eval`, plugin hook, shell invocation, or scheduler.

The implementation preserves the established layers:

```text
React / typed TypeScript IPC
        ↓
Tauri command adapter (thin)
        ↓
WorldService + trusted RuleEngine (lr-application)
        ↓
closed domain events/conditions/actions + persistence port
        ↓
SQLite migration 0006 / one atomic WorldStore batch
```

## Implemented vocabulary

**Events and triggers:** The fixed version-1 event set is `quest_completed`, `player_xp_changed`, and `stat_changed`. Events are typed Rust variants with structured Player/Quest/stat context. A Rule trigger selects one of these event kinds; it is event-driven, not a polled state predicate.

**Conditions:** `always`, event-kind match, event-scoped numeric and text comparisons, and nested `ALL`, `ANY`, and `NOT`. A missing/incompatible event subject fails closed, including underneath `NOT` and `ANY`; rule definitions are also checked against the selected trigger before storage. Conditions can inspect only the selected event's fields and do not evaluate user-authored expressions or query arbitrary live entities. Nesting is limited to 8.

**Actions:** `award_xp` (positive award or negative penalty), `complete_quest` (only the existing valid Quest completion transition), `set_player_stat`, and `modify_player_stat`. XP floor, valid Quest lifecycle, finite values, and stat bounds are checked using existing domain operations. Each Rule supports 1–16 ordered actions.

**Rule:** A persistent ID/name/description/enabled/priority/definition/metadata/timestamp record. Definition JSON has `schemaVersion: 1`; unsupported versions, unknown tagged variants/fields, invalid limits, and trigger-incompatible subjects are rejected. There are no permanent built-in demo mechanics; fixtures/tests create sample rules through the same service path.

## Execution, ordering, and atomicity

For an event, enabled rules execute in **descending priority**, then ascending stable Rule ID. Each event's rule set is processed in that order; generated events enter a FIFO queue in action order. Current source operations may provide multiple initial typed events (Quest completion and its XP reward, for example). Execution is deterministic; it relies on neither SQL's incidental row order nor frontend/map order.

The application evaluator validates domain changes and prepares typed operations. It sends the root operation, derived operations, matching XP ledger transactions, and successful/condition-failed audit records through one persistence-port batch. SQLite rechecks expected state/ownership and commits the batch in one transaction. A later action failure, invalid stat bound, depth/evaluation/action budget, or repeated rule/event pair causes all root and derived world writes in that chain to roll back. A separate failure/guard audit insert is attempted after rollback.

Limits are: condition nesting **8**, logical-group width **16**, condition nodes per definition **128**, chain depth **8**, actions per Rule **16**, actions per chain **32**, rule evaluations per chain **64**, plus exact repeated `(Rule ID, canonical event payload)` detection. Text fields are length-bounded. A guard result is a structured application error; it is not silently swallowed.

`rule_execution_history` is a bounded audit/debug layer recording chain/rule/event, condition outcome, configured actions, status/error, depth, and execution time. It is append-only. XP transactions, current aggregate state, and daily snapshots retain their existing authoritative roles.

## Persistence and IPC changes

- Migration `0006_phase3_rules.sql` adds persistent declarative Rule and rule-execution-history tables, indexes, checks, and immutable audit guards; clean databases now reach schema version **6**.
- The application exposes the closed rule model, validation, errors, CRUD/read APIs, event evaluator, action planner, FIFO dispatcher, safety budgets, and `WorldStore` chain operations.
- SQLite implements stable trigger lookup, Rule persistence/enable state, expected-state checks, atomic Quest/XP/stat batches, requested/applied XP ledger writes, and audit records.
- Rust contracts and TypeScript mirrors expose camelCase Rule/RuleDefinition/RuleExecution DTOs. Tauri commands expose list/create/enable-disable/history APIs without rule logic.
- The desktop displays a small authoring and audit panel. It supports common numeric comparisons and XP, Quest-completion, and stat actions; when a Player is active it also offers a normal-path +25 XP test event. This is a proof surface, not the complete RPG frontend.

## Demonstrated examples

1. **XP milestone:** on `player_xp_changed`, compare `current_xp >= 100`, then award XP.
2. **Quest bonus:** on `quest_completed`, compare `quest_type == main`, then award XP. Quest completion, its configured reward, the declarative bonus, and corresponding XP Transactions commit together.
3. **Stat reaction chain:** on `stat_changed`, when Focus equals 5, modify Focus and perform another ordered XP action; the generated stat event can match a second Rule which awards another XP amount.
4. **Penalty:** a negative XP action preserves requested `-10`, records only `-5` applied when the Player has 5 XP, and leaves XP at zero.
5. **Safety:** recursive XP/stat actions, over-budget definitions, a bad later stat action, and repeated event fingerprints are rejected with no partial root/action state writes.

## Migration behavior

Migration tests cover a clean database to version 6, idempotent application, prior-version continuation, and a realistic **version-5 Phase 2.1 → version-6 Phase 3** upgrade retaining Player XP/level, dynamic Player Stat data, XP ledger, and Player snapshot rows. Earlier Phase 1/2.1 tests remain in place.

## Verification performed

Final sign-off runs recorded in this branch:

- `cargo fmt --all -- --check` — passed
- `cargo test --workspace` — **63 unit tests passed**, 0 failed (3 desktop shell, 13 application, 7 contracts, 7 domain, 33 persistence; doc-tests passed)
- Workspace migration tests included clean schema version 6, idempotence, legacy upgrade coverage, and preservation of Player/Stat/XP ledger/snapshot rows through the Phase 2.1 version-5 → Phase 3 version-6 upgrade.
- The persistence tests demonstrated ordered XP rule chaining, Quest completion with rule bonus, generated stat/XP events, XP-floor penalties, disabled-rule behavior, failed-action atomic rollback, depth/action/evaluation/loop guards, durable Rule/audit reopen, and deterministic history.
- `npm run typecheck` — passed
- `npm test -- --run` — **32 tests passed** across 5 files (including Rule authoring and test-event IPC)
- `npm run build:vite` — passed; optimized production bundle generated
- `git diff --check` — passed before checkpoint

## Known limitations / intentionally deferred

- No live-entity-state predicates, state polling, scheduler, background process, or time-based trigger.
- No skill XP/unlock, Effect lifecycle, Quest start/abandon/fail, narrative, or other action families. Only current valid Quest completion is available.
- No user-facing advanced logical-condition editor, action editor, Rule edit/reorder screen, or rule-import format; priority and common comparison/action controls are exposed in the compact panel.
- Rule execution is synchronous and local to the SQLite application port; this is intentionally not an event-sourced or distributed engine.
- If storage itself is unavailable during failure handling, the failure cannot be guaranteed to persist in audit history; the service still returns the structured error.
- No Phase 4 UI state/workspace/presentation persistence is started.

## Recommended Phase 4 starting point

Begin with a separate design/audit for persistent UI state and workspace/presentation records, keeping them separate from Player/world entities and Rule definitions. Use the current typed IPC boundary and existing visual tokens; do not add those concepts to the game-domain schema by analogy.
