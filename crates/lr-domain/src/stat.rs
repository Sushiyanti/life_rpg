//! Data-defined stat vocabulary and per-player values.

use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp};

fn text(field: &'static str, value: impl Into<String>) -> DomainResult<String> {
    let value = value.into();
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 160 {
        return Err(DomainError::invalid_value(
            field,
            "must be non-blank and at most 160 characters",
        ));
    }
    Ok(value.to_owned())
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatDefinition {
    pub id: EntityId,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub unit: Option<String>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl StatDefinition {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: EntityId,
        code: impl Into<String>,
        name: impl Into<String>,
        description: Option<String>,
        unit: Option<String>,
        minimum: Option<f64>,
        maximum: Option<f64>,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        let code = text("stat code", code)?;
        if !code
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            || !code.as_bytes()[0].is_ascii_lowercase()
        {
            return Err(DomainError::invalid_value("stat code", "must start with a lowercase letter and contain only lowercase letters, digits, or underscores"));
        }
        if minimum.is_some_and(|v| !v.is_finite())
            || maximum.is_some_and(|v| !v.is_finite())
            || matches!((minimum, maximum), (Some(min), Some(max)) if min > max)
        {
            return Err(DomainError::invalid_value(
                "stat bounds",
                "bounds must be finite and minimum must not exceed maximum",
            ));
        }
        Ok(Self {
            id,
            code,
            name: text("stat name", name)?,
            description,
            unit: unit.map(|v| text("stat unit", v)).transpose()?,
            minimum,
            maximum,
            is_active: true,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
    pub fn validate_value(&self, value: f64) -> DomainResult<()> {
        if !value.is_finite()
            || self.minimum.is_some_and(|min| value < min)
            || self.maximum.is_some_and(|max| value > max)
        {
            return Err(DomainError::invalid_value(
                "player stat value",
                "must be finite and within its definition bounds",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerStat {
    pub player_id: EntityId,
    pub stat_code: String,
    pub current_value: f64,
    pub metadata_json: String,
    pub updated_at: Iso8601Timestamp,
}
impl PlayerStat {
    pub fn new(
        player_id: EntityId,
        definition: &StatDefinition,
        current_value: f64,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        if !definition.is_active {
            return Err(DomainError::Invariant(
                "cannot set an inactive stat definition".into(),
            ));
        }
        definition.validate_value(current_value)?;
        Ok(Self {
            player_id,
            stat_code: definition.code.clone(),
            current_value,
            metadata_json: "{}".into(),
            updated_at: now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn now() -> Iso8601Timestamp {
        Iso8601Timestamp::parse("2026-09-27T00:00:00Z").unwrap()
    }
    #[test]
    fn dynamic_definitions_validate_identity_and_bounds() {
        let definition = StatDefinition::new(
            EntityId::new("stat-focus").unwrap(),
            "programming_focus",
            "Programming Focus",
            None,
            Some("points".into()),
            Some(0.0),
            Some(10.0),
            now(),
        )
        .unwrap();
        assert!(definition.validate_value(10.0).is_ok());
        assert!(definition.validate_value(10.01).is_err());
        assert!(StatDefinition::new(
            EntityId::new("s").unwrap(),
            "Bad Code",
            "Bad",
            None,
            None,
            None,
            None,
            now()
        )
        .is_err());
        assert!(StatDefinition::new(
            EntityId::new("s").unwrap(),
            "x",
            "Bad",
            None,
            None,
            Some(10.0),
            Some(0.0),
            now()
        )
        .is_err());
    }
}
