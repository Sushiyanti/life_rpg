//! User-written comments attached to an existing domain entity.

use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentTargetKind {
    Player,
    Quest,
    Skill,
    SkillTree,
    Effect,
    Transaction,
    NarrativeEntry,
}
impl CommentTargetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Quest => "quest",
            Self::Skill => "skill",
            Self::SkillTree => "skill_tree",
            Self::Effect => "effect",
            Self::Transaction => "transaction",
            Self::NarrativeEntry => "narrative_entry",
        }
    }
    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "player" => Ok(Self::Player),
            "quest" => Ok(Self::Quest),
            "skill" => Ok(Self::Skill),
            "skill_tree" => Ok(Self::SkillTree),
            "effect" => Ok(Self::Effect),
            "transaction" => Ok(Self::Transaction),
            "narrative_entry" => Ok(Self::NarrativeEntry),
            _ => Err(DomainError::invalid_value(
                "comment target kind",
                "unknown target kind",
            )),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub id: Option<i64>,
    pub author_player_id: Option<EntityId>,
    pub target_kind: CommentTargetKind,
    pub target_id: EntityId,
    pub body: String,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl Comment {
    pub fn new(
        author_player_id: Option<EntityId>,
        target_kind: CommentTargetKind,
        target_id: EntityId,
        body: impl Into<String>,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        let body = body.into();
        let body = body.trim();
        if body.is_empty() {
            return Err(DomainError::invalid_value(
                "comment body",
                "must not be blank",
            ));
        }
        Ok(Self {
            id: None,
            author_player_id,
            target_kind,
            target_id,
            body: body.to_string(),
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
}
