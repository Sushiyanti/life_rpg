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
//! `#[tauri::command]` functions are the *only* public API the frontend has.
//! Modules keep commands grouped by bounded application responsibility; all
//! storage and gameplay decisions remain below this adapter layer.

pub mod backup;
pub mod concepts;
pub mod semantics;
pub mod status;
pub mod tags;
pub mod timeline;
pub mod world;
