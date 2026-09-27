//! Explicit Concept associations; a closed set of target kinds, not a graph engine.
use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssociatedEntityKind {
    Quest,
    Stage,
    Branch,
    Session,
    Skill,
    SkillTree,
    Effect,
    Transaction,
    Comment,
    Content,
    ConceptProgress,
}
impl AssociatedEntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Quest => "quest",
            Self::Stage => "quest_stage",
            Self::Branch => "quest_branch",
            Self::Session => "quest_session",
            Self::Skill => "skill",
            Self::SkillTree => "skill_tree",
            Self::Effect => "effect",
            Self::Transaction => "transaction",
            Self::Comment => "comment",
            Self::Content => "narrative_entry",
            Self::ConceptProgress => "concept_progress",
        }
    }
    pub fn parse(v: &str) -> DomainResult<Self> {
        match v {
            "quest" => Ok(Self::Quest),
            "quest_stage" => Ok(Self::Stage),
            "quest_branch" => Ok(Self::Branch),
            "quest_session" => Ok(Self::Session),
            "skill" => Ok(Self::Skill),
            "skill_tree" => Ok(Self::SkillTree),
            "effect" => Ok(Self::Effect),
            "transaction" => Ok(Self::Transaction),
            "comment" => Ok(Self::Comment),
            "narrative_entry" => Ok(Self::Content),
            "concept_progress" => Ok(Self::ConceptProgress),
            _ => Err(DomainError::invalid_value(
                "association target",
                "unsupported kind",
            )),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptAssociation {
    pub id: EntityId,
    pub player_id: EntityId,
    pub concept_id: EntityId,
    pub entity_kind: AssociatedEntityKind,
    pub entity_id: String,
    pub association_code: String,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl ConceptAssociation {
    pub fn new(
        id: EntityId,
        player_id: EntityId,
        concept_id: EntityId,
        entity_kind: AssociatedEntityKind,
        entity_id: impl Into<String>,
        association_code: impl Into<String>,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        let entity_id = entity_id.into();
        let association_code = association_code.into();
        if entity_id.trim().is_empty()
            || entity_id.len() > 160
            || association_code.is_empty()
            || association_code.len() > 160
            || !association_code
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        {
            return Err(DomainError::invalid_value(
                "Concept association",
                "requires an entity ID and lowercase role code",
            ));
        }
        Ok(Self {
            id,
            player_id,
            concept_id,
            entity_kind,
            entity_id,
            association_code,
            is_active: true,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
}
