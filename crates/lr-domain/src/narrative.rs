//! Intentional game content, intentionally distinct from user comments.

use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp, TypeRef};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NarrativeEntry {
    pub id: EntityId,
    pub player_id: EntityId,
    pub kind: TypeRef,
    pub title: String,
    pub content: String,
    pub author: Option<String>,
    pub source_kind: Option<String>,
    pub source_id: Option<String>,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl NarrativeEntry {
    pub fn new(
        id: EntityId,
        player_id: EntityId,
        kind: TypeRef,
        title: impl Into<String>,
        content: impl Into<String>,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        let title = title.into();
        let content = content.into();
        if kind.namespace != "narrative_entry" {
            return Err(DomainError::Invariant(
                "narrative kind must use `narrative_entry` namespace".into(),
            ));
        }
        if title.trim().is_empty() || content.trim().is_empty() {
            return Err(DomainError::invalid_value(
                "narrative entry",
                "title and content must not be blank",
            ));
        }
        Ok(Self {
            id,
            player_id,
            kind,
            title: title.trim().into(),
            content: content.trim().into(),
            author: None,
            source_kind: None,
            source_id: None,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
}
