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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    pub id: EntityId,
    pub name: String,
    pub description: Option<String>,
    pub level: i32,
    pub level_name: Option<String>,
    pub progression_label: Option<String>,
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
            level_name: None,
            progression_label: None,
            current_xp: 0,
            is_active: true,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
    pub fn apply_xp(&mut self, amount: i64, now: Iso8601Timestamp) -> DomainResult<()> {
        let next_xp = self
            .current_xp
            .checked_add(amount)
            .ok_or_else(|| DomainError::Invariant("XP overflow".into()))?;
        self.current_xp = next_xp.max(0);
        self.updated_at = now;
        Ok(())
    }
    /// Set player-authored progression. XP changes never call this implicitly.
    pub fn set_progression(
        &mut self,
        level: i32,
        level_name: Option<String>,
        progression_label: Option<String>,
        now: Iso8601Timestamp,
    ) -> DomainResult<()> {
        if level < 1 {
            return Err(DomainError::invalid_value(
                "player level",
                "must be positive",
            ));
        }
        self.level_name = level_name.map(|v| text("level name", v)).transpose()?;
        self.progression_label = progression_label
            .map(|v| text("progression label", v))
            .transpose()?;
        self.level = level;
        self.updated_at = now;
        Ok(())
    }
    pub fn rename(&mut self, name: impl Into<String>, now: Iso8601Timestamp) -> DomainResult<()> {
        self.name = text("player name", name)?;
        self.updated_at = now;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn now() -> Iso8601Timestamp {
        Iso8601Timestamp::parse("2026-09-27T00:00:00Z").unwrap()
    }
    #[test]
    fn xp_penalties_floor_at_zero_and_awards_progress_normally() {
        let mut player = Player::new(EntityId::new("p1").unwrap(), "Ada", now()).unwrap();
        player.apply_xp(10, now()).unwrap();
        assert_eq!(player.current_xp, 10);
        player.apply_xp(-4, now()).unwrap();
        assert_eq!(player.current_xp, 6);
        player.apply_xp(-25, now()).unwrap();
        assert_eq!(player.current_xp, 0);
        player.apply_xp(-25, now()).unwrap();
        assert_eq!(player.current_xp, 0);
        player.apply_xp(1_000, now()).unwrap();
        assert_eq!((player.current_xp, player.level), (1_000, 1));
        player
            .set_progression(2, Some("Journeyman".into()), Some("Learning".into()), now())
            .unwrap();
        assert_eq!(
            (player.level, player.level_name.as_deref()),
            (2, Some("Journeyman"))
        );
        player.set_progression(1, None, None, now()).unwrap();
        assert_eq!(
            player.level, 1,
            "manual level edits can move progression backward"
        );
    }
}
