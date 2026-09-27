//! Bounded lifecycle and revision primitives for recoverable, typed world records.
use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Active,
    Archived,
    Trashed,
}
impl LifecycleState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
            Self::Trashed => "trashed",
        }
    }
    pub fn parse(v: &str) -> DomainResult<Self> {
        match v {
            "active" => Ok(Self::Active),
            "archived" => Ok(Self::Archived),
            "trashed" => Ok(Self::Trashed),
            _ => Err(DomainError::invalid_value(
                "lifecycle state",
                "unknown state",
            )),
        }
    }
}

/// Only supported domain aggregates may have revisions; this is not an arbitrary table registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevisionTargetKind {
    Player,
    Quest,
    SkillTree,
    Skill,
    Concept,
    ConceptProgress,
    QuestStage,
    QuestBranch,
    QuestSession,
    NarrativeEntry,
}
impl RevisionTargetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Quest => "quest",
            Self::SkillTree => "skill_tree",
            Self::Skill => "skill",
            Self::Concept => "concept",
            Self::ConceptProgress => "concept_progress",
            Self::QuestStage => "quest_stage",
            Self::QuestBranch => "quest_branch",
            Self::QuestSession => "quest_session",
            Self::NarrativeEntry => "narrative_entry",
        }
    }
    pub fn parse(v: &str) -> DomainResult<Self> {
        match v {
            "player" => Ok(Self::Player),
            "quest" => Ok(Self::Quest),
            "skill_tree" => Ok(Self::SkillTree),
            "skill" => Ok(Self::Skill),
            "concept" => Ok(Self::Concept),
            "concept_progress" => Ok(Self::ConceptProgress),
            "quest_stage" => Ok(Self::QuestStage),
            "quest_branch" => Ok(Self::QuestBranch),
            "quest_session" => Ok(Self::QuestSession),
            "narrative_entry" => Ok(Self::NarrativeEntry),
            _ => Err(DomainError::invalid_value(
                "revision target",
                "unsupported entity kind",
            )),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityRevision {
    pub id: EntityId,
    pub player_id: EntityId,
    pub target_kind: RevisionTargetKind,
    pub target_id: EntityId,
    pub revision_number: u32,
    pub recorded_at: Iso8601Timestamp,
    pub author_player_id: Option<EntityId>,
    pub reason: Option<String>,
    pub snapshot_json: String,
    pub metadata_json: String,
}
impl EntityRevision {
    pub fn new(
        id: EntityId,
        player_id: EntityId,
        target_kind: RevisionTargetKind,
        target_id: EntityId,
        revision_number: u32,
        recorded_at: Iso8601Timestamp,
        author_player_id: Option<EntityId>,
        reason: Option<String>,
        snapshot_json: impl Into<String>,
    ) -> DomainResult<Self> {
        if revision_number == 0 {
            return Err(DomainError::invalid_value(
                "revision number",
                "must be positive",
            ));
        }
        let snapshot_json = snapshot_json.into();
        serde_json::from_str::<serde_json::Value>(&snapshot_json)
            .map_err(|_| DomainError::invalid_value("revision snapshot", "must be valid JSON"))?;
        if reason.as_ref().is_some_and(|s| s.chars().count() > 512) {
            return Err(DomainError::invalid_value(
                "revision reason",
                "must be at most 512 characters",
            ));
        }
        Ok(Self {
            id,
            player_id,
            target_kind,
            target_id,
            revision_number,
            recorded_at,
            author_player_id,
            reason,
            snapshot_json,
            metadata_json: "{}".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lifecycle_and_revision_types_fail_closed() {
        assert_eq!(
            LifecycleState::parse("trashed").unwrap(),
            LifecycleState::Trashed
        );
        assert!(LifecycleState::parse("deleted_forever").is_err());
        assert!(RevisionTargetKind::parse("anything").is_err());
        let t = Iso8601Timestamp::parse("2026-09-27T00:00:00Z").unwrap();
        assert!(EntityRevision::new(
            EntityId::new("r").unwrap(),
            EntityId::new("p").unwrap(),
            RevisionTargetKind::Quest,
            EntityId::new("q").unwrap(),
            1,
            t,
            None,
            None,
            "not-json"
        )
        .is_err());
    }
}
