//! Player-owned organizational labels and their explicit, closed-target assignments.
use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp, LifecycleState};
use std::collections::HashSet;

/// Supported current world records that may be explicitly organized with Tags.
/// Immutable records and Workspace configuration are intentionally excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TagTargetKind {
    Quest,
    QuestStage,
    QuestBranch,
    QuestSession,
    SkillTree,
    Skill,
    Concept,
    Effect,
    NarrativeEntry,
    Comment,
}
impl TagTargetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Quest => "quest",
            Self::QuestStage => "quest_stage",
            Self::QuestBranch => "quest_branch",
            Self::QuestSession => "quest_session",
            Self::SkillTree => "skill_tree",
            Self::Skill => "skill",
            Self::Concept => "concept",
            Self::Effect => "effect",
            Self::NarrativeEntry => "narrative_entry",
            Self::Comment => "comment",
        }
    }
    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "quest" => Ok(Self::Quest),
            "quest_stage" => Ok(Self::QuestStage),
            "quest_branch" => Ok(Self::QuestBranch),
            "quest_session" => Ok(Self::QuestSession),
            "skill_tree" => Ok(Self::SkillTree),
            "skill" => Ok(Self::Skill),
            "concept" => Ok(Self::Concept),
            "effect" => Ok(Self::Effect),
            "narrative_entry" => Ok(Self::NarrativeEntry),
            "comment" => Ok(Self::Comment),
            _ => Err(DomainError::invalid_value(
                "Tag target kind",
                "unsupported world record kind",
            )),
        }
    }
}

/// A deliberately small matching choice; Tags do not expose a boolean query language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TagMatchMode {
    #[default]
    Any,
    All,
}
impl TagMatchMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::All => "all",
        }
    }
    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "any" => Ok(Self::Any),
            "all" => Ok(Self::All),
            _ => Err(DomainError::invalid_value(
                "Tag match mode",
                "must be any or all",
            )),
        }
    }
}

/// Comparison canonicalization used only to prevent indistinguishable names.
/// Original casing is retained in `name`; whitespace runs collapse and Unicode
/// lowercase is applied without implying any semantic aliasing.
pub fn normalize_tag_name(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub id: EntityId,
    pub player_id: EntityId,
    /// Stable opaque transfer reference, distinct from this installation's row ID.
    pub transfer_key: String,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    /// Read from the shared recoverable lifecycle ledger; not duplicated in `tags`.
    pub lifecycle: LifecycleState,
    /// Derived from active assignment facts at query time; never cached.
    pub usage_count: u32,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl Tag {
    pub fn new(
        id: EntityId,
        player_id: EntityId,
        transfer_key: impl Into<String>,
        name: &str,
        description: Option<String>,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 {
            return Err(DomainError::invalid_value(
                "Tag name",
                "must contain 1-80 characters",
            ));
        }
        let transfer_key = transfer_key.into();
        if transfer_key.trim().is_empty() || transfer_key.chars().count() > 128 {
            return Err(DomainError::invalid_value(
                "Tag transfer key",
                "must contain 1-128 characters",
            ));
        }
        if description
            .as_ref()
            .is_some_and(|value| value.chars().count() > 4000)
        {
            return Err(DomainError::invalid_value(
                "Tag description",
                "must be at most 4000 characters",
            ));
        }
        Ok(Self {
            id,
            player_id,
            transfer_key,
            name: name.to_owned(),
            normalized_name: normalize_tag_name(name),
            description: description
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty()),
            lifecycle: LifecycleState::Active,
            usage_count: 0,
            created_at: now.clone(),
            updated_at: now,
        })
    }
    /// Rename in place. Identity, transfer reference, lifecycle, and assignments remain unchanged.
    pub fn rename(
        &mut self,
        name: &str,
        description: Option<String>,
        now: Iso8601Timestamp,
    ) -> DomainResult<()> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 {
            return Err(DomainError::invalid_value(
                "Tag name",
                "must contain 1-80 characters",
            ));
        }
        if description
            .as_ref()
            .is_some_and(|value| value.chars().count() > 4000)
        {
            return Err(DomainError::invalid_value(
                "Tag description",
                "must be at most 4000 characters",
            ));
        }
        self.name = name.to_owned();
        self.normalized_name = normalize_tag_name(name);
        self.description = description
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        self.updated_at = now;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagRelationship {
    pub id: EntityId,
    pub player_id: EntityId,
    pub tag_id: EntityId,
    pub target_kind: TagTargetKind,
    /// String identity supports both opaque world IDs and integer Comment IDs.
    pub target_id: String,
    pub added_at: Iso8601Timestamp,
    pub removed_at: Option<Iso8601Timestamp>,
}
impl TagRelationship {
    pub fn new(
        id: EntityId,
        player_id: EntityId,
        tag_id: EntityId,
        target_kind: TagTargetKind,
        target_id: &str,
        added_at: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        EntityId::new(target_id.to_owned())?;
        Ok(Self {
            id,
            player_id,
            tag_id,
            target_kind,
            target_id: target_id.to_owned(),
            added_at,
            removed_at: None,
        })
    }
}

/// Tag assignment returned together with the current canonical Tag record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedRecord {
    pub relationship: TagRelationship,
    pub tag: Tag,
}

/// One assignment-history row for the Tag Manager, joined to its canonical target name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagTargetReference {
    pub relationship: TagRelationship,
    pub target_name: String,
    pub target_lifecycle: Option<LifecycleState>,
}

/// Shared bounded validation for Search and declarative Workspace filters.
pub fn validate_tag_filter(ids: &[EntityId], mode: TagMatchMode) -> DomainResult<()> {
    let _ = mode;
    if ids.len() > 20 {
        return Err(DomainError::invalid_value(
            "Tag filter",
            "at most 20 Tags may be selected",
        ));
    }
    let unique: HashSet<&EntityId> = ids.iter().collect();
    if unique.len() != ids.len() {
        return Err(DomainError::invalid_value(
            "Tag filter",
            "duplicate Tag IDs are not allowed",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time() -> Iso8601Timestamp {
        Iso8601Timestamp::parse("2026-09-28T00:00:00Z").unwrap()
    }

    #[test]
    fn tag_names_normalize_predictably_and_identity_survives_rename() {
        let mut tag = Tag::new(
            EntityId::new("tag-local").unwrap(),
            EntityId::new("player-a").unwrap(),
            "tag-ref-opaque",
            "  Learning  ",
            Some("  study  ".into()),
            time(),
        )
        .unwrap();
        assert_eq!(tag.normalized_name, "learning");
        let id = tag.id.clone();
        let transfer = tag.transfer_key.clone();
        tag.rename("LEARNING", None, time()).unwrap();
        assert_eq!(tag.normalized_name, "learning");
        assert_eq!(tag.id, id);
        assert_eq!(tag.transfer_key, transfer);
        assert_eq!(tag.description, None);
    }

    #[test]
    fn target_kinds_and_tag_modes_fail_closed() {
        assert_eq!(
            TagTargetKind::parse("quest_session").unwrap().as_str(),
            "quest_session"
        );
        assert!(TagTargetKind::parse("transaction").is_err());
        assert_eq!(TagMatchMode::parse("all").unwrap(), TagMatchMode::All);
        assert!(TagMatchMode::parse("sql").is_err());
        assert!(Tag::new(
            EntityId::new("t").unwrap(),
            EntityId::new("p").unwrap(),
            "ref",
            "   ",
            None,
            time()
        )
        .is_err());
    }
}
