//! Intentional game content, intentionally distinct from user comments.

use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp, TypeRef};

/// A Player-authored content record. `NarrativeEntry` is retained as the stable
/// domain name and database identity while serving the broader Content Guidebook.
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
    pub is_active: bool,
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
        let (title, content) = Self::validated_text(title, content)?;
        Self::validate_kind(&kind)?;
        Ok(Self {
            id,
            player_id,
            kind,
            title,
            content,
            author: None,
            source_kind: None,
            source_id: None,
            is_active: true,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn edit(
        &mut self,
        kind: TypeRef,
        title: impl Into<String>,
        content: impl Into<String>,
        author: Option<String>,
        source_kind: Option<String>,
        source_id: Option<String>,
        now: Iso8601Timestamp,
    ) -> DomainResult<()> {
        Self::validate_kind(&kind)?;
        let (title, content) = Self::validated_text(title, content)?;
        if source_kind.is_some() != source_id.is_some() {
            return Err(DomainError::invalid_value(
                "content source",
                "source kind and source id must either both be present or both be absent",
            ));
        }
        self.kind = kind;
        self.title = title;
        self.content = content;
        self.author = match author {
            Some(value) => non_blank(value, "content author")?,
            None => None,
        };
        self.source_kind = match source_kind {
            Some(value) => non_blank(value, "content source kind")?,
            None => None,
        };
        self.source_id = match source_id {
            Some(value) => non_blank(value, "content source id")?,
            None => None,
        };
        self.updated_at = now;
        Ok(())
    }

    fn validate_kind(kind: &TypeRef) -> DomainResult<()> {
        if kind.namespace != "narrative_entry" {
            return Err(DomainError::Invariant(
                "narrative kind must use `narrative_entry` namespace".into(),
            ));
        }
        Ok(())
    }

    fn validated_text(
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> DomainResult<(String, String)> {
        let title = title.into();
        let content = content.into();
        let title = title.trim();
        let content = content.trim();
        if title.is_empty() || content.is_empty() {
            return Err(DomainError::invalid_value(
                "narrative entry",
                "title and content must not be blank",
            ));
        }
        if title.chars().count() > 512 {
            return Err(DomainError::invalid_value(
                "content title",
                "must contain at most 512 characters",
            ));
        }
        Ok((title.into(), content.into()))
    }
}

fn non_blank(value: String, field: &'static str) -> DomainResult<Option<String>> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > 512 {
        return Err(DomainError::invalid_value(
            field,
            "must contain at most 512 characters",
        ));
    }
    Ok(Some(value.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(value: &str) -> Iso8601Timestamp {
        Iso8601Timestamp::parse(value).unwrap()
    }

    #[test]
    fn content_edits_preserve_identity_and_validate_source_pairs() {
        let mut content = NarrativeEntry::new(
            EntityId::new("content-1").unwrap(),
            EntityId::new("player-1").unwrap(),
            TypeRef::new("narrative_entry", "note").unwrap(),
            "Original",
            "Body",
            at("2026-09-28T00:00:00Z"),
        )
        .unwrap();
        assert!(content
            .edit(
                TypeRef::new("narrative_entry", "guide").unwrap(),
                "Edited",
                "Updated body",
                Some("Ada".into()),
                Some("import".into()),
                None,
                at("2026-09-28T01:00:00Z"),
            )
            .is_err());
        content
            .edit(
                TypeRef::new("narrative_entry", "guide").unwrap(),
                "Edited",
                "Updated body",
                Some("Ada".into()),
                Some("import".into()),
                Some("source-1".into()),
                at("2026-09-28T01:00:00Z"),
            )
            .unwrap();
        assert_eq!(content.id.as_str(), "content-1");
        assert_eq!(content.kind.code, "guide");
        assert_eq!(content.author.as_deref(), Some("Ada"));
        assert_eq!(content.source_id.as_deref(), Some("source-1"));
    }
}
