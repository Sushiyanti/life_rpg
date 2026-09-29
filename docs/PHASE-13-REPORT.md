# Phase 13 — Release Candidate Validation Report

- **Decision:** READY FOR 1.0 — validated scope: Linux x86_64, Ubuntu 22.04 (Jammy) or a compatible newer distribution
- **Product version tested:** `0.1.0`
- **Database schema:** `17` (no schema migration added)
- **Branch:** `phase-13-release-candidate`
- **Validated source revision:** `60f1e9cb691fbdeb9d50dc4a0ec1b0d32a3b184a` (same as the Phase 12 parent; Phase 12 commit remains an ancestor and branch HEAD was not moved)

## Executive decision

No release-blocking data-loss, persistence, packaging, build, or keyboard-operability defect was found in the tested Linux x86_64 scope. The full Rust and frontend suites passed; a populated-world workflow survived backup/restore and repeated process restarts; deterministic SIGKILL tests recovered without partial world writes; and both Jammy Linux packages launched in isolated profiles.

**Release recommendation:** proceed to a 1.0 release for **Linux x86_64 with Ubuntu 22.04 or a compatible newer GTK 3/WebKitGTK 4.1 environment as the minimum tested baseline**. This is not a certification for Ubuntu 20.04 or earlier, other Linux architectures, Windows, or macOS. The release package still reports version `0.1.0`; apply the normal version/signing/release-channel steps when cutting the actual 1.0 release. No product version, gameplay, or schema change was made in this validation phase.

## Scope and branch integrity

- Work remained on `phase-13-release-candidate`; HEAD stayed at the verified Phase 12 parent revision. No commit, reset, rebase, or force-update was performed.
- The Phase 13 changes are validation fixtures, release evidence, and package outputs only. No gameplay system or domain architecture was expanded.
- The mandatory parent-commit baseline gate had already passed before scenario work: **150 Rust tests** and **124 frontend tests**, plus production build/package and security checks.
- The final test run added coverage to that baseline and exercised the unchanged production implementation. The package artifacts were built from the same production revision.

## Validation results

| Area | Result |
|---|---|
| `cargo test --workspace` | **158 passed, 0 failed, 4 ignored** (ignored entries are child-process helpers intentionally invoked by their parent SIGKILL tests) |
| Frontend/Vitest | **124 passed**, 22 test files |
| TypeScript typecheck | Passed |
| Vite production build | Passed; 78 modules transformed |
| `cargo fmt --all -- --check` | Passed |
| `git diff --check` | Passed |
| Baseline security scans | Frontend: 0 vulnerabilities; RustSec: 0 vulnerabilities across 449 locked crates, with two informational transitive warnings (details below) |
| Current schema | v17; no Phase 13 migration introduced |

## Real-world and persistence scenarios

### Populated Player world

`populated_player_workflow_survives_backup_restore_and_repeated_restarts` created and exercised a disposable file-backed world containing a Player, authored stat, Skill tree and Skills, Skill availability policy, Concept with percentage progress and snapshots, Quest with stage and branch, Tags, a recorded Session, narrative/Content relations, comment, Workspace and filtered panel, and an Effect with history.

A bounded cross-event Rule chain completed a Quest, awarded XP and created an Effect, then used XP to award Skill XP and unlock a related Skill. The workflow verified the expected state, wrote and inspected a backup, restored it, reloaded the app state, and reopened the same database through **four repeated restart cycles**. IDs, relations, history, state, and integrity remained stable.

### Performance and volume trial

The bounded synthetic trial populated **500 Quests, 100 Content records, and 500 Tag links**, plus a 30-panel Workspace. It exercised filtered 100-result search, ANY/ALL Tag filtering, repeated and paged Timeline queries, Workspace loading, integrity inspection, backup, and restore.

| Operation | Observed |
|---|---:|
| Startup | 288 ms |
| Populate 500 Quests / 100 Content / 500 links | 2,719 ms |
| Load 30 Workspace panels | 9 ms |
| Search returning 100 hits | 67 ms |
| Timeline page of 200 transactions | 18 ms |
| Backup, 4,112,384-byte database | 494 ms |
| Restore | 824 ms |

These are single-run observations in the sandbox, not hardware-independent service-level guarantees. Search, Timeline, Workspace results, and integrity were checked for correctness and deterministic paging as well as timing.

### Abrupt termination and fault injection

All kill tests use disposable files and real subprocess termination; the application did not gain runtime crash hooks.

- **In-flight SQLite world transaction:** a child was SIGKILLed after uncommitted aggregate-plus-ledger writes. Reopening recovered a consistent database and the uncommitted transition was absent.
- **Rule execution:** a test-only SQLite trigger held the writer lock during a real `QuestCompleted` Rule audit insert. The parent observed the lock for the controlled interval and SIGKILLed the child. On reopen, the Quest remained active, Player XP was unchanged, no ledger or Rule-execution rows were partially committed, and integrity checks passed.
- **Backup publication:** the child was killed while the archive was still temporary. The selected destination remained unpublished/byte-identical and the source database remained healthy.
- **Restore replacement:** the child was killed during live replacement after the verified safety snapshot step. Recovery reopened a complete old or complete restored world—never a torn mixture—and integrity checks passed.
- **Migration matrix:** historical schema versions 1 and 13–16 were upgraded to v17 in fixtures; existing Player data was checked where present. Existing invalid-world refusal and migration safety tests also passed.
- **Archive boundary:** a sparse archive just over the 512 MiB limit was rejected before payload reads, without allocating or writing the full payload.

## Linux artifact validation and supported baseline

The `.deb` and AppImage were built inside the disposable Ubuntu 22.04 Jammy rootfs, installed/extracted there, and launched under a private Xvfb display with separate disposable app-data locations.

- **`.deb`:** `amd64`, package `life-rpg` version `0.1.0`; installed successfully and launched to first run. Declared runtime dependencies are `libwebkit2gtk-4.1-0` and `libgtk-3-0`. The first-run world was created by keyboard-only input, and a Workspace action was activated with Enter.
- **AppImage:** launched with `--appimage-extract-and-run` and rendered the first-run window successfully in the same Jammy environment.
- The installed `.deb` executable had no missing dynamic-library dependencies in the Jammy rootfs; that userspace reports glibc **2.35**. Jammy/compatible newer is therefore the minimum **tested** baseline; older releases and other architectures remain unvalidated.

The validation packages remain available from the immutable [Phase 13 RC commit](https://github.com/Sushiyanti/life_rpg/tree/071a13a0730e8995c46f2e24b94375ca0e41d22d/release/phase-13/):

| Artifact | Size | SHA-256 |
|---|---:|---|
| [Life-RPG_0.1.0_amd64_ubuntu-22.04.deb](https://raw.githubusercontent.com/Sushiyanti/life_rpg/071a13a0730e8995c46f2e24b94375ca0e41d22d/release/phase-13/Life-RPG_0.1.0_amd64_ubuntu-22.04.deb) | 3,133,432 bytes | `e576f17dac7b14ef629a3167979284a6aaf26db83b502c7f46f01e1a25e67677` |
| [Life-RPG_0.1.0_amd64_ubuntu-22.04.AppImage](https://raw.githubusercontent.com/Sushiyanti/life_rpg/071a13a0730e8995c46f2e24b94375ca0e41d22d/release/phase-13/Life-RPG_0.1.0_amd64_ubuntu-22.04.AppImage) | 81,373,688 bytes | `70ca6feaf522d5a2932ed1618991bfa413dc9256fbe183b9f9162e9427e3e6bf` |

Screenshots: [`.deb` keyboard-create focus](phase13-evidence/deb-keyboard-create-focus.png), [Workspace opened by keyboard](phase13-evidence/deb-keyboard-workspace.png), and [AppImage first run](phase13-evidence/appimage-first-run.png).

## Accessibility and keyboard walkthrough

A real keyboard-only walk-through was performed against the installed `.deb` under Xvfb. Tab traversal showed a visible focus indicator across navigation and first-run form controls; the test Player name and optional note were entered without pointer input, and the world was submitted with Enter. After creation, Tab exposed the main actions; Open Workspace was activated with Enter and navigated to the expected view. The AppImage first-run screen was also visually confirmed.

This was a focused keyboard-operability pass, **not** screen-reader certification, an automated axe scan, or a formal WCAG conformance audit. No keyboard trap or inability to create the initial world was observed.

## Security and known limitations

- Frontend audit: **0 vulnerabilities**.
- RustSec audit of 449 locked crates: **0 vulnerabilities**. The two existing informational notices are `RUSTSEC-2024-0429` (unsound `glib 0.18.5` `VariantStrIter` methods; GTK/Tauri transitive dependency) and `RUSTSEC-2024-0370` (`proc-macro-error 1.0.4` unmaintained). Neither is reported as a vulnerability; no direct use of the affected iterator API was found in the reviewed application/Tauri integration. These remain dependency-upgrade follow-ups, not Phase 13 blockers.
- Backups are local and unencrypted; users should treat them as sensitive world data.
- Only Linux x86_64 / Jammy-compatible operation was validated here. No Windows/macOS, earlier Ubuntu, other CPU architecture, screen-reader, or assistive-technology certification is claimed.

## Final release-candidate decision

**READY FOR 1.0 — for Linux x86_64 on Ubuntu 22.04 or compatible newer GTK 3/WebKitGTK 4.1 systems.** The exercised critical paths passed, no partial-commit or data-loss defect was observed, and no release blocker was discovered in the declared tested scope. Before publishing, apply the project’s chosen 1.0 version, signing, and release-channel process; do not extend the support claim beyond the platforms and baseline validated above.
