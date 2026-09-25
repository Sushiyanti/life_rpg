//! SQLite connection configuration.
//!
//! Pragmas are not decoration; each one here is load-bearing for a
//! single-user, offline desktop app:
//!
//! | Pragma          | Value  | Why                                                      |
//! |-----------------|--------|----------------------------------------------------------|
//! | `journal_mode`  | `WAL`  | Readers don't block the writer; survives a hard crash.    |
//! | `foreign_keys`  | `ON`   | Off by default in SQLite — we want real referential integrity. |
//! | `synchronous`   | `NORMAL` | Durable under WAL while keeping writes fast on SSD.     |
//! | `busy_timeout`  | 5000ms | Tolerate momentary contention instead of erroring out.    |
//! | `temp_store`    | `MEMORY` | Avoid spilling scratch data to disk.                    |

use rusqlite::Connection;

use crate::error::PersistenceError;

/// Apply the application's pragma configuration to a connection.
///
/// Safe to call on every open; it is idempotent. `journal_mode` is a
/// persistent property of the file, the rest are per-connection.
pub fn configure(conn: &Connection) -> Result<(), PersistenceError> {
    // Returns the resulting mode as a row, so we must query rather than update.
    let _mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;

    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.busy_timeout(std::time::Duration::from_millis(5_000))?;

    Ok(())
}

/// Read the journal mode currently in effect.
///
/// Note: an in-memory store reports `memory` — `WAL` requires a real file. That
/// is expected and the status screen shows it verbatim rather than pretending.
pub fn journal_mode(conn: &Connection) -> Result<String, PersistenceError> {
    let mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
    Ok(mode)
}

/// Read whether foreign-key enforcement is on for this connection.
pub fn foreign_keys_enabled(conn: &Connection) -> Result<bool, PersistenceError> {
    let enabled: i64 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    Ok(enabled == 1)
}
