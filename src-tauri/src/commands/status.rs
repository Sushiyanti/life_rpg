//! Status / health commands.
//!
//! These back the Phase 1 status screen. `get_status` is the important one: it
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
    let report = state.health.run();
    Ok(HealthReportDto::from(report))
}

/// Where the world database lives on disk, or the reason it is not file-backed.
///
/// Split out from `get_status` so the UI can show the path in its own section
/// (and so a future "reveal in file manager" button has a cheap call to make).
#[tauri::command]
pub fn get_world_location(state: State<'_, AppState>) -> Result<Option<String>, CommandErrorDto> {
    Ok(state.health.store().location())
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
