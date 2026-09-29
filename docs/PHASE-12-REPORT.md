# Phase 12 — Release Hardening & Data Safety Report

**Status:** Phase 12 complete; this is not a v1.0 declaration or a start on Phase 13.  
**Product version:** `0.1.0`  
**Database schema:** `17` (no new migration was introduced)  
**Branch:** `phase-12-release-hardening`

## Executive summary

Phase 12 adds recoverable, locally managed world backups; read-only integrity inspection; verified staged restore; pre-migration checkpoints; safer startup failure handling; and an accessible recovery surface on System Health. The world model and gameplay rules are unchanged. Unsafe startup or restore paths fail closed rather than silently recreating or continuing to edit an unverified world.

The native Linux release executable, `.deb`, and AppImage built successfully. The executable was launched in isolated test profiles; both packages were extracted and their packaged binary, desktop entry, and icons inspected, but neither package was installed system-wide. Native checks covered fresh startup, persisted Player/Quest/Chronicle data, backup/preview, same-world restore and reload, malformed-backup rejection, and a schema-16-to-17 upgrade with a retained pre-migration snapshot.

## Backup architecture

- A backup is a version-1 `LIFERPG-BACKUP` envelope containing bounded JSON metadata and a standalone SQLite snapshot made with SQLite's online-backup API, so an open WAL database is captured consistently.
- The manifest records product/application/format/schema versions, creation time, stable Player IDs and display names, database byte count, and a SHA-256 digest. It does not include the source database path or credentials; the embedded SQLite payload is the complete world and may contain the Player's data.
- The archive is limited to 512 MiB; the manifest is limited to 1 MiB and Player descriptors to 10,000. Parsing uses explicit length checks and checked offsets, and database copying/hashing is chunked.
- Publication uses a temporary file in the selected existing directory, flushes and syncs it, validates the complete temporary artifact, publishes without clobbering an existing destination, syncs the directory, and validates the published archive again. Existing destinations are never overwritten.
- Preview and integrity inspection read and validate the selected file without changing it. The preview shows version/schema compatibility, Player identity, digest, size, and integrity outcome.

## Integrity validation

The bounded read-only checker evaluates SQLite `integrity_check`, foreign-key violations, contiguous migration-ledger validity, required tables for the reported schema, malformed configuration JSON, and cross-Player Tag relationships. Startup runs integrity validation before creating an upgrade checkpoint or applying migrations. A failed check produces a stable safe diagnostic rather than exposing raw SQLite/path details or attempting an implicit repair.

Tampered, malformed, oversized, unsupported-format, unsupported-newer-schema, and invalid-ledger archives are rejected. The selected backup is never rewritten by inspection or restore staging.

## Restore architecture and failure handling

1. Re-read and validate the archive, SHA-256, schema, Player descriptors, and integrity.
2. Compare stable Player-ID sets—not mutable names—to identify a different world. The UI requires typing `RESTORE` when identity differs or cannot be verified.
3. Copy the archive to a separate staging database and apply any older supported migrations there. Validate the staged schema, identity, and integrity before touching the live connection.
4. Create and verify a retained pre-restore safety backup of the current file-backed world.
5. Replace the live database and validate it again. If replacement or validation fails, restore from the verified safety copy. If rollback also fails, mark the store unavailable and block subsequent normal operations until recovery/reload.
6. On success, block the stale frontend view and require an explicit app reload before editing resumes.

A non-file-backed temporary fallback is never presented as a durable portable backup source. When persistent startup fails but a verified backup is available, recovery can target the retained failed file-backed store while ordinary operations remain unavailable. The successful-restore dialog shows the location of the newly retained local safety backup; startup health and failure diagnostics use safe labels rather than leaking the database path.

## Migration safety

The current migration ledger remains at schema 17. No Phase 12 migration was necessary, so historical migrations were not rewritten. Existing stores are checked before upgrade; a verified pre-migration snapshot is written and retained before a supported migration runs. Migration-ledger gaps/inconsistencies are rejected rather than guessed or repaired.

Automated tests cover fresh creation, schema-16-to-17 data preservation, checkpoint schema/content, invalid existing data refusing migration without mutation, migration failure with recovery available, and unsupported future schemas refusing downgrade. A separate native smoke profile was seeded from a disposable schema-16 SQLite copy containing one Player, one Quest, and one Chronicle record. Normal app startup upgraded it to schema 17, preserved those records, passed SQLite integrity, and retained one named `pre-migration-v16` backup.

## Native verification, restart, and crash findings

In isolated temporary XDG app-data profiles, the release executable was launched under Xvfb. Fresh schema-17 startup and the normal status screen were observed. The smoke world contained a Player and Quest; a Quick Capture Chronicle note appeared in Timeline. The native file chooser created a verified 896 KiB same-world backup; inspection reported a valid checksum, matching world identity, schema v17, and zero foreign-key issues. Restoring it produced the reload lockout and retained safety backup; reload returned to an operational v17 view with the Player/Quest intact. A malformed archive was rejected with a clear non-destructive message; afterward the database still contained the expected records, passed `integrity_check`, and the valid backup's SHA-256 remained unchanged.

Normal close/relaunch of an isolated profile and restore-triggered UI reload succeeded. The schema-16 native upgrade above also exercised a fresh process start. No power-loss or forced termination during an in-flight SQLite write was simulated; interrupted replacement rollback and migration-failure preservation are covered by automated fault-injection tests. Native manual data entry was intentionally minimal; the automated suites provide the broader feature-family coverage.

## Error handling and accessibility

- Persistent storage fallback is surfaced to the user; it is not silently described as a healthy durable world.
- Stable diagnostic identifiers and recovery guidance replace raw OS/SQLite error strings. Ordinary status IPC no longer exposes an absolute database path.
- Backup controls remain accessible from System Health when the health read is unavailable. The panel uses semantic headings, labeled controls, native Open/Save dialogs, live status/error regions, visible compatibility/integrity results, and an explicit consequences step before restore.
- Unknown/different world identity requires typed confirmation. After restore, stale application content is inert/hidden behind a blocking reload alert dialog.
- Native interaction and component tests verified the described controls and states. No screen-reader/assistive-technology certification or automated axe scan was performed.

## Version, packaging, and performance

`Cargo.toml`, `package.json`, Tauri configuration, and the app-reported health screen consistently show version `0.1.0`; the build label is Phase 12 and the database reports `v17 of v17`. Configured release targets are Linux `.deb` and AppImage, plus the executable. The `.deb` metadata identifies `life-rpg` `0.1.0` for `amd64` and its GTK/WebKit runtime dependencies. Both packages contain the expected `Life RPG` desktop entry and icons. The AppImage and `.deb` were extracted for inspection, not installed. The release executable itself was launched natively in the isolated smoke profiles.

An interactive 896 KiB backup/inspection/restore completed without visible UI blockage; this is a smoke observation, not a benchmark. No near-512-MiB stress or large-world performance benchmark was run. The AppImage is substantially larger than the `.deb` because it bundles runtime content.

## Security review

- `npm ci` completed from the lockfile; `npm audit` reported **0 vulnerabilities** across the installed frontend dependency graph.
- RustSec `cargo audit` scanned **449 locked crate dependencies**: **0 vulnerabilities**, with two informational warnings: `RUSTSEC-2024-0429` (unsound `glib 0.18.5` `VariantStrIter` methods, fixed upstream in glib >=0.20) and `RUSTSEC-2024-0370` (unmaintained `proc-macro-error 1.0.4`). The glib version is pulled transitively by the current Tauri/GTK3 Linux stack. No direct `VariantStrIter` call was found in Life RPG or the reviewed Tauri/GTK integration sources. This remains an upstream dependency limitation to revisit before broad release-candidate validation; no ad-hoc fork or cross-stack GTK upgrade was introduced in Phase 12. The proc-macro warning is an unmaintained build-time dependency, not a reported vulnerability.
- The new archive reader bounds total size/manifest/player count, checks integer offsets, validates JSON and checksums, uses temporary files, and avoids destination clobbering. User data is declarative; no new executable import or arbitrary-process path was added. The runtime uses local Tauri IPC and native file dialogs; no analytics, telemetry, or network client was added.
- SHA-256 detects accidental/tampered bytes but is not a signature or proof of origin. Backups are not encrypted; users should protect backup files as sensitive world data.
- Repository hygiene scans found no committed temporary databases/backups/build outputs, credentials, personal absolute paths, debug prints, TODO/FIXME placeholders, unsafe HTML injection, or process-launch path in maintained runtime code.

## Validation results

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo check --workspace` | Passed |
| `cargo test --workspace` | **150 passed**, 0 failed (85 persistence, 27 domain, 17 application, 16 contracts, 5 native-shell tests) |
| `npm ci` | Passed; clean install |
| `npm run typecheck` | Passed |
| `npm test` | **124 passed**, 22 test files |
| `npm run build:vite` | Passed |
| `npm run build` | Passed; executable, `.deb`, and AppImage produced |
| `npm audit` | 0 vulnerabilities |
| `cargo audit` | 0 vulnerabilities; 2 informational warnings detailed above |
| `git diff --check` | Passed |

Strict workspace Clippy was attempted after installing the standard component. It is not clean due stylistic lints in unchanged Phase 11 code (including `too_many_arguments`, `needless_lifetimes`, `manual_contains`, `large_enum_variant`, and `field_reassign_with_default`). Those unrelated APIs/structures were left unchanged rather than refactored during release hardening.

## Known limitations and Phase 13 deferrals

- Linux packaging is built and inspected; Windows/macOS packaging and installation are not validated here.
- No destructive/abrupt OS-crash test, screen-reader certification, near-limit backup benchmark, or RustSec remediation of the transitive glib warning was performed.
- Backups are local and unencrypted; copying them between machines is a user-managed action.
- Phase 13 remains Release Candidate / real-world validation: broader supported-platform testing, accessibility certification, large-world/performance trials, and reassessment of the GTK/glib dependency warning belong there. No product-feature phase was started, no version was declared 1.0, and no new schema version was created.

**Invariant:** a Player's world can be created, mutated, migrated, backed up, restored, inspected, and recovered without silent data loss or unexplained destructive behavior; the application fails safely when it cannot guarantee the world's integrity.