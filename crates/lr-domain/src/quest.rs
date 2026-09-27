//! Generic, hierarchical quests; subtype semantics live in the type registry.

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
pub enum QuestStatus {
    Open,
    Active,
    Completed,
    Abandoned,
}
impl QuestStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
        }
    }
    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "open" => Ok(Self::Open),
            "active" => Ok(Self::Active),
            "completed" => Ok(Self::Completed),
            "abandoned" => Ok(Self::Abandoned),
            _ => Err(DomainError::invalid_value("quest status", "unknown status")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quest {
    pub id: EntityId,
    pub player_id: EntityId,
    pub quest_type: TypeRef,
    pub parent_quest_id: Option<EntityId>,
    pub skill_id: Option<EntityId>,
    pub title: String,
    pub description: Option<String>,
    pub story: Option<String>,
    pub instructions: Option<String>,
    pub status: QuestStatus,
    pub difficulty: Option<i32>,
    pub progress: i32,
    pub xp_reward: i64,
    pub due_at: Option<Iso8601Timestamp>,
    pub started_at: Option<Iso8601Timestamp>,
    pub completed_at: Option<Iso8601Timestamp>,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl Quest {
    pub fn new(
        id: EntityId,
        player_id: EntityId,
        quest_type: TypeRef,
        title: impl Into<String>,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        if quest_type.namespace != "quest" {
            return Err(DomainError::Invariant(
                "quest type must use the `quest` namespace".into(),
            ));
        }
        Ok(Self {
            id,
            player_id,
            quest_type,
            parent_quest_id: None,
            skill_id: None,
            title: required("quest title", title)?,
            description: None,
            story: None,
            instructions: None,
            status: QuestStatus::Open,
            difficulty: None,
            progress: 0,
            xp_reward: 0,
            due_at: None,
            started_at: None,
            completed_at: None,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
    pub fn set_progress(&mut self, progress: i32, now: Iso8601Timestamp) -> DomainResult<()> {
        if !(0..=100).contains(&progress) {
            return Err(DomainError::invalid_value(
                "quest progress",
                "must be 0 through 100",
            ));
        }
        self.progress = progress;
        self.updated_at = now;
        Ok(())
    }
    pub fn start(&mut self, now: Iso8601Timestamp) -> DomainResult<()> {
        if self.status != QuestStatus::Open {
            return Err(DomainError::Invariant(
                "only an open quest can start".into(),
            ));
        }
        self.status = QuestStatus::Active;
        self.started_at = Some(now.clone());
        self.updated_at = now;
        Ok(())
    }
    pub fn complete(&mut self, now: Iso8601Timestamp) -> DomainResult<()> {
        if matches!(self.status, QuestStatus::Completed | QuestStatus::Abandoned) {
            return Err(DomainError::Invariant(
                "quest is no longer completable".into(),
            ));
        }
        self.status = QuestStatus::Completed;
        self.progress = 100;
        self.completed_at = Some(now.clone());
        self.updated_at = now;
        Ok(())
    }
}
