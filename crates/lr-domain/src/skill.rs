//! Skill trees and hierarchical skills.

use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp, TypeRef};
fn required(field: &'static str, value: impl Into<String>) -> DomainResult<String> {
    let value = value.into();
    let v = value.trim();
    if v.is_empty() {
        Err(DomainError::invalid_value(field, "must not be blank"))
    } else {
        Ok(v.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillStatus {
    Active,
    Paused,
    Completed,
    Archived,
}
impl SkillStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }
    pub fn parse(s: &str) -> DomainResult<Self> {
        match s {
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "archived" => Ok(Self::Archived),
            _ => Err(DomainError::invalid_value("skill status", "unknown status")),
        }
    }
}

/// Availability is independent of the Skill's active/paused/completed lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillAvailability {
    Locked,
    Available,
}
impl SkillAvailability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Locked => "locked",
            Self::Available => "available",
        }
    }
    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "locked" => Ok(Self::Locked),
            "available" => Ok(Self::Available),
            _ => Err(DomainError::invalid_value(
                "Skill availability",
                "unknown value",
            )),
        }
    }
}

/// The Player may explicitly delegate unlock authority to Rules per Skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillAvailabilityControl {
    Manual,
    RuleControlled,
}
impl SkillAvailabilityControl {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::RuleControlled => "rule_controlled",
        }
    }
    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "manual" => Ok(Self::Manual),
            "rule_controlled" => Ok(Self::RuleControlled),
            _ => Err(DomainError::invalid_value(
                "Skill availability control",
                "unknown value",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillTree {
    pub id: EntityId,
    pub player_id: EntityId,
    pub tree_type: TypeRef,
    pub name: String,
    pub description: Option<String>,
    pub story: Option<String>,
    pub instructions: Option<String>,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl SkillTree {
    pub fn new(
        id: EntityId,
        player_id: EntityId,
        tree_type: TypeRef,
        name: impl Into<String>,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        if tree_type.namespace != "skill_tree" {
            return Err(DomainError::Invariant(
                "skill tree type must use `skill_tree` namespace".into(),
            ));
        }
        Ok(Self {
            id,
            player_id,
            tree_type,
            name: required("skill tree name", name)?,
            description: None,
            story: None,
            instructions: None,
            is_active: true,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    pub id: EntityId,
    pub skill_tree_id: EntityId,
    pub parent_skill_id: Option<EntityId>,
    pub skill_type: TypeRef,
    pub name: String,
    pub description: Option<String>,
    pub story: Option<String>,
    pub instructions: Option<String>,
    pub level: i32,
    pub level_name: Option<String>,
    pub progression_label: Option<String>,
    pub current_xp: i64,
    pub availability: SkillAvailability,
    pub availability_control: SkillAvailabilityControl,
    pub invested_minutes: i64,
    pub status: SkillStatus,
    pub started_at: Option<Iso8601Timestamp>,
    pub completed_at: Option<Iso8601Timestamp>,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl Skill {
    pub fn set_parent(&mut self, parent: Option<EntityId>) -> DomainResult<()> {
        if parent.as_ref() == Some(&self.id) {
            return Err(DomainError::Invariant("skill cannot parent itself".into()));
        }
        self.parent_skill_id = parent;
        Ok(())
    }
    pub fn new(
        id: EntityId,
        skill_tree_id: EntityId,
        skill_type: TypeRef,
        name: impl Into<String>,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        if skill_type.namespace != "skill" {
            return Err(DomainError::Invariant(
                "skill type must use `skill` namespace".into(),
            ));
        }
        Ok(Self {
            id,
            skill_tree_id,
            parent_skill_id: None,
            skill_type,
            name: required("skill name", name)?,
            description: None,
            story: None,
            instructions: None,
            level: 1,
            level_name: None,
            progression_label: None,
            current_xp: 0,
            availability: SkillAvailability::Available,
            availability_control: SkillAvailabilityControl::Manual,
            invested_minutes: 0,
            status: SkillStatus::Active,
            started_at: Some(now.clone()),
            completed_at: None,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
    pub fn invest_time(&mut self, minutes: i64, now: Iso8601Timestamp) -> DomainResult<()> {
        if minutes <= 0 {
            return Err(DomainError::invalid_value(
                "invested minutes",
                "must be positive",
            ));
        }
        self.invested_minutes = self
            .invested_minutes
            .checked_add(minutes)
            .ok_or_else(|| DomainError::Invariant("invested time overflow".into()))?;
        self.updated_at = now;
        Ok(())
    }
    /// Skill level is player-authored; XP is optional and never implies a level.
    /// Apply Skill XP with a zero floor. The Player-authored level is untouched.
    pub fn adjust_xp(&mut self, requested: i64, now: Iso8601Timestamp) -> DomainResult<i64> {
        if requested == 0 {
            return Err(DomainError::invalid_value(
                "Skill XP adjustment",
                "must not be zero",
            ));
        }
        let attempted = self
            .current_xp
            .checked_add(requested)
            .ok_or_else(|| DomainError::Invariant("Skill XP adjustment overflow".into()))?;
        let next = attempted.max(0);
        let applied = next - self.current_xp;
        self.current_xp = next;
        self.updated_at = now;
        Ok(applied)
    }
    /// An explicit Player lock/unlock reclaims availability authority.
    pub fn set_availability(&mut self, value: SkillAvailability, now: Iso8601Timestamp) {
        self.availability = value;
        self.availability_control = SkillAvailabilityControl::Manual;
        self.updated_at = now;
    }
    pub fn set_availability_control(
        &mut self,
        value: SkillAvailabilityControl,
        now: Iso8601Timestamp,
    ) {
        self.availability_control = value;
        self.updated_at = now;
    }
    /// A Rule may unlock only after the Player has explicitly delegated this axis.
    /// Returns `false` for an already available Skill and never changes level.
    pub fn unlock_by_rule(&mut self, now: Iso8601Timestamp) -> DomainResult<bool> {
        if self.availability == SkillAvailability::Available {
            return Ok(false);
        }
        if self.availability_control != SkillAvailabilityControl::RuleControlled {
            return Err(DomainError::Invariant(
                "manually controlled Skill availability cannot be changed by a Rule".into(),
            ));
        }
        self.availability = SkillAvailability::Available;
        self.updated_at = now;
        Ok(true)
    }
    pub fn set_progression(
        &mut self,
        level: i32,
        level_name: Option<String>,
        progression_label: Option<String>,
        now: Iso8601Timestamp,
    ) -> DomainResult<()> {
        if level < 1 {
            return Err(DomainError::invalid_value(
                "skill level",
                "must be positive",
            ));
        }
        self.level_name = level_name.map(|v| required("level name", v)).transpose()?;
        self.progression_label = progression_label
            .map(|v| required("progression label", v))
            .transpose()?;
        self.level = level;
        self.updated_at = now;
        Ok(())
    }
    pub fn complete(&mut self, now: Iso8601Timestamp) -> DomainResult<()> {
        if self.status == SkillStatus::Completed {
            return Err(DomainError::Invariant("skill is already complete".into()));
        }
        self.status = SkillStatus::Completed;
        self.completed_at = Some(now.clone());
        self.updated_at = now;
        Ok(())
    }
}

#[cfg(test)]
mod phase10_tests {
    use super::*;

    fn time() -> Iso8601Timestamp {
        Iso8601Timestamp::parse("2026-09-28T12:00:00Z").unwrap()
    }
    fn skill() -> Skill {
        Skill::new(
            EntityId::new("skill-1").unwrap(),
            EntityId::new("tree-1").unwrap(),
            TypeRef::skill("core").unwrap(),
            "Practice",
            time(),
        )
        .unwrap()
    }

    #[test]
    fn skill_xp_has_a_zero_floor_and_never_changes_player_authored_level() {
        let mut value = skill();
        value
            .set_progression(7, Some("Expert".into()), Some("Authored".into()), time())
            .unwrap();
        assert_eq!(value.adjust_xp(25, time()).unwrap(), 25);
        assert_eq!(value.current_xp, 25);
        assert_eq!(value.adjust_xp(-40, time()).unwrap(), -25);
        assert_eq!(value.current_xp, 0);
        assert_eq!(value.level, 7);
        assert_eq!(value.level_name.as_deref(), Some("Expert"));
        assert!(value.adjust_xp(0, time()).is_err());
        value.current_xp = i64::MAX;
        assert!(value.adjust_xp(1, time()).is_err());
        assert_eq!(
            value.current_xp,
            i64::MAX,
            "overflow must not partially mutate XP"
        );
    }

    #[test]
    fn manual_lock_is_separate_from_lifecycle_and_rule_authority_is_explicit() {
        let mut value = skill();
        value.set_availability(SkillAvailability::Locked, time());
        assert_eq!(value.availability, SkillAvailability::Locked);
        assert_eq!(value.status, SkillStatus::Active);
        assert!(
            value.unlock_by_rule(time()).is_err(),
            "a manually controlled lock must fail closed"
        );

        value.set_availability_control(SkillAvailabilityControl::RuleControlled, time());
        assert!(value.unlock_by_rule(time()).unwrap());
        assert_eq!(value.availability, SkillAvailability::Available);
        assert!(
            !value.unlock_by_rule(time()).unwrap(),
            "an already-unlocked Skill is a deterministic no-op"
        );

        value.set_availability(SkillAvailability::Locked, time());
        assert_eq!(value.availability_control, SkillAvailabilityControl::Manual);
        assert!(
            value.unlock_by_rule(time()).is_err(),
            "manual lock reclaims authority"
        );
    }
}
