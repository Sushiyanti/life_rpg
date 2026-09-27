//! Immutable daily historical state snapshots.

use crate::skill::SkillStatus;
use crate::{DateValue, DomainError, DomainResult, EntityId, Iso8601Timestamp};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerStateSnapshot {
    pub id: Option<i64>,
    pub player_id: EntityId,
    pub snapshot_date: DateValue,
    pub level: i32,
    pub current_xp: i64,
    pub state_json: String,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
}
impl PlayerStateSnapshot {
    pub fn new(
        player_id: EntityId,
        snapshot_date: DateValue,
        level: i32,
        current_xp: i64,
        created_at: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        if level < 1 {
            return Err(DomainError::invalid_value(
                "snapshot level",
                "must be at least one",
            ));
        }
        Ok(Self {
            id: None,
            player_id,
            snapshot_date,
            level,
            current_xp,
            state_json: "{}".into(),
            metadata_json: "{}".into(),
            created_at,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillStateSnapshot {
    pub id: Option<i64>,
    pub skill_id: EntityId,
    pub snapshot_date: DateValue,
    pub level: i32,
    pub current_xp: i64,
    pub status: SkillStatus,
    pub invested_minutes: i64,
    pub state_json: String,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
}
impl SkillStateSnapshot {
    pub fn new(
        skill_id: EntityId,
        snapshot_date: DateValue,
        level: i32,
        current_xp: i64,
        status: SkillStatus,
        invested_minutes: i64,
        created_at: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        if level < 1 || invested_minutes < 0 {
            return Err(DomainError::invalid_value(
                "skill snapshot",
                "invalid level or invested minutes",
            ));
        }
        Ok(Self {
            id: None,
            skill_id,
            snapshot_date,
            level,
            current_xp,
            status,
            invested_minutes,
            state_json: "{}".into(),
            metadata_json: "{}".into(),
            created_at,
        })
    }
}
