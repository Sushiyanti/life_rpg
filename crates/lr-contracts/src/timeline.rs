//! Stable, closed IPC DTOs for the read-only Timeline query.

use lr_application::{
    TimelineCategory, TimelineEntityKind, TimelineItem, TimelineQuery, TimelineSort,
    TimelineTimestampKind,
};
use lr_domain::{DomainError, EntityId, Iso8601Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineCategoryDto {
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
impl From<TimelineCategory> for TimelineCategoryDto {
    fn from(value: TimelineCategory) -> Self {
        match value {
            TimelineCategory::Session => Self::Session,
            TimelineCategory::Transaction => Self::Transaction,
            TimelineCategory::EffectHistory => Self::EffectHistory,
            TimelineCategory::Content => Self::Content,
            TimelineCategory::Comment => Self::Comment,
            TimelineCategory::ConceptProgress => Self::ConceptProgress,
            TimelineCategory::Revision => Self::Revision,
            TimelineCategory::Snapshot => Self::Snapshot,
            TimelineCategory::RecordChange => Self::RecordChange,
            TimelineCategory::Lifecycle => Self::Lifecycle,
            TimelineCategory::RelationshipHistory => Self::RelationshipHistory,
        }
    }
}
impl From<TimelineCategoryDto> for TimelineCategory {
    fn from(value: TimelineCategoryDto) -> Self {
        match value {
            TimelineCategoryDto::Session => Self::Session,
            TimelineCategoryDto::Transaction => Self::Transaction,
            TimelineCategoryDto::EffectHistory => Self::EffectHistory,
            TimelineCategoryDto::Content => Self::Content,
            TimelineCategoryDto::Comment => Self::Comment,
            TimelineCategoryDto::ConceptProgress => Self::ConceptProgress,
            TimelineCategoryDto::Revision => Self::Revision,
            TimelineCategoryDto::Snapshot => Self::Snapshot,
            TimelineCategoryDto::RecordChange => Self::RecordChange,
            TimelineCategoryDto::Lifecycle => Self::Lifecycle,
            TimelineCategoryDto::RelationshipHistory => Self::RelationshipHistory,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineEntityKindDto {
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
impl From<TimelineEntityKind> for TimelineEntityKindDto {
    fn from(value: TimelineEntityKind) -> Self {
        match value {
            TimelineEntityKind::Player => Self::Player,
            TimelineEntityKind::Concept => Self::Concept,
            TimelineEntityKind::Quest => Self::Quest,
            TimelineEntityKind::QuestStage => Self::QuestStage,
            TimelineEntityKind::QuestBranch => Self::QuestBranch,
            TimelineEntityKind::QuestSession => Self::QuestSession,
            TimelineEntityKind::SkillTree => Self::SkillTree,
            TimelineEntityKind::Skill => Self::Skill,
            TimelineEntityKind::Effect => Self::Effect,
            TimelineEntityKind::Transaction => Self::Transaction,
            TimelineEntityKind::Comment => Self::Comment,
            TimelineEntityKind::NarrativeEntry => Self::NarrativeEntry,
            TimelineEntityKind::ConceptProgress => Self::ConceptProgress,
        }
    }
}
impl From<TimelineEntityKindDto> for TimelineEntityKind {
    fn from(value: TimelineEntityKindDto) -> Self {
        match value {
            TimelineEntityKindDto::Player => Self::Player,
            TimelineEntityKindDto::Concept => Self::Concept,
            TimelineEntityKindDto::Quest => Self::Quest,
            TimelineEntityKindDto::QuestStage => Self::QuestStage,
            TimelineEntityKindDto::QuestBranch => Self::QuestBranch,
            TimelineEntityKindDto::QuestSession => Self::QuestSession,
            TimelineEntityKindDto::SkillTree => Self::SkillTree,
            TimelineEntityKindDto::Skill => Self::Skill,
            TimelineEntityKindDto::Effect => Self::Effect,
            TimelineEntityKindDto::Transaction => Self::Transaction,
            TimelineEntityKindDto::Comment => Self::Comment,
            TimelineEntityKindDto::NarrativeEntry => Self::NarrativeEntry,
            TimelineEntityKindDto::ConceptProgress => Self::ConceptProgress,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineTimestampKindDto {
    Occurred,
    Recorded,
    Created,
    Updated,
    Captured,
    Removed,
}
impl From<TimelineTimestampKind> for TimelineTimestampKindDto {
    fn from(value: TimelineTimestampKind) -> Self {
        match value {
            TimelineTimestampKind::Occurred => Self::Occurred,
            TimelineTimestampKind::Recorded => Self::Recorded,
            TimelineTimestampKind::Created => Self::Created,
            TimelineTimestampKind::Updated => Self::Updated,
            TimelineTimestampKind::Captured => Self::Captured,
            TimelineTimestampKind::Removed => Self::Removed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineSortDto {
    Newest,
    Oldest,
}
impl From<TimelineSortDto> for TimelineSort {
    fn from(value: TimelineSortDto) -> Self {
        match value {
            TimelineSortDto::Newest => Self::Newest,
            TimelineSortDto::Oldest => Self::Oldest,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineQueryDto {
    pub player_id: String,
    pub category: Option<TimelineCategoryDto>,
    pub entity_kind: Option<TimelineEntityKindDto>,
    pub concept_id: Option<String>,
    pub from: Option<String>,
    pub through: Option<String>,
    pub sort: TimelineSortDto,
    pub limit: u32,
    pub offset: u32,
}
impl TryFrom<TimelineQueryDto> for TimelineQuery {
    type Error = DomainError;
    fn try_from(dto: TimelineQueryDto) -> Result<Self, Self::Error> {
        let query = Self {
            player_id: EntityId::new(dto.player_id)?,
            category: dto.category.map(Into::into),
            entity_kind: dto.entity_kind.map(Into::into),
            concept_id: dto.concept_id.map(EntityId::new).transpose()?,
            from: dto.from.map(Iso8601Timestamp::parse).transpose()?,
            through: dto.through.map(Iso8601Timestamp::parse).transpose()?,
            sort: dto.sort.into(),
            limit: dto.limit,
            offset: dto.offset,
        };
        query.validate()?;
        Ok(query)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineItemDto {
    pub source_id: String,
    pub player_id: String,
    pub category: TimelineCategoryDto,
    pub entity_kind: TimelineEntityKindDto,
    pub entity_id: String,
    pub timestamp: String,
    pub secondary_timestamp: Option<String>,
    pub timestamp_kind: TimelineTimestampKindDto,
    pub secondary_timestamp_kind: Option<TimelineTimestampKindDto>,
    pub title: String,
    pub summary: String,
    pub concept_id: Option<String>,
    pub type_code: Option<String>,
    pub state: Option<String>,
}
impl From<TimelineItem> for TimelineItemDto {
    fn from(value: TimelineItem) -> Self {
        Self {
            source_id: value.source_id,
            player_id: value.player_id.into_string(),
            category: value.category.into(),
            entity_kind: value.entity_kind.into(),
            entity_id: value.entity_id,
            timestamp: value.timestamp.into_string(),
            secondary_timestamp: value.secondary_timestamp.map(Iso8601Timestamp::into_string),
            timestamp_kind: value.timestamp_kind.into(),
            secondary_timestamp_kind: value.secondary_timestamp_kind.map(Into::into),
            title: value.title,
            summary: value.summary,
            concept_id: value.concept_id.map(EntityId::into_string),
            type_code: value.type_code,
            state: value.state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> TimelineQueryDto {
        TimelineQueryDto {
            player_id: "player-1".into(),
            category: Some(TimelineCategoryDto::ConceptProgress),
            entity_kind: Some(TimelineEntityKindDto::Concept),
            concept_id: Some("concept-1".into()),
            from: Some("2026-09-01T00:00:00Z".into()),
            through: Some("2026-09-30T23:59:59Z".into()),
            sort: TimelineSortDto::Oldest,
            limit: 25,
            offset: 50,
        }
    }

    #[test]
    fn typed_query_serializes_filters_and_pagination_in_camel_case() {
        let json = serde_json::to_value(request()).unwrap();
        assert_eq!(json["category"], "concept_progress");
        assert_eq!(json["entityKind"], "concept");
        assert_eq!(json["conceptId"], "concept-1");
        assert_eq!(json["sort"], "oldest");
        assert_eq!(json["limit"], 25);
        assert_eq!(json["offset"], 50);
        assert!(json.get("player_id").is_none());
    }

    #[test]
    fn request_conversion_checks_closed_ranges_and_bounded_paging() {
        let query = TimelineQuery::try_from(request()).unwrap();
        assert_eq!(query.category, Some(TimelineCategory::ConceptProgress));
        assert_eq!(query.offset, 50);
        let mut invalid = request();
        invalid.limit = TimelineQuery::MAX_LIMIT + 1;
        assert!(TimelineQuery::try_from(invalid).is_err());
        let mut invalid_time = request();
        invalid_time.from = Some("not a date".into());
        assert!(TimelineQuery::try_from(invalid_time).is_err());
    }

    #[test]
    fn item_contract_contains_semantic_timestamps_and_no_arbitrary_json() {
        let item = TimelineItem {
            source_id: "session:session-1".into(),
            player_id: EntityId::new("player-1").unwrap(),
            category: TimelineCategory::Session,
            entity_kind: TimelineEntityKind::QuestSession,
            entity_id: "session-1".into(),
            timestamp: Iso8601Timestamp::parse("2026-09-28T10:00:00Z").unwrap(),
            secondary_timestamp: Some(Iso8601Timestamp::parse("2026-09-28T11:00:00Z").unwrap()),
            timestamp_kind: TimelineTimestampKind::Occurred,
            secondary_timestamp_kind: Some(TimelineTimestampKind::Occurred),
            title: "Session · Garden".into(),
            summary: "Prepared seeds".into(),
            concept_id: None,
            type_code: None,
            state: Some("completed".into()),
        };
        let json = serde_json::to_value(TimelineItemDto::from(item)).unwrap();
        for key in [
            "sourceId",
            "playerId",
            "category",
            "entityKind",
            "entityId",
            "timestamp",
            "secondaryTimestamp",
            "timestampKind",
            "secondaryTimestampKind",
            "title",
            "summary",
            "conceptId",
            "typeCode",
            "state",
        ] {
            assert!(json.get(key).is_some(), "missing {key}");
        }
        assert_eq!(json["timestampKind"], "occurred");
        assert_eq!(json["secondaryTimestampKind"], "occurred");
        assert!(json.get("snapshotJson").is_none());
    }
}
