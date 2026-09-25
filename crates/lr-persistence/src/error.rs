//! Persistence-local error type.
//!
//! Everything that can go wrong while driving SQLite is collapsed into this
//! enum, and then translated into [`lr_application::StorageError`] so the rest
//! of the application never sees a `rusqlite` type. Two hops, on purpose: the
//! local error keeps SQL context for logs, the port error keeps the
//! application decoupled.

use lr_application::StorageError;
use thiserror::Error;

/// A failure while talking to the local SQLite store.
#[derive(Debug, Error)]
pub enum PersistenceError {
    /// Opening (or creating) the database file failed.
    #[error("could not open SQLite database at `{path}`: {source}")]
    Open {
        /// Path we tried to open.
        path: String,
        /// Underlying driver error.
        #[source]
        source: rusqlite::Error,
    },

    /// A statement against the database failed.
    #[error("SQLite statement failed: {0}")]
    Query(#[from] rusqlite::Error),

    /// A migration failed, leaving the store at a known version.
    #[error("migration {version} (`{name}`) failed: {source}")]
    Migration {
        /// Version that failed.
        version: u32,
        /// Migration name.
        name: String,
        /// Underlying driver error.
        #[source]
        source: rusqlite::Error,
    },

    /// The store's schema is newer than this binary understands.
    #[error(
        "store schema v{found} is newer than this build's v{expected}; \
         upgrade the application instead of downgrading its data"
    )]
    SchemaTooNew {
        /// Version found in the store.
        found: u32,
        /// Version this build ships.
        expected: u32,
    },

    /// The store is not usable for this operation.
    #[error("store unavailable: {0}")]
    Unavailable(String),
}

impl From<PersistenceError> for StorageError {
    fn from(value: PersistenceError) -> Self {
        match value {
            PersistenceError::Unavailable(reason) => StorageError::Unreachable(reason),
            PersistenceError::SchemaTooNew { found, expected } => StorageError::Schema(format!(
                "store schema v{found} is newer than build schema v{expected}"
            )),
            other => StorageError::Operation(other.to_string()),
        }
    }
}
