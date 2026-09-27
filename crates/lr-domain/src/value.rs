//! Validated value objects.
//!
//! These are the smallest pieces of the world model. They exist so that later
//! aggregates (`Player`, `Quest`, `Skill`, `Transaction`, …) can be built out
//! of primitives that cannot hold nonsense.

use crate::error::{DomainError, DomainResult};

/// Maximum length accepted for an opaque entity identifier.
pub const MAX_IDENTIFIER_LEN: usize = 128;

/// Stable identity of a domain entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityId(String);

impl EntityId {
    /// Validate and wrap an identifier.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaVersion(u32);
impl SchemaVersion {
    pub const ZERO: Self = Self(0);
    pub const fn new(version: u32) -> Self {
        Self(version)
    }
    pub const fn get(self) -> u32 {
        self.0
    }
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

/// A timestamp guaranteed to be a valid RFC 3339 / ISO 8601 instant in UTC.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Iso8601Timestamp(String);
impl Iso8601Timestamp {
    pub fn parse(raw: impl Into<String>) -> DomainResult<Self> {
        let raw = raw.into();
        let parsed = chrono::DateTime::parse_from_rfc3339(raw.trim()).map_err(|err| {
            DomainError::invalid_value("Iso8601Timestamp", format!("not RFC 3339 ({err})"))
        })?;
        Ok(Self(parsed.to_utc().to_rfc3339()))
    }
    pub fn from_datetime(dt: chrono::DateTime<chrono::Utc>) -> Self {
        Self(dt.to_rfc3339())
    }
    pub fn now() -> Self {
        Self(chrono::Utc::now().to_rfc3339())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn into_string(self) -> String {
        self.0
    }
    pub fn to_datetime(&self) -> DomainResult<chrono::DateTime<chrono::Utc>> {
        chrono::DateTime::parse_from_rfc3339(&self.0)
            .map(|dt| dt.to_utc())
            .map_err(|err| {
                DomainError::invalid_value(
                    "Iso8601Timestamp",
                    format!("stored value is corrupt ({err})"),
                )
            })
    }
}
impl std::fmt::Display for Iso8601Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A calendar date (`YYYY-MM-DD`) without a time of day.
///
/// Daily snapshots intentionally key on this value rather than an instant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DateValue(String);
impl DateValue {
    pub fn parse(raw: impl Into<String>) -> DomainResult<Self> {
        let raw = raw.into();
        let parsed = chrono::NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d").map_err(|err| {
            DomainError::invalid_value("DateValue", format!("not a YYYY-MM-DD date ({err})"))
        })?;
        Ok(Self(parsed.format("%Y-%m-%d").to_string()))
    }
    pub fn today() -> Self {
        Self(chrono::Utc::now().format("%Y-%m-%d").to_string())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn into_string(self) -> String {
        self.0
    }
}
impl std::fmt::Display for DateValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn entity_id_validates() {
        assert_eq!(EntityId::new(" player-1 ").unwrap().as_str(), "player-1");
        assert!(EntityId::new(" ").is_err());
        assert!(EntityId::new("x".repeat(MAX_IDENTIFIER_LEN + 1)).is_err());
    }
    #[test]
    fn schema_version_orders() {
        assert!(SchemaVersion::ZERO.is_behind(SchemaVersion::new(1)));
    }
    #[test]
    fn timestamp_normalizes() {
        assert_eq!(
            Iso8601Timestamp::parse("2026-09-25T10:00:00+02:00")
                .unwrap()
                .as_str(),
            "2026-09-25T08:00:00+00:00"
        );
    }
    #[test]
    fn date_validates() {
        assert_eq!(
            DateValue::parse(" 2026-09-25 ").unwrap().as_str(),
            "2026-09-25"
        );
        assert!(DateValue::parse("2026-02-30").is_err());
    }
}
