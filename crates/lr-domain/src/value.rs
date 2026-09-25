//! Validated value objects.
//!
//! These are the smallest pieces of the world model. They exist so that later
//! aggregates (`Player`, `Quest`, `Skill`, `Transaction`, …) can be built out
//! of primitives that cannot hold nonsense. A newtype that validates on
//! construction is the cheapest way to make illegal states unrepresentable.

use crate::error::{DomainError, DomainResult};

/// Maximum length accepted for an opaque entity identifier.
pub const MAX_IDENTIFIER_LEN: usize = 128;

/// Stable identity of a domain entity.
///
/// Opaque on purpose: the storage layer may store it as `TEXT`, the UI as a
/// React key, and neither gets to assume anything about its shape. That is what
/// keeps the persistence strategy swappable.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityId(String);

impl EntityId {
    /// Validate and wrap an identifier.
    ///
    /// Rejects empty/whitespace-only ids and ids longer than
    /// [`MAX_IDENTIFIER_LEN`] characters.
    pub fn new(raw: impl Into<String>) -> DomainResult<Self> {
        let raw = raw.into();
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            return Err(DomainError::InvalidIdentifier(
                "must not be empty or whitespace-only".to_string(),
            ));
        }
        if trimmed.chars().count() > MAX_IDENTIFIER_LEN {
            return Err(DomainError::InvalidIdentifier(format!(
                "must be at most {MAX_IDENTIFIER_LEN} characters, got {}",
                trimmed.chars().count()
            )));
        }

        Ok(Self(trimmed.to_string()))
    }

    /// Borrow the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the identifier, returning the owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<&str> for EntityId {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Schema version of the persistent store.
///
/// Ordering matters: migration runners compare versions, and the "is the store
/// newer than the binary?" downgrade guard depends on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaVersion(u32);

impl SchemaVersion {
    /// The version of a store that has never been migrated.
    pub const ZERO: Self = Self(0);

    /// Wrap a raw version number.
    pub const fn new(version: u32) -> Self {
        Self(version)
    }

    /// The raw version number.
    pub const fn get(self) -> u32 {
        self.0
    }

    /// True when `self` is strictly older than `other`.
    pub const fn is_behind(self, other: Self) -> bool {
        self.0 < other.0
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl From<u32> for SchemaVersion {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

/// A timestamp that is guaranteed to be a valid, parseable RFC 3339 / ISO 8601
/// instant.
///
/// Stored as the normalized string form so it round-trips through SQLite
/// `TEXT` unchanged and sorts lexicographically when written in UTC.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Iso8601Timestamp(String);

impl Iso8601Timestamp {
    /// Parse and normalize an RFC 3339 timestamp.
    pub fn parse(raw: impl Into<String>) -> DomainResult<Self> {
        let raw = raw.into();
        let parsed = chrono::DateTime::parse_from_rfc3339(raw.trim()).map_err(|err| {
            DomainError::invalid_value("Iso8601Timestamp", format!("not RFC 3339 ({err})"))
        })?;

        Ok(Self(parsed.to_utc().to_rfc3339()))
    }

    /// Build a timestamp from a `chrono` instant.
    pub fn from_datetime(dt: chrono::DateTime<chrono::Utc>) -> Self {
        Self(dt.to_rfc3339())
    }

    /// Current wall-clock instant as UTC.
    pub fn now() -> Self {
        Self(chrono::Utc::now().to_rfc3339())
    }

    /// The normalized string form.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parse back into a `chrono` instant.
    pub fn to_datetime(&self) -> DomainResult<chrono::DateTime<chrono::Utc>> {
        chrono::DateTime::parse_from_rfc3339(&self.0)
            .map(|dt| dt.to_utc())
            .map_err(|err| {
                DomainError::invalid_value("Iso8601Timestamp", format!("stored value is corrupt ({err})"))
            })
    }
}

impl std::fmt::Display for Iso8601Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_id_accepts_normal_values() {
        let id = EntityId::new("player-1").expect("valid id");
        assert_eq!(id.as_str(), "player-1");
        assert_eq!(id.to_string(), "player-1");
    }

    #[test]
    fn entity_id_trims_surrounding_whitespace() {
        let id = EntityId::new("  quest-42  ").expect("valid id");
        assert_eq!(id.as_str(), "quest-42");
    }

    #[test]
    fn entity_id_rejects_blank_and_overlong_values() {
        assert!(EntityId::new("").is_err());
        assert!(EntityId::new("    ").is_err());
        assert!(EntityId::new("x".repeat(MAX_IDENTIFIER_LEN + 1)).is_err());
        assert!(EntityId::new("x".repeat(MAX_IDENTIFIER_LEN)).is_ok());
    }

    #[test]
    fn entity_id_is_interchangeable_via_try_from() {
        let id: EntityId = "skill-7".try_into().expect("valid id");
        assert_eq!(id.as_str(), "skill-7");
    }

    #[test]
    fn schema_version_ordering_supports_downgrade_guard() {
        assert!(SchemaVersion::ZERO.is_behind(SchemaVersion::new(1)));
        assert!(!SchemaVersion::new(3).is_behind(SchemaVersion::new(3)));
        assert_eq!(SchemaVersion::new(2).to_string(), "v2");
    }

    #[test]
    fn iso_timestamp_normalizes_to_utc() {
        let ts = Iso8601Timestamp::parse("2026-09-25T10:00:00+02:00").expect("valid");
        assert_eq!(ts.as_str(), "2026-09-25T08:00:00+00:00");
    }

    #[test]
    fn iso_timestamp_rejects_garbage() {
        assert!(Iso8601Timestamp::parse("yesterday").is_err());
        assert!(Iso8601Timestamp::parse("2026-13-45T99:99:99Z").is_err());
    }

    #[test]
    fn iso_timestamp_round_trips_through_datetime() {
        let ts = Iso8601Timestamp::parse("2026-09-25T08:00:00Z").expect("valid");
        let dt = ts.to_datetime().expect("parse back");
        assert_eq!(Iso8601Timestamp::from_datetime(dt), ts);
    }
}
