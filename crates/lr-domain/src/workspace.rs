//! User-configurable workspace presentation, intentionally separate from world entities.
use crate::{DateValue, DomainError, DomainResult, EntityId, Iso8601Timestamp};

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

/// One reusable presentation instance. Every option is data from a closed,
/// validated vocabulary; no field contains a predicate, SQL fragment or code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePanel {
    pub id: EntityId,
    pub workspace_id: EntityId,
    pub panel_type: String,
    pub title: Option<String>,
    pub variant: String,
    pub density: String,
    pub filter_status: Option<String>,
    pub filter_active: Option<bool>,
    pub filter_type_code: Option<String>,
    pub filter_concept_id: Option<EntityId>,
    pub filter_recent_days: Option<i32>,
    pub filter_timeline_category: Option<String>,
    pub filter_timeline_entity_kind: Option<String>,
    pub filter_timeline_entity_id: Option<String>,
    pub filter_timeline_from: Option<DateValue>,
    pub filter_timeline_through: Option<DateValue>,
    pub sort_by: String,
    pub item_limit: i32,
    pub sort_order: i32,
    pub grid_span: i32,
    pub is_visible: bool,
    pub is_pinned: bool,
    pub is_collapsed: bool,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl WorkspacePanel {
    pub fn validate(&self) -> DomainResult<()> {
        let variants: &[&str] = match self.panel_type.as_str() {
            "quests" => &["cards", "rows", "compact", "detailed"],
            "skills" => &["cards", "rows", "compact", "tree"],
            "concepts" => &["cards", "rows", "compact"],
            "effects" => &["rows", "compact", "detailed"],
            "activity" | "transactions" => &["rows", "compact", "detailed", "timeline"],
            "journal" => &["cards", "rows", "compact", "detailed"],
            "player" => &["metrics", "cards", "compact", "detailed"],
            "progress" => &["metrics", "rows", "cards"],
            "timeline" => &["rows", "compact", "timeline"],
            _ => {
                return Err(DomainError::invalid_value(
                    "panel type",
                    "unsupported panel type",
                ))
            }
        };
        if !variants.contains(&self.variant.as_str()) {
            return Err(DomainError::invalid_value(
                "panel variant",
                "unsupported for this panel source",
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
        if !(1..=2).contains(&self.grid_span) {
            return Err(DomainError::invalid_value(
                "panel grid span",
                "must be between 1 and 2",
            ));
        }
        if !(0..=999).contains(&self.sort_order) {
            return Err(DomainError::invalid_value(
                "panel sort order",
                "must be between 0 and 999",
            ));
        }
        let statuses: &[&str] = match self.panel_type.as_str() {
            "quests" => &["open", "active", "completed", "abandoned"],
            "skills" => &["active", "paused", "completed", "archived"],
            "concepts" | "progress" => &["active", "archived"],
            "effects" => &["active", "inactive"],
            "activity" => &["in_progress", "completed", "interrupted"],
            _ => &[],
        };
        if self
            .filter_status
            .as_deref()
            .is_some_and(|v| !statuses.contains(&v))
        {
            return Err(DomainError::invalid_value(
                "panel status filter",
                "unsupported for this panel source",
            ));
        }
        if self.filter_type_code.as_deref().is_some_and(|v| {
            v.is_empty()
                || v.len() > 160
                || !v
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        }) {
            return Err(DomainError::invalid_value(
                "panel type-code filter",
                "use 1-160 lowercase ASCII letters, digits, or underscores",
            ));
        }
        if self
            .filter_recent_days
            .is_some_and(|d| !(1..=365).contains(&d))
        {
            return Err(DomainError::invalid_value(
                "panel recent-days filter",
                "must be between 1 and 365",
            ));
        }
        let supports_active = matches!(
            self.panel_type.as_str(),
            "skills" | "concepts" | "progress" | "effects"
        );
        let supports_type = matches!(
            self.panel_type.as_str(),
            "quests" | "skills" | "concepts" | "progress" | "effects" | "transactions" | "journal"
        );
        if self.filter_active.is_some() && !supports_active {
            return Err(DomainError::invalid_value(
                "panel active filter",
                "unsupported for this panel source",
            ));
        }
        if self.filter_type_code.is_some() && !supports_type {
            return Err(DomainError::invalid_value(
                "panel type-code filter",
                "unsupported for this panel source",
            ));
        }
        if (self.filter_concept_id.is_some() || self.filter_recent_days.is_some())
            && self.panel_type == "player"
        {
            return Err(DomainError::invalid_value(
                "panel filter",
                "Concept and recent-day filters are unsupported for Player status",
            ));
        }
        const TIMELINE_CATEGORIES: &[&str] = &[
            "session",
            "transaction",
            "effect_history",
            "content",
            "comment",
            "concept_progress",
            "revision",
            "snapshot",
            "record_change",
            "lifecycle",
            "relationship_history",
        ];
        const TIMELINE_ENTITY_KINDS: &[&str] = &[
            "player",
            "concept",
            "quest",
            "quest_stage",
            "quest_branch",
            "quest_session",
            "skill_tree",
            "skill",
            "effect",
            "transaction",
            "comment",
            "narrative_entry",
            "concept_progress",
        ];
        if self
            .filter_timeline_category
            .as_deref()
            .is_some_and(|value| !TIMELINE_CATEGORIES.contains(&value))
        {
            return Err(DomainError::invalid_value(
                "Timeline category filter",
                "unsupported recorded category",
            ));
        }
        if self
            .filter_timeline_entity_kind
            .as_deref()
            .is_some_and(|value| !TIMELINE_ENTITY_KINDS.contains(&value))
        {
            return Err(DomainError::invalid_value(
                "Timeline entity filter",
                "unsupported world record kind",
            ));
        }
        if self
            .filter_timeline_entity_id
            .as_ref()
            .is_some_and(|id| id.is_empty() || id.len() > 160)
        {
            return Err(DomainError::invalid_value(
                "Timeline entity identity filter",
                "must contain 1-160 characters",
            ));
        }
        if self.filter_timeline_entity_id.is_some() && self.filter_timeline_entity_kind.is_none() {
            return Err(DomainError::invalid_value(
                "Timeline entity identity filter",
                "requires an entity kind",
            ));
        }
        let has_timeline_filter = self.filter_timeline_category.is_some()
            || self.filter_timeline_entity_kind.is_some()
            || self.filter_timeline_entity_id.is_some()
            || self.filter_timeline_from.is_some()
            || self.filter_timeline_through.is_some();
        if has_timeline_filter && self.panel_type != "timeline" {
            return Err(DomainError::invalid_value(
                "Timeline panel filter",
                "supported only by Timeline panels",
            ));
        }
        if self.panel_type == "timeline"
            && (self.filter_status.is_some()
                || self.filter_active.is_some()
                || self.filter_type_code.is_some()
                || self.filter_recent_days.is_some())
        {
            return Err(DomainError::invalid_value(
                "Timeline panel filters",
                "use the typed Timeline category, entity, Concept, and date filters",
            ));
        }
        if self
            .filter_timeline_from
            .as_ref()
            .zip(self.filter_timeline_through.as_ref())
            .is_some_and(|(from, through)| from > through)
        {
            return Err(DomainError::invalid_value(
                "Timeline date range",
                "start must not be after end",
            ));
        }
        let sorts: &[&str] = match self.panel_type.as_str() {
            "quests" => &[
                "updated_desc",
                "created_desc",
                "name_asc",
                "status_asc",
                "progress_desc",
            ],
            "skills" => &[
                "name_asc",
                "updated_desc",
                "created_desc",
                "status_asc",
                "level_desc",
            ],
            "concepts" => &[
                "name_asc",
                "updated_desc",
                "created_desc",
                "status_asc",
                "progress_desc",
            ],
            "effects" => &["name_asc", "updated_desc", "created_desc", "status_asc"],
            "journal" => &["created_desc", "updated_desc", "name_asc"],
            "player" => &["name_asc", "updated_desc"],
            "progress" => &["name_asc", "updated_desc", "progress_desc", "level_desc"],
            "activity" => &["started_desc", "created_desc", "status_asc"],
            "transactions" => &["occurred_desc", "created_desc"],
            "timeline" => &["timeline_newest", "timeline_oldest"],
            _ => &[],
        };
        if !sorts.contains(&self.sort_by.as_str()) {
            return Err(DomainError::invalid_value(
                "panel sort",
                "unsupported for this panel source",
            ));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> WorkspacePanel {
        let now = Iso8601Timestamp::parse("2026-09-27T10:00:00Z").unwrap();
        WorkspacePanel {
            id: EntityId::new("panel-1").unwrap(),
            workspace_id: EntityId::new("workspace-1").unwrap(),
            panel_type: "quests".into(),
            title: Some("Open goals".into()),
            variant: "cards".into(),
            density: "cozy".into(),
            filter_status: Some("active".into()),
            filter_active: None,
            filter_type_code: Some("main".into()),
            filter_concept_id: Some(EntityId::new("concept-1").unwrap()),
            filter_recent_days: Some(30),
            filter_timeline_category: None,
            filter_timeline_entity_kind: None,
            filter_timeline_entity_id: None,
            filter_timeline_from: None,
            filter_timeline_through: None,
            sort_by: "updated_desc".into(),
            item_limit: 8,
            sort_order: 0,
            grid_span: 1,
            is_visible: true,
            is_pinned: true,
            is_collapsed: false,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    #[test]
    fn accepts_bounded_filters_and_source_specific_variants() {
        assert!(panel().validate().is_ok());
        let mut value = panel();
        value.variant = "tree".into();
        value.panel_type = "skills".into();
        value.filter_status = Some("active".into());
        value.sort_by = "level_desc".into();
        assert!(value.validate().is_ok());
    }

    #[test]
    fn rejects_executable_or_unknown_filters_and_unsupported_variants() {
        let mut value = panel();
        value.filter_status = Some("active' OR 1=1 --".into());
        assert!(value.validate().is_err());
        value = panel();
        value.filter_type_code = Some("main OR 1=1".into());
        assert!(value.validate().is_err());
        value = panel();
        value.variant = "tree".into();
        assert!(value.validate().is_err());
        value = panel();
        value.grid_span = 3;
        assert!(value.validate().is_err());
        value = panel();
        value.sort_order = 1000;
        assert!(value.validate().is_err());
        value = panel();
        value.filter_recent_days = Some(366);
        assert!(value.validate().is_err());
    }

    #[test]
    fn rejects_filters_and_sorts_not_registered_for_a_source() {
        let mut value = panel();
        value.panel_type = "player".into();
        value.variant = "metrics".into();
        value.filter_status = None;
        value.filter_type_code = None;
        value.filter_concept_id = None;
        value.filter_recent_days = None;
        value.sort_by = "name_asc".into();
        assert!(value.validate().is_ok());

        value.filter_active = Some(true);
        assert!(value.validate().is_err());
        value.filter_active = None;
        value.filter_recent_days = Some(7);
        assert!(value.validate().is_err());
        value.filter_recent_days = None;
        value.filter_type_code = Some("main".into());
        assert!(value.validate().is_err());

        value.filter_type_code = None;
        value.panel_type = "quests".into();
        value.variant = "cards".into();
        value.sort_by = "not_a_sort".into();
        assert!(value.validate().is_err());
        value = panel();
        value.sort_order = 1000;
        assert!(value.validate().is_err());
    }

    #[test]
    fn timeline_panel_accepts_only_closed_filters_and_ordered_calendar_dates() {
        let mut value = panel();
        value.panel_type = "timeline".into();
        value.variant = "timeline".into();
        value.filter_status = None;
        value.filter_type_code = None;
        value.filter_concept_id = None;
        value.filter_recent_days = None;
        value.filter_timeline_category = Some("session".into());
        value.filter_timeline_entity_kind = Some("quest_session".into());
        value.filter_timeline_entity_id = Some("session-1".into());
        value.filter_timeline_from = Some(DateValue::parse("2026-09-01").unwrap());
        value.filter_timeline_through = Some(DateValue::parse("2026-09-30").unwrap());
        value.sort_by = "timeline_oldest".into();
        assert!(value.validate().is_ok());

        value.filter_timeline_entity_kind = None;
        assert!(value.validate().is_err());
        value.filter_timeline_entity_kind = Some("quest_session".into());

        value.filter_timeline_category = Some("session OR 1=1".into());
        assert!(value.validate().is_err());
        value.filter_timeline_category = None;
        value.filter_timeline_entity_kind = Some("quest; DROP TABLE quests".into());
        assert!(value.validate().is_err());
        value.filter_timeline_entity_kind = None;
        value.filter_timeline_from = Some(DateValue::parse("2026-10-01").unwrap());
        assert!(value.validate().is_err());
        value.filter_timeline_from = None;
        value.filter_recent_days = Some(7);
        assert!(value.validate().is_err());
    }
}
