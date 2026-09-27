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
    pub current_xp: i64,
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
            current_xp: 0,
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
