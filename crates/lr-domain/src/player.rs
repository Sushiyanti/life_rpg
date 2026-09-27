//! The Player aggregate: cached present state, distinct from the transaction ledger.

use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp};

fn text(field: &'static str, value: impl Into<String>) -> DomainResult<String> {
    let value = value.into();
    let v = value.trim();
    if v.is_empty() {
        return Err(DomainError::invalid_value(field, "must not be blank"));
    }
    if v.chars().count() > 500 {
        return Err(DomainError::invalid_value(field, "is too long"));
    }
    Ok(v.to_string())
}

/// XP per level in the Phase 2 cached progression model. Rules-engine work is deferred.
pub const XP_PER_LEVEL: i64 = 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    pub id: EntityId,
    pub name: String,
    pub description: Option<String>,
    pub level: i32,
    pub current_xp: i64,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl Player {
    pub fn new(id: EntityId, name: impl Into<String>, now: Iso8601Timestamp) -> DomainResult<Self> {
        Ok(Self {
            id,
            name: text("player name", name)?,
            description: None,
            level: 1,
            current_xp: 0,
            is_active: true,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
    pub fn apply_xp(&mut self, amount: i64, now: Iso8601Timestamp) -> DomainResult<()> {
        self.current_xp = self
            .current_xp
            .checked_add(amount)
            .ok_or_else(|| DomainError::Invariant("XP overflow".into()))?;
        self.level = (self.current_xp.div_euclid(XP_PER_LEVEL) + 1).max(1) as i32;
        self.updated_at = now;
        Ok(())
    }
    pub fn rename(&mut self, name: impl Into<String>, now: Iso8601Timestamp) -> DomainResult<()> {
        self.name = text("player name", name)?;
        self.updated_at = now;
        Ok(())
    }
}
