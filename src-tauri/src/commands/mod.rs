//! The command surface — this application's IPC API.
//!
//! Every command follows the same three-line shape:
//!
//! ```text
//! 1. take State<'_, AppState>          (dependency injection, no globals)
//! 2. call exactly one use case         (no logic lives here)
//! 3. convert the result into a DTO     (never leak a Rust type to the webview)
//! ```
//!
//! `#[tauri::command]` functions are the *only* public API the frontend has. In
//! Phase 1 there are three: a status read, a tiny liveness ping, and a location
//! readout. That is intentionally small.

pub mod status;
