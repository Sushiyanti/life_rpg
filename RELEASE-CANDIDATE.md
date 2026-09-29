# Life RPG — Phase 13 Release Candidate Decision (Historical)

Historical snapshot at the `0.1.0` Release Candidate. For the final v1.0.0 release scope and current downloads, see the [v1.0.0 release notes](V1.0.0-RELEASE-NOTES.md) and [README](README.md).

- **Decision:** READY FOR 1.0
- **Validated release scope:** Linux x86_64 on Ubuntu 22.04 (Jammy) or a compatible newer distribution with GTK 3 and WebKitGTK 4.1.
- **Current app/package version:** `0.1.0`
- **Schema:** `17`

The Release Candidate passed the final Rust workspace suite (**158 passed, 0 failed, 4 child-process helpers intentionally ignored**), the frontend suite (**124 passed across 22 files**), TypeScript typechecking, Rust formatting, and Vite production build. The populated-world/restart workflow, medium-volume trial, historical migration matrix, archive size boundary, and deterministic SIGKILL scenarios for SQLite writes, Rule execution, backup publication, and restore replacement passed.

Both Linux packages were produced in an isolated Jammy userspace. The `.deb` installed and launched; the AppImage launched in extract-and-run mode. Jammy reports glibc 2.35 and the installed `.deb` executable had no missing shared-library dependencies. Keyboard-only first-run world creation and Workspace navigation succeeded.

**Release conditions and boundaries:**

- Limit the initial support claim to Linux x86_64 on Jammy-compatible systems; Ubuntu 20.04 and earlier, other architectures, Windows, macOS, and screen-reader certification were not validated.
- The artifacts are labeled `0.1.0`; apply the project’s version bump, signing, and release-channel process when cutting the actual 1.0 release.
- Existing RustSec informational notices remain (`glib 0.18.5` unsoundness and unmaintained `proc-macro-error 1.0.4`); the audit reported zero vulnerabilities. Backups remain unencrypted and should be treated as sensitive local data.

Full evidence and the SHA-256 values for both packages are in [the Phase 13 report](docs/PHASE-13-REPORT.md). The tested Linux artifacts remain available from the [immutable Phase 13 RC commit](https://github.com/Sushiyanti/life_rpg/tree/071a13a0730e8995c46f2e24b94375ca0e41d22d/release/phase-13/).
