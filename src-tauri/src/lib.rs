//! # `life-rpg` desktop shell — the composition root and IPC registration.
//!
//! It wires persistence into application services, but contains no SQL or game rules.

pub mod commands;
pub mod state;
pub use state::AppState;

use lr_application::{Clock, ConceptService, HealthService, SemanticsService, WorldService};
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
        WorldService::new(store.clone(), SystemClock),
        SemanticsService::new(store.clone(), SystemClock),
        ConceptService::new(store, SystemClock),
    )
}
pub fn bootstrap_fallback(reason: impl Into<String>, now: &str) -> AppState {
    let store = Arc::new(SqliteHealthStore::open_in_memory(now));
    let mut state = AppState::new(
        HealthService::new(store.clone(), SystemClock),
        WorldService::new(store.clone(), SystemClock),
        SemanticsService::new(store.clone(), SystemClock),
        ConceptService::new(store, SystemClock),
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
            commands::world::set_player_progression,
            commands::world::set_skill_progression,
            commands::world::award_xp,
            commands::world::create_quest,
            commands::world::start_quest,
            commands::world::complete_quest,
            commands::world::create_skill_tree,
            commands::world::add_skill,
            commands::world::invest_skill_time,
            commands::world::capture_player_snapshot,
            commands::world::capture_skill_snapshot,
            commands::world::list_player_snapshots,
            commands::world::list_skill_snapshots,
            commands::world::define_stat,
            commands::world::list_stat_definitions,
            commands::world::set_player_stat,
            commands::world::list_player_stats,
            commands::world::list_rules,
            commands::world::create_rule,
            commands::world::set_rule_enabled,
            commands::world::list_rule_executions,
            commands::world::deactivate_effect,
            commands::world::add_comment,
            commands::world::list_comments,
            commands::world::write_narrative,
            commands::world::get_world_overview,
            commands::world::list_transactions,
            commands::concepts::create_concept,
            commands::concepts::list_concepts,
            commands::concepts::list_concept_progress,
            commands::concepts::set_concept_progress,
            commands::concepts::list_concept_relationships,
            commands::concepts::list_concept_relationship_types,
            commands::concepts::relate_concepts,
            commands::semantics::create_quest_stage,
            commands::semantics::list_quest_stages,
            commands::semantics::create_quest_branch,
            commands::semantics::list_quest_branches,
            commands::semantics::start_quest_session,
            commands::semantics::finish_quest_session,
            commands::semantics::list_quest_sessions,
            commands::semantics::attach_content,
            commands::semantics::list_attached_content,
            commands::semantics::associate_concept,
            commands::semantics::list_concept_associations,
            commands::semantics::set_concept_association_active,
            commands::semantics::set_entity_lifecycle,
            commands::semantics::get_entity_lifecycle,
            commands::semantics::list_entity_revisions,
            commands::semantics::restore_entity_revision,
            commands::semantics::set_presentation_preference,
            commands::semantics::set_presentation_visibility,
            commands::semantics::list_presentation_preferences,
            commands::semantics::create_workspace,
            commands::semantics::list_workspaces,
            commands::semantics::rename_workspace,
            commands::semantics::delete_workspace,
            commands::semantics::list_workspace_panels,
            commands::semantics::save_workspace_panel,
            commands::semantics::delete_workspace_panel,
            commands::semantics::suggest_concept_progress,
            commands::semantics::list_progress_suggestions,
            commands::semantics::accept_progress_suggestion,
            commands::semantics::reject_progress_suggestion,
            commands::semantics::search_world,
            commands::semantics::set_concept_progress_control
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
    fn phase21_world_stats_and_snapshots_survive_restart() {
        let dir = tempfile::tempdir().unwrap();
        let id;
        let skill_id;
        {
            let state = bootstrap(dir.path(), T0);
            let player = state.world.create_player("Ada", None).unwrap();
            id = player.id.to_string();
            state.world.award_xp(&id, 125, None, None).unwrap();
            state
                .world
                .define_stat(
                    "focus",
                    "Focus",
                    None,
                    Some("points".into()),
                    Some(0.0),
                    Some(10.0),
                )
                .unwrap();
            state.world.set_player_stat(&id, "focus", 7.5).unwrap();
            let tree = state
                .world
                .create_skill_tree(&id, "programming", "Programming")
                .unwrap();
            let skill = state
                .world
                .add_skill(tree.id.as_str(), "core", "Rust", None)
                .unwrap();
            skill_id = skill.id.to_string();
            state.world.capture_player_snapshot(&id).unwrap();
            state.world.capture_skill_snapshot(&skill_id).unwrap();
        }
        let state = bootstrap(dir.path(), T0);
        assert_eq!(
            state.world.get_player(&id).unwrap().unwrap().current_xp,
            125
        );
        assert_eq!(
            state.world.list_player_stats(&id).unwrap()[0].current_value,
            7.5
        );
        let player_history = state.world.list_player_snapshots(&id).unwrap();
        assert_eq!(player_history.len(), 1);
        assert_eq!(player_history[0].current_xp, 125);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&player_history[0].state_json).unwrap()
                ["stats"][0]["value"],
            7.5
        );
        let skill_history = state.world.list_skill_snapshots(&skill_id).unwrap();
        assert_eq!(skill_history.len(), 1);
        assert_eq!(skill_history[0].status.as_str(), "active");
    }
}
