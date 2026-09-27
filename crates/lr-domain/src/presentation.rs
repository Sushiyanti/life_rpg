//! Presentation preferences are contextual and do not change whether a world entity exists.
use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationPreference {
    pub player_id: EntityId,
    pub entity_kind: String,
    pub entity_id: EntityId,
    pub context: String,
    pub is_visible: bool,
    pub sort_order: i32,
    pub is_pinned: bool,
    pub is_collapsed: Option<bool>,
    pub variant: Option<String>,
    pub density: Option<String>,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl PresentationPreference {
    pub fn new(
        player_id: EntityId,
        entity_kind: impl Into<String>,
        entity_id: EntityId,
        context: impl Into<String>,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        let entity_kind = entity_kind.into();
        let context = context.into();
        for (field, value) in [
            ("presentation entity kind", entity_kind.as_str()),
            ("presentation context", context.as_str()),
        ] {
            if value.is_empty()
                || value.len() > 160
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            {
                return Err(DomainError::invalid_value(
                    field,
                    "use 1-160 lowercase ASCII letters, digits, or underscores",
                ));
            }
        }
        Ok(Self {
            player_id,
            entity_kind,
            entity_id,
            context,
            is_visible: true,
            sort_order: 0,
            is_pinned: false,
            is_collapsed: None,
            variant: None,
            density: None,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presentation_scope_is_validated_and_independent() {
        let now = Iso8601Timestamp::parse("2026-09-27T00:00:00Z").unwrap();
        let mut p = PresentationPreference::new(
            EntityId::new("p").unwrap(),
            "skill",
            EntityId::new("s").unwrap(),
            "skill_tree",
            now.clone(),
        )
        .unwrap();
        p.is_visible = false;
        p.is_pinned = true;
        assert!(!p.is_visible);
        assert!(PresentationPreference::new(
            EntityId::new("p").unwrap(),
            "Skill",
            EntityId::new("s").unwrap(),
            "bad page",
            now
        )
        .is_err());
    }
}
