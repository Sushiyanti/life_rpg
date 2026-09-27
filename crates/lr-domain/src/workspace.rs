//! User-configurable workspace presentation, intentionally separate from world entities.
use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: EntityId,
    pub player_id: EntityId,
    pub name: String,
    pub template: String,
    pub sort_order: i32,
    pub is_default: bool,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl Workspace {
    pub fn new(
        id: EntityId,
        player_id: EntityId,
        name: &str,
        template: &str,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 {
            return Err(DomainError::invalid_value(
                "workspace name",
                "must contain 1-80 characters",
            ));
        }
        if !matches!(
            template,
            "overview" | "focus" | "learning" | "health" | "review" | "custom"
        ) {
            return Err(DomainError::invalid_value(
                "workspace template",
                "unsupported template",
            ));
        }
        Ok(Self {
            id,
            player_id,
            name: name.to_string(),
            template: template.to_string(),
            sort_order: 0,
            is_default: false,
            created_at: now.clone(),
            updated_at: now,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePanel {
    pub id: EntityId,
    pub workspace_id: EntityId,
    pub panel_type: String,
    pub title: Option<String>,
    pub variant: String,
    pub density: String,
    pub filter_status: Option<String>,
    pub item_limit: i32,
    pub sort_order: i32,
    pub is_pinned: bool,
    pub is_collapsed: bool,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl WorkspacePanel {
    pub fn validate(&self) -> DomainResult<()> {
        if !matches!(
            self.panel_type.as_str(),
            "quests" | "skills" | "concepts" | "effects" | "activity" | "transactions" | "journal"
        ) {
            return Err(DomainError::invalid_value(
                "panel type",
                "unsupported panel type",
            ));
        }
        if !matches!(self.variant.as_str(), "cards" | "rows") {
            return Err(DomainError::invalid_value(
                "panel variant",
                "must be cards or rows",
            ));
        }
        if !matches!(self.density.as_str(), "cozy" | "compact") {
            return Err(DomainError::invalid_value(
                "panel density",
                "must be cozy or compact",
            ));
        }
        if !(1..=50).contains(&self.item_limit) {
            return Err(DomainError::invalid_value(
                "panel item limit",
                "must be between 1 and 50",
            ));
        }
        if let Some(status) = self.filter_status.as_deref() {
            if !matches!(
                status,
                "active" | "in_progress" | "pending" | "completed" | "archived"
            ) {
                return Err(DomainError::invalid_value(
                    "panel status filter",
                    "unsupported status",
                ));
            }
        }
        if self
            .title
            .as_deref()
            .is_some_and(|v| v.trim().is_empty() || v.chars().count() > 120)
        {
            return Err(DomainError::invalid_value(
                "panel title",
                "must contain 1-120 characters",
            ));
        }
        Ok(())
    }
}
