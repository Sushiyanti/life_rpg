//! # `life-rpg` desktop shell — the composition root and IPC registration.
//!
//! It wires persistence into application services, but contains no SQL or game rules.

pub mod commands;
pub mod state;
pub use state::AppState;

use lr_application::{Clock, HealthService, WorldService};
use lr_persistence::SqliteHealthStore;
use std::{path::PathBuf, sync::Arc};
use tauri::Manager;

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
pub const WORLD_DB_FILENAME: &str = "life-rpg.sqlite3";
pub fn world_db_path(app_data_dir: &std::path::Path) -> PathBuf {
    app_data_dir.join(WORLD_DB_FILENAME)
}
pub fn bootstrap(app_data_dir: &std::path::Path, now: &str) -> AppState {
    let store = Arc::new(SqliteHealthStore::open_file(
        world_db_path(app_data_dir),
        now,
    ));
    AppState::new(
        HealthService::new(store.clone(), SystemClock),
        WorldService::new(store, SystemClock),
    )
}
pub fn bootstrap_fallback(reason: impl Into<String>, now: &str) -> AppState {
    let store = Arc::new(SqliteHealthStore::open_in_memory(now));
    let mut state = AppState::new(
        HealthService::new(store.clone(), SystemClock),
        WorldService::new(store, SystemClock),
    );
    state.set_startup_warning(Some(reason.into()));
    state
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let now = SystemClock.now_rfc3339();
            let state = match app.path().app_data_dir() {
                Ok(dir) => {
                    let state = bootstrap(&dir, &now);
                    if !state.health.store().is_available() {
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
            commands::world::create_player,
            commands::world::get_player,
            commands::world::award_xp,
            commands::world::create_quest,
            commands::world::start_quest,
            commands::world::complete_quest,
            commands::world::create_skill_tree,
            commands::world::add_skill,
            commands::world::invest_skill_time,
            commands::world::capture_player_snapshot,
            commands::world::add_comment,
            commands::world::write_narrative,
            commands::world::get_world_overview,
            commands::world::list_transactions
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Life RPG desktop shell");
}

#[cfg(test)]
mod tests {
    use super::*;
    use lr_application::HealthStatus;
    const T0: &str = "2026-09-25T00:00:00+00:00";
    #[test]
    fn world_db_path_is_inside_data_dir() {
        assert_eq!(
            world_db_path(std::path::Path::new("/tmp/data")),
            PathBuf::from("/tmp/data/life-rpg.sqlite3")
        );
    }
    #[test]
    fn bootstrap_creates_working_world() {
        let dir = tempfile::tempdir().unwrap();
        let state = bootstrap(dir.path(), T0);
        assert_eq!(state.health.run().status, HealthStatus::Ok);
        assert!(dir.path().join(WORLD_DB_FILENAME).exists());
    }
    #[test]
    fn phase2_world_survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let id;
        {
            let state = bootstrap(dir.path(), T0);
            let player = state.world.create_player("Ada", None).unwrap();
            id = player.id.to_string();
            state.world.award_xp(&id, 125, None, None).unwrap();
        }
        let state = bootstrap(dir.path(), T0);
        assert_eq!(
            state.world.get_player(&id).unwrap().unwrap().current_xp,
            125
        );
    }
}
