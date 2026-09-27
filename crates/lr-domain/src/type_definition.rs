//! Database-driven type vocabulary shared by quests, skills, effects, and history.

use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp};

fn non_blank(field: &'static str, value: impl Into<String>) -> DomainResult<String> {
    let value = value.into();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DomainError::invalid_value(field, "must not be blank"));
    }
    if trimmed.chars().count() > 160 {
        return Err(DomainError::invalid_value(field, "is too long"));
    }
    Ok(trimmed.to_string())
}

/// A stable reference into the data-defined type registry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeRef {
    pub namespace: String,
    pub code: String,
}
impl TypeRef {
    pub fn new(namespace: impl Into<String>, code: impl Into<String>) -> DomainResult<Self> {
        Ok(Self {
            namespace: non_blank("type namespace", namespace)?,
            code: non_blank("type code", code)?,
        })
    }
    pub fn quest(code: impl Into<String>) -> DomainResult<Self> {
        Self::new("quest", code)
    }
    pub fn skill_tree(code: impl Into<String>) -> DomainResult<Self> {
        Self::new("skill_tree", code)
    }
    pub fn skill(code: impl Into<String>) -> DomainResult<Self> {
        Self::new("skill", code)
    }
    pub fn effect(code: impl Into<String>) -> DomainResult<Self> {
        Self::new("effect", code)
    }
    pub fn transaction(code: impl Into<String>) -> DomainResult<Self> {
        Self::new("transaction", code)
    }
}

/// One row from the extensible type registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDefinition {
    pub id: EntityId,
    pub type_ref: TypeRef,
    pub label: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub is_system: bool,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl TypeDefinition {
    pub fn new(
        id: EntityId,
        type_ref: TypeRef,
        label: impl Into<String>,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        Ok(Self {
            id,
            type_ref,
            label: non_blank("type label", label)?,
            description: None,
            sort_order: 0,
            is_active: true,
            is_system: false,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
    pub fn deactivate(&mut self, now: Iso8601Timestamp) {
        self.is_active = false;
        self.updated_at = now;
    }
}
