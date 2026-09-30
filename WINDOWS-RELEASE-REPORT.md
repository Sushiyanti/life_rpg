# Life RPG Windows Release Report

## Release identity

| Field | Result |
|---|---|
| Intended release | `v1.0.1` |
| Correct branch | `release/windows-v1.0.1` |
| Exact parent/base | `v1.0.0` → `d077bcb4c4f72cbffb884efecbaf1ba28d325e9f` |
| Ancestry | Verified: the branch was recreated directly from `v1.0.0`; `git merge-base HEAD v1.0.0` was the baseline commit before changes |
| Final preparation commit | `175acd39e3610721f9d9e3fb543e91627bb58fc5` |
| Tag | Not created; required product validation is incomplete |
| GitHub Release | Not created |

The previous mistaken remote branch was deleted before this branch was created. The old branch was based on Phase 1 and is no longer an active branch. No history was rewritten, rebased, squashed, force-pushed, or retagged. The existing Linux `v1.0.0` tag and GitHub Release were not modified.

## Native Windows environment

Evidence source: successful GitHub Actions run [36569734261](https://github.com/Sushiyanti/life_rpg/actions/runs/36569734261).

| Field | Result |
|---|---|
| Runner | GitHub-hosted `windows-latest` |
| Windows | Microsoft Windows NT `10.0.26100.0` / build `26100` |
| Architecture | `AMD64` / x86_64 |
| WebView2 | `153.0.4234.48` |
| Rust | `rustc 1.98.1 (48a229cea 2026-09-01)` |
| Rust host/toolchain | `x86_64-pc-windows-msvc`, stable MSVC |
| Cargo | `1.98.1` |
| Node | `v22.23.2` |
| npm | `10.9.8` |
| Tauri CLI | `tauri-cli 2.11.5` |

## Active release changes

- `package.json` and `package-lock.json`: `1.0.0` → `1.0.1`.
- Rust workspace package metadata: `1.0.0` → `1.0.1`.
- `src-tauri/tauri.conf.json`: `1.0.0` → `1.0.1`.
- Existing Linux `deb` and AppImage targets preserved.
- Windows NSIS target added.
- A Windows `.ico` resource was added because Tauri requires it for Windows resource generation.
- No product features, gameplay systems, migrations, or unrelated dependency upgrades were added.

## Native build

Exact workflow build command:

```text
npm run build -- --bundles nsis
```

Native Windows build result: **PASS**.

The workflow produced one x64-targeted NSIS installer:

| Artifact | Size | SHA-256 |
|---|---:|---|
| `Life RPG_1.0.1_x64-setup.exe` | 2,470,564 bytes | `6b6470ebccad7cfcb94c9fc109369cff3191554fac5958a17e7702c66d069351` |

The checksum was generated on Windows and independently verified after downloading the raw GitHub Actions artifact archive. The installer is available in workflow artifact `life-rpg-windows-v1.0.1` from run 36569734261.

## Automated validation

All of the following passed on the corrected `v1.0.0`-derived branch in the native Windows runner:

- `npm ci`.
- `npm test -- --run`.
- `npm run typecheck`.
- `cargo fmt --all -- --check`.
- `cargo test --workspace --all-targets`.
- `cargo check --workspace`.
- `npm run build -- --bundles nsis`.
- NSIS installer generation.
- Installer launch and silent installation.
- Installed application launch.
- Application close and relaunch.
- Windows SQLite database creation at `%APPDATA%\com.liferpg.desktop\life-rpg.sqlite3`.
- Windows npm audit: **0 vulnerabilities** across 240 audited dependencies.

## Native runtime validation actually performed

The successful workflow performed a real Windows install/launch smoke test:

1. Built the installer on `windows-latest`.
2. Installed it silently into a disposable runner directory.
3. Located and launched the installed executable independently of the source/build tree.
4. Confirmed the application stayed alive after launch.
5. Confirmed the Windows application-data SQLite file was created.
6. Terminated the process for the smoke test.
7. Relaunched the installed application successfully.

Result: **PASS for installation, launch, close, relaunch, and initial database creation.**

## Required validation still missing

The workflow did not perform the complete product-level validation required for a release claim. The following remain unverified on Windows:

- Actual interactive first-run/world creation through the installed UI.
- Player creation through the installed UI.
- Version `1.0.1` observed in the running UI.
- Keyboard focus/navigation through the installed UI.
- Representative UI workflows for Player, Skills/Skill Tree, Quest, Stage, Branch, Session, Content, Concept/progress, Effect, Tags, Workspace, and comments/history.
- Real application backup creation, restore, and malformed-backup rejection.
- Windows-specific path cases involving spaces, Unicode, backup paths, temporary files, and permissions.
- A transaction-focused abrupt process-termination test followed by SQLite integrity verification.
- Normal uninstall, reinstall, and post-reinstall launch.

The corrected branch does contain and pass the existing Rust release-candidate integration coverage for populated worlds, relationships, backup/restore, repeated restarts, integrity, and controlled SIGKILL scenarios. That is valuable regression evidence, but it is not a substitute for the missing installed-Windows application workflows required by this task.


### Final UI-gate attempts

Three fresh runs were attempted after adding a bounded installed-UI harness:

- [36574865973](https://github.com/Sushiyanti/life_rpg/actions/runs/36574865973): app initialization timing failure.
- [36576492791](https://github.com/Sushiyanti/life_rpg/actions/runs/36576492791): WebView2 CDP endpoint `http://127.0.0.1:9222/json/version` did not appear.
- [36577982721](https://github.com/Sushiyanti/life_rpg/actions/runs/36577982721): same WebView2 CDP endpoint remained unavailable after isolating the WebView2 process and user-data folder.

The installed application launched and the existing native smoke checks remained successful, but the runner did not expose a controllable WebView2 debugging endpoint. Therefore the interactive UI, native file-dialog, and post-reinstall product gates were **not claimed as passed**. The release remains blocked rather than being published with incomplete evidence.

## Staged validation pipeline

The release workflow was refactored at commit `175acd39e3610721f9d9e3fb543e91627bb58fc5` and run [36716060176](https://github.com/Sushiyanti/life_rpg/actions/runs/36716060176). The installer was built once and reused through the Actions artifact `life-rpg-windows-v1.0.1-installer`.

| Stage | Result | Timeout | Evidence |
|---|---|---:|---|
| Build Windows installer once | PASS | 45 min | NSIS installer, checksum, environment artifact |
| Basic installed smoke | PASS | 15 min | Install, launch, SQLite initialization, close, relaunch |
| WebView2 diagnostic | PASS | 20 min | WebView2 `153.0.4234.48`, `127.0.0.1:9222/json/version`, CDP PASS |
| Durability and reinstall | PASS | 25 min | Abrupt restart, SQLite initialization, uninstall, reinstall, persistence |
| UI and product validation | BLOCKED | 30 min | Installed app reaches real backup flow; valid-backup preview does not render |

The UI evidence shows first-run/world creation, runtime version, representative workflows, keyboard navigation, native save/open dialogs, and Windows spaces/Unicode backup creation passing. The remaining failure occurs after the valid backup dialog closes: the app remains on `Working…` and exposes neither the verified preview nor an error alert. The stage now fails within its bounded timeout and uploads logs/screenshots instead of consuming the whole workflow indefinitely.

## Manual beta feedback

Manual testing of `v1.0.1-beta.1` identified substantial product-readiness concerns beyond packaging: unclear navigation and status presentation, insufficient inline detail/popups, weak notes/concept editing and reuse, missing player level metadata management, limited customization, and hard-coded workflows. These are recorded as product backlog concerns and reinforce the decision not to publish final `v1.0.1` until the core release blockers and an agreed product-quality scope are addressed.

## Security and signing

- Frontend/npm audit on the corrected branch: **0 vulnerabilities**.
- Rust workspace formatting/tests/checks: passed.
- RustSec audit: no new audit was run in the workflow; the preserved v1.0.0 Phase 13 baseline recorded 0 RustSec vulnerabilities and two informational notices.
- No certificate or signing infrastructure was available.
- Installer status: **unsigned**. Windows SmartScreen warnings are possible; no signed status is claimed.
- No secrets, certificates, or private keys were committed.

## Scope

**Validated:**

- Correct ancestry from Linux `v1.0.0`.
- Native Windows x64 build and NSIS packaging.
- Windows installer installation, launch, close, relaunch, and initial database creation.
- Corrected-branch automated frontend and Rust checks.

**Not released/fully validated:**

- Windows x64 as a complete product-support claim, pending the product-level workflows listed above.
- Windows ARM64.
- macOS.
- Other architectures/platforms.

**Existing release preserved:** Linux x86_64 `v1.0.0`.

## Final decision

# NOT READY FOR WINDOWS RELEASE

The native Windows build and basic installer smoke test succeeded, but the required installed-application first-run, representative workflow, backup/restore, termination-integrity, filesystem, and uninstall/reinstall evidence is incomplete. Per the release instructions, no `v1.0.1` tag, GitHub Release, or final release checksum manifest has been created.
