//! Application error vocabulary.
//!
//! Two levels on purpose:
//!
//! * [`StorageError`] is the *port* error — what the application can say about
//!   any persistent store without knowing which one it is. `lr-persistence`
//!   translates `rusqlite::Error` into this and nothing else escapes it.
//! * [`AppError`] is what a command handler returns: storage failures, domain
//!   rule violations, and genuine internal bugs, each still distinguishable so
//!   the UI can react differently to "your store is corrupted" vs "you typed
//!   something invalid".

use lr_domain::DomainError;
use thiserror::Error;

/// Failure of a storage port operation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// The store could not be opened or reached at all.
    #[error("store is not reachable: {0}")]
    Unreachable(String),

    /// A specific operation failed against a reachable store.
    #[error("store operation failed: {0}")]
    Operation(String),

    /// The store's schema is missing, incomplete, or newer than this build.
    #[error("schema problem: {0}")]
    Schema(String),
}

/// Top-level application failure.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AppError {
    /// A storage port failed.
    #[error(transparent)]
    Storage(#[from] StorageError),

    /// A domain rule was violated.
    #[error(transparent)]
    Domain(#[from] DomainError),

    /// A programming error or unexpected state.
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Stable machine-readable code, used by the IPC error DTO so the frontend
    /// can branch on failure kind without parsing prose.
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Storage(StorageError::Unreachable(_)) => "storage_unreachable",
            AppError::Storage(StorageError::Operation(_)) => "storage_operation_failed",
            AppError::Storage(StorageError::Schema(_)) => "storage_schema_problem",
            AppError::Domain(_) => "domain_rule_violated",
            AppError::Internal(_) => "internal_error",
        }
    }
}
