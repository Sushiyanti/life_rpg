//! Domain error vocabulary.
//!
//! Domain errors describe *rule violations*, never infrastructure failures.
//! "The database is locked" is not a domain error; "a snapshot already exists
//! for this player/date" would be. Keeping these separate is what lets the
//! rules be unit-tested with no I/O anywhere in sight.

use thiserror::Error;

/// A rule or invariant in the world model was violated.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// An identifier was empty, too long, or otherwise unusable.
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),

    /// A value object was constructed with a value outside its contract.
    #[error("invalid value for `{field}`: {reason}")]
    InvalidValue {
        /// Field / value-object name.
        field: &'static str,
        /// Why the value was rejected.
        reason: String,
    },

    /// A multi-field invariant did not hold.
    #[error("invariant violated: {0}")]
    Invariant(String),
}

impl DomainError {
    /// Convenience constructor for [`DomainError::InvalidValue`].
    pub fn invalid_value(field: &'static str, reason: impl Into<String>) -> Self {
        Self::InvalidValue {
            field,
            reason: reason.into(),
        }
    }
}

/// Result alias for domain operations.
pub type DomainResult<T> = Result<T, DomainError>;
