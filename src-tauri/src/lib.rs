//! # `life-rpg` — the desktop shell (composition root)
//!
//! This crate is the *only* place where the layers meet. Its job:
//!
//! 1. Decide where the world database lives (`app_data_dir`, an OS-appropriate
//!    per-user directory).
//! 2. Build the persistence adapter and inject it into the application service.
//! 3. Expose that service to the frontend as typed **commands**.
//!
//! What this crate must **not** contain: SQL, business rules, or anything the
//! domain already knows. Commands are adapters — they translate an IPC request
//! into a use-case call and a use case result into a DTO. If a command grows an
//! `if` that encodes a game rule, that rule is in the wrong layer.
//!
//! ## Why there is no HTTP server here
//!
//! Tauri v2 exposes `#[tauri::command]` functions over an **in-process IPC
//! channel** between the Rust core and the webview. There is no port to open, no
//! socket, no CORS, no auth, and nothing reachable from another machine. This is
//! the "local/native application boundary" the requirements ask for — an HTTP
//! server would add an attack surface, a port conflict, a startup race and a
//! serialization hop in exchange for nothing, because there is exactly one
//! client and it is already in the same process tree.
//!
//! ## Two construction paths
//!
//! * [`bootstrap`] — real startup: open the OS data directory.
//! * [`bootstrap_fallback`] — if that fails, an **in-memory** world so the
//!   window still opens and the status screen can explain the failure. The app
//!   never dies to a blank screen.

pub mod commands;
pub mod state;

pub use state::AppState;

use std::path::PathBuf;

// `Manager` provides `App::path()` and `App::manage()`; `Listener` would provide
// event hooks, which Phase 1 does not need yet.
use tauri::Manager;

use lr_application::{Clock, HealthService};
use lr_persistence::SqliteHealthStore;

/// Real clock, as a tiny zero-sized type so `HealthService<_, SystemClock>` is
/// trivially constructible and the frozen test clock stays shape-compatible.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_rfc3339(&self) -> String {
        chrono::Utc::now().to_rfc3339()
    }

    fn now_unix_nanos(&self) -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    }
}

/// Filename of the world database inside the app data directory.
pub const WORLD_DB_FILENAME: &str = "life-rpg.sqlite3";

/// Resolve the world database path inside an OS app-data directory.
pub fn world_db_path(app_data_dir: &std::path::Path) -> PathBuf {
    app_data_dir.join(WORLD_DB_FILENAME)
}

/// Open the world for real, at `app_data_dir`.
pub fn bootstrap(app_data_dir: &std::path::Path, now: &str) -> AppState {
    let path = world_db_path(app_data_dir);
    AppState::new(HealthService::new(
        SqliteHealthStore::open_file(path, now),
        SystemClock,
    ))
}

/// Open an in-memory world so the UI can still launch and report the failure.
pub fn bootstrap_fallback(reason: impl Into<String>, now: &str) -> AppState {
    let store = SqliteHealthStore::open_in_memory(now);
    let mut state = AppState::new(HealthService::new(store, SystemClock));
    state.set_startup_warning(Some(reason.into()));
    state
}

/// Tauri entry point. Called from `main.rs` and from integration tests.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Resolve the OS-appropriate per-user data directory. On Linux this
            // is ~/.local/share/com.liferpg.desktop, on macOS
            // ~/Library/Application Support/..., on Windows %APPDATA%\...
            let now = SystemClock.now_rfc3339();

            let state = match app.path().app_data_dir() {
                Ok(dir) => {
                    let state = bootstrap(&dir, &now);
                    if !state.health.store().is_available() {
                        // The file-backed store failed; keep running so the user
                        // can see why instead of getting a dead window.
                        let reason = state
                            .health
                            .store()
                            .unavailability_reason()
                            .unwrap_or("unknown storage failure")
                            .to_string();
                        bootstrap_fallback(
                            format!(
                                "Using an in-memory world: could not open {} ({reason})",
                                world_db_path(&dir).display()
                            ),
                            &now,
                        )
                    } else {
                        state
                    }
                }
                Err(err) => bootstrap_fallback(
                    format!("Using an in-memory world: no app data directory ({err})"),
                    &now,
                ),
            };

            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::status::get_status,
            commands::status::get_world_location,
            commands::status::ping,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Life RPG desktop shell");
}

#[cfg(test)]
mod tests {
    use super::*;
    use lr_application::{HealthStatus, HealthStore};

    const T0: &str = "2026-09-25T00:00:00+00:00";

    #[test]
    fn world_db_path_is_inside_the_app_data_dir() {
        let path = world_db_path(std::path::Path::new("/tmp/liferpg-data"));
        assert_eq!(
            path,
            PathBuf::from("/tmp/liferpg-data").join("life-rpg.sqlite3")
        );
        assert_eq!(path.file_name().unwrap(), "life-rpg.sqlite3");
    }

    #[test]
    fn bootstrap_creates_a_working_world() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = bootstrap(dir.path(), T0);

        assert!(state.health.store().is_available());
        assert!(state.startup_warning().is_none());

        let report = state.health.run();
        assert_eq!(report.status, HealthStatus::Ok, "{:?}", report.problems);
        assert!(dir.path().join(WORLD_DB_FILENAME).exists(), "db file on disk");
    }

    #[test]
    fn bootstrap_fallback_keeps_the_ui_usable_and_attaches_the_reason() {
        let state = bootstrap_fallback("simulated failure", T0);

        let warning = state.startup_warning().expect("warning recorded");
        assert!(warning.contains("simulated failure"));

        // Still a working (in-memory) world: the status screen has real data.
        let report = state.health.run();
        assert_eq!(report.status, HealthStatus::Ok, "{:?}", report.problems);
        assert!(report.round_trip.as_ref().unwrap().matches);
    }

    #[test]
    fn system_clock_produces_parseable_rfc3339() {
        let now = SystemClock.now_rfc3339();
        let parsed = lr_domain::Iso8601Timestamp::parse(now).expect("valid rfc3339");
        assert!(parsed.as_str().ends_with("+00:00"));
        assert!(SystemClock.now_unix_nanos() > 0);
    }

    #[test]
    fn bootstrapped_world_is_durable_across_restarts() {
        let dir = tempfile::tempdir().expect("tempdir");

        {
            let state = bootstrap(dir.path(), T0);
            state.health.store().verify_round_trip("first-boot", T0).expect("write");
        }

        let second = bootstrap(dir.path(), T0);
        let proof = second
            .health
            .store()
            .verify_round_trip("second-boot", T0)
            .expect("write");
        assert_eq!(proof.probe_rows, 2, "the world survived a restart");
    }
}
