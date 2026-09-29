//! Status / health commands.
//!
//! These back the system-health view. `get_status` is the important one: it
//! calls the health use case, which performs a **real transaction-wrapped write
//! and read-back** against SQLite. When the UI says "SQLite is reachable", it is
//! reporting committed evidence, not a hardcoded `true`.

use tauri::State;

use lr_contracts::{CommandErrorDto, HealthReportDto};

use crate::state::AppState;

/// Run a full health check including a persistence round trip.
///
/// Returns a [`HealthReportDto`] on success. Note that a *degraded* or *failed*
/// world is still a successful call — the report carries the verdict. The
/// `Result` is for genuine command-level failures (serialization, poisoned
/// state), which the frontend handles through [`CommandErrorDto`].
#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> Result<HealthReportDto, CommandErrorDto> {
    let mut report = state.health.run();
    if let Some(warning) = state.startup_warning() {
        report.status = lr_application::HealthStatus::Degraded;
        report.headline =
            "Persistent storage is unavailable — this temporary world will not save changes"
                .to_string();
        report.problems.insert(0, warning);
    }
    Ok(HealthReportDto::from(report))
}

/// Safe location hint for the world database, without disclosing a machine path.
#[tauri::command]
pub fn get_world_location(state: State<'_, AppState>) -> Result<Option<String>, CommandErrorDto> {
    Ok(state
        .health
        .store()
        .location()
        .map(|_| "Life RPG application data folder".to_string()))
}

/// Liveness probe used by the frontend before it decides to render.
///
/// Deliberately trivial: it returns a static string and must never touch the
/// database, so the UI can distinguish "the core is not responding at all" from
/// "the core responded but storage is down".
#[tauri::command]
pub fn ping() -> String {
    "pong".to_string()
}
