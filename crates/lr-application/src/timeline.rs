//! Read-oriented chronological projection over persisted world and history records.
//!
//! This is deliberately a query vocabulary, not an event store. Each item keeps
//! its source identity and timestamp meaning so presentation cannot erase the
//! semantics of the underlying record.

use lr_domain::{DomainError, EntityId, Iso8601Timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimelineCategory {
    Session,
    Transaction,
    EffectHistory,
    Content,
    Comment,
    ConceptProgress,
    Revision,
    Snapshot,
    RecordChange,
    Lifecycle,
    RelationshipHistory,
}
impl TimelineCategory {
    pub const ALL: [Self; 11] = [
        Self::Session,
        Self::Transaction,
        Self::EffectHistory,
        Self::Content,
        Self::Comment,
        Self::ConceptProgress,
        Self::Revision,
        Self::Snapshot,
        Self::RecordChange,
        Self::Lifecycle,
        Self::RelationshipHistory,
    ];
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::Transaction => "transaction",
            Self::EffectHistory => "effect_history",
            Self::Content => "content",
            Self::Comment => "comment",
            Self::ConceptProgress => "concept_progress",
            Self::Revision => "revision",
            Self::Snapshot => "snapshot",
            Self::RecordChange => "record_change",
            Self::Lifecycle => "lifecycle",
            Self::RelationshipHistory => "relationship_history",
        }
    }
    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|value| value.as_str() == raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimelineEntityKind {
    Player,
    Concept,
    Quest,
    QuestStage,
    QuestBranch,
    QuestSession,
    SkillTree,
    Skill,
    Effect,
    Transaction,
    Comment,
    NarrativeEntry,
    ConceptProgress,
}
impl TimelineEntityKind {
    pub const ALL: [Self; 13] = [
        Self::Player,
        Self::Concept,
        Self::Quest,
        Self::QuestStage,
        Self::QuestBranch,
        Self::QuestSession,
        Self::SkillTree,
        Self::Skill,
        Self::Effect,
        Self::Transaction,
        Self::Comment,
        Self::NarrativeEntry,
        Self::ConceptProgress,
    ];
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Concept => "concept",
            Self::Quest => "quest",
            Self::QuestStage => "quest_stage",
            Self::QuestBranch => "quest_branch",
            Self::QuestSession => "quest_session",
            Self::SkillTree => "skill_tree",
            Self::Skill => "skill",
            Self::Effect => "effect",
            Self::Transaction => "transaction",
            Self::Comment => "comment",
            Self::NarrativeEntry => "narrative_entry",
            Self::ConceptProgress => "concept_progress",
        }
    }
    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|value| value.as_str() == raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineTimestampKind {
    Occurred,
    Recorded,
    Created,
    Updated,
    Captured,
    Removed,
}
impl TimelineTimestampKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Occurred => "occurred",
            Self::Recorded => "recorded",
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Captured => "captured",
            Self::Removed => "removed",
        }
    }
    pub fn parse(raw: &str) -> Option<Self> {
        Some(match raw {
            "occurred" => Self::Occurred,
            "recorded" => Self::Recorded,
            "created" => Self::Created,
            "updated" => Self::Updated,
            "captured" => Self::Captured,
            "removed" => Self::Removed,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineSort {
    Newest,
    Oldest,
}
impl TimelineSort {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Newest => "desc",
            Self::Oldest => "asc",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineQuery {
    pub player_id: EntityId,
    pub category: Option<TimelineCategory>,
    pub entity_kind: Option<TimelineEntityKind>,
    pub entity_id: Option<EntityId>,
    pub concept_id: Option<EntityId>,
    pub from: Option<Iso8601Timestamp>,
    pub through: Option<Iso8601Timestamp>,
    pub sort: TimelineSort,
    pub limit: u32,
    pub offset: u32,
}
impl TimelineQuery {
    pub const MAX_LIMIT: u32 = 200;
    pub const MAX_OFFSET: u32 = 100_000;

    pub fn validate(&self) -> Result<(), DomainError> {
        if self
            .from
            .as_ref()
            .zip(self.through.as_ref())
            .is_some_and(|(a, b)| a > b)
        {
            return Err(DomainError::invalid_value(
                "timeline range",
                "start must not be after end",
            ));
        }
        if self.limit == 0 || self.limit > Self::MAX_LIMIT {
            return Err(DomainError::invalid_value(
                "timeline limit",
                format!("must be between 1 and {}", Self::MAX_LIMIT),
            ));
        }
        if self.offset > Self::MAX_OFFSET {
            return Err(DomainError::invalid_value(
                "timeline offset",
                format!("must not exceed {}", Self::MAX_OFFSET),
            ));
        }
        Ok(())
    }
}

/// Exact persisted context for a Content relationship history fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineRelationshipContext {
    pub relationship_id: String,
    pub content_id: String,
    pub content_title: String,
    pub role_code: String,
    pub created_at: Iso8601Timestamp,
    pub removed_at: Option<Iso8601Timestamp>,
}

/// Safe compact projection; it never contains raw database rows or arbitrary JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineItem {
    /// Stable identity of this source fact; content create/update facts are distinct.
    pub source_id: String,
    pub player_id: EntityId,
    pub category: TimelineCategory,
    /// Canonical record to open. For a Comment, this is the exact commented target.
    pub entity_kind: TimelineEntityKind,
    pub entity_id: String,
    pub timestamp: Iso8601Timestamp,
    pub secondary_timestamp: Option<Iso8601Timestamp>,
    pub timestamp_kind: TimelineTimestampKind,
    pub secondary_timestamp_kind: Option<TimelineTimestampKind>,
    pub title: String,
    pub summary: String,
    pub concept_id: Option<EntityId>,
    pub type_code: Option<String>,
    /// Source-specific closed status (for example Session status or Effect history kind).
    pub state: Option<String>,
    /// Present only for explicit Content relationship history facts.
    pub relationship_context: Option<TimelineRelationshipContext>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query() -> TimelineQuery {
        TimelineQuery {
            player_id: EntityId::new("player-1").unwrap(),
            category: None,
            entity_kind: None,
            entity_id: None,
            concept_id: None,
            from: None,
            through: None,
            sort: TimelineSort::Newest,
            limit: 50,
            offset: 0,
        }
    }

    #[test]
    fn timeline_categories_and_timestamp_semantics_are_closed_and_stable() {
        assert_eq!(TimelineCategory::ALL.len(), 11);
        assert_eq!(
            TimelineCategory::parse("effect_history"),
            Some(TimelineCategory::EffectHistory)
        );
        assert_eq!(TimelineCategory::parse("made_up"), None);
        assert_eq!(
            TimelineTimestampKind::parse("captured"),
            Some(TimelineTimestampKind::Captured)
        );
        assert_eq!(TimelineTimestampKind::parse("expired"), None);
        assert_eq!(
            TimelineEntityKind::parse("quest_session"),
            Some(TimelineEntityKind::QuestSession)
        );
        assert_eq!(TimelineEntityKind::parse("timeline_events"), None);
    }

    #[test]
    fn timeline_query_bounds_ranges_and_pagination() {
        let mut q = query();
        assert!(q.validate().is_ok());
        q.limit = TimelineQuery::MAX_LIMIT;
        assert!(q.validate().is_ok());
        q.limit += 1;
        assert!(q.validate().is_err());
        q.limit = 10;
        q.offset = TimelineQuery::MAX_OFFSET + 1;
        assert!(q.validate().is_err());
        q.offset = 0;
        q.from = Some(Iso8601Timestamp::parse("2026-09-29T00:00:00Z").unwrap());
        q.through = Some(Iso8601Timestamp::parse("2026-09-28T00:00:00Z").unwrap());
        assert!(q.validate().is_err());
    }
}
