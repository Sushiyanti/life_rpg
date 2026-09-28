//! Serializable IPC mirrors for Phase 3.6 world semantics.
use lr_application::{SearchHit, SearchQuery};
use lr_domain::{
    ConceptAssociation, ContentAttachment, EffectHistoryEntry, EntityRevision, LifecycleState,
    PresentationPreference, ProgressSuggestion, QuestBranch, QuestSession, QuestStage,
    SessionEffect, Workspace, WorkspacePanel,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectHistoryDto {
    pub id: String,
    pub player_id: String,
    pub effect_id: String,
    pub session_id: Option<String>,
    pub event_kind: String,
    pub recorded_at: String,
    pub previous_state_json: Option<String>,
    pub current_state_json: String,
}
impl From<EffectHistoryEntry> for EffectHistoryDto {
    fn from(value: EffectHistoryEntry) -> Self {
        Self {
            id: value.id.to_string(),
            player_id: value.player_id.to_string(),
            effect_id: value.effect_id.to_string(),
            session_id: value.session_id.map(|id| id.to_string()),
            event_kind: value.kind.as_str().into(),
            recorded_at: value.recorded_at.to_string(),
            previous_state_json: value.previous_state_json,
            current_state_json: value.current_state_json,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEffectDto {
    pub id: String,
    pub player_id: String,
    pub session_id: String,
    pub effect_id: String,
    pub role: String,
    pub added_at: String,
    pub removed_at: Option<String>,
}
impl From<SessionEffect> for SessionEffectDto {
    fn from(value: SessionEffect) -> Self {
        Self {
            id: value.id.to_string(),
            player_id: value.player_id.to_string(),
            session_id: value.session_id.to_string(),
            effect_id: value.effect_id.to_string(),
            role: value.role.as_str().into(),
            added_at: value.added_at.to_string(),
            removed_at: value.removed_at.map(|at| at.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDto {
    pub id: String,
    pub player_id: String,
    pub name: String,
    pub template: String,
    pub sort_order: i32,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}
impl From<Workspace> for WorkspaceDto {
    fn from(v: Workspace) -> Self {
        Self {
            id: v.id.to_string(),
            player_id: v.player_id.to_string(),
            name: v.name,
            template: v.template,
            sort_order: v.sort_order,
            is_default: v.is_default,
            created_at: v.created_at.to_string(),
            updated_at: v.updated_at.to_string(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePanelDto {
    pub id: String,
    pub workspace_id: String,
    pub panel_type: String,
    pub title: Option<String>,
    pub variant: String,
    pub density: String,
    pub filter_status: Option<String>,
    pub filter_active: Option<bool>,
    pub filter_type_code: Option<String>,
    pub filter_concept_id: Option<String>,
    pub filter_tag_ids: Vec<String>,
    pub filter_tag_match: String,
    pub filter_recent_days: Option<i32>,
    pub filter_timeline_category: Option<String>,
    pub filter_timeline_entity_kind: Option<String>,
    pub filter_timeline_entity_id: Option<String>,
    pub filter_timeline_from: Option<String>,
    pub filter_timeline_through: Option<String>,
    pub sort_by: String,
    pub item_limit: i32,
    pub sort_order: i32,
    pub grid_span: i32,
    pub is_visible: bool,
    pub is_pinned: bool,
    pub is_collapsed: bool,
    pub created_at: String,
    pub updated_at: String,
}
impl From<WorkspacePanel> for WorkspacePanelDto {
    fn from(v: WorkspacePanel) -> Self {
        Self {
            id: v.id.to_string(),
            workspace_id: v.workspace_id.to_string(),
            panel_type: v.panel_type,
            title: v.title,
            variant: v.variant,
            density: v.density,
            filter_status: v.filter_status,
            filter_active: v.filter_active,
            filter_type_code: v.filter_type_code,
            filter_concept_id: v.filter_concept_id.map(|x| x.to_string()),
            filter_tag_ids: v
                .filter_tag_ids
                .into_iter()
                .map(|id| id.to_string())
                .collect(),
            filter_tag_match: v.filter_tag_match.as_str().into(),
            filter_recent_days: v.filter_recent_days,
            filter_timeline_category: v.filter_timeline_category,
            filter_timeline_entity_kind: v.filter_timeline_entity_kind,
            filter_timeline_entity_id: v.filter_timeline_entity_id,
            filter_timeline_from: v.filter_timeline_from.map(|x| x.to_string()),
            filter_timeline_through: v.filter_timeline_through.map(|x| x.to_string()),
            sort_by: v.sort_by,
            item_limit: v.item_limit,
            sort_order: v.sort_order,
            grid_span: v.grid_span,
            is_visible: v.is_visible,
            is_pinned: v.is_pinned,
            is_collapsed: v.is_collapsed,
            created_at: v.created_at.to_string(),
            updated_at: v.updated_at.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestStageDto {
    pub id: String,
    pub player_id: String,
    pub quest_id: String,
    pub title: String,
    pub description: Option<String>,
    pub story: Option<String>,
    pub instructions: Option<String>,
    pub status: String,
    pub sort_order: i32,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}
impl From<QuestStage> for QuestStageDto {
    fn from(v: QuestStage) -> Self {
        Self {
            id: v.id.to_string(),
            player_id: v.player_id.to_string(),
            quest_id: v.quest_id.to_string(),
            title: v.title,
            description: v.description,
            story: v.story,
            instructions: v.instructions,
            status: v.status.as_str().into(),
            sort_order: v.sort_order,
            is_active: v.is_active,
            metadata_json: v.metadata_json,
            created_at: v.created_at.to_string(),
            updated_at: v.updated_at.to_string(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestBranchDto {
    pub id: String,
    pub player_id: String,
    pub quest_id: String,
    pub stage_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub sort_order: i32,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}
impl From<QuestBranch> for QuestBranchDto {
    fn from(v: QuestBranch) -> Self {
        Self {
            id: v.id.to_string(),
            player_id: v.player_id.to_string(),
            quest_id: v.quest_id.to_string(),
            stage_id: v.stage_id.to_string(),
            title: v.title,
            description: v.description,
            status: v.status.as_str().into(),
            sort_order: v.sort_order,
            is_active: v.is_active,
            metadata_json: v.metadata_json,
            created_at: v.created_at.to_string(),
            updated_at: v.updated_at.to_string(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestSessionDto {
    pub id: String,
    pub player_id: String,
    pub quest_id: Option<String>,
    pub stage_id: Option<String>,
    pub branch_id: Option<String>,
    pub skill_id: Option<String>,
    pub concept_id: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub status: String,
    pub progress_before: Option<i32>,
    pub progress_after: Option<i32>,
    pub result: Option<String>,
    pub notes: Option<String>,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}
impl From<QuestSession> for QuestSessionDto {
    fn from(v: QuestSession) -> Self {
        Self {
            id: v.id.to_string(),
            player_id: v.player_id.to_string(),
            quest_id: v.quest_id.map(|x| x.to_string()),
            stage_id: v.stage_id.map(|x| x.to_string()),
            branch_id: v.branch_id.map(|x| x.to_string()),
            skill_id: v.skill_id.map(|x| x.to_string()),
            concept_id: v.concept_id.map(|x| x.to_string()),
            started_at: v.started_at.to_string(),
            ended_at: v.ended_at.map(|x| x.to_string()),
            status: v.status.as_str().into(),
            progress_before: v.progress_before,
            progress_after: v.progress_after,
            result: v.result,
            notes: v.notes,
            is_active: v.is_active,
            metadata_json: v.metadata_json,
            created_at: v.created_at.to_string(),
            updated_at: v.updated_at.to_string(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentAttachmentDto {
    pub id: String,
    pub content_id: String,
    pub player_id: String,
    pub target_kind: String,
    pub target_id: String,
    pub role_code: String,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub removed_at: Option<String>,
}
impl From<ContentAttachment> for ContentAttachmentDto {
    fn from(v: ContentAttachment) -> Self {
        Self {
            id: v.id.to_string(),
            content_id: v.content_id.to_string(),
            player_id: v.player_id.to_string(),
            target_kind: v.target_kind.as_str().into(),
            target_id: v.target_id.to_string(),
            role_code: v.role_code,
            sort_order: v.sort_order,
            is_active: v.is_active,
            created_at: v.created_at.to_string(),
            updated_at: v.updated_at.to_string(),
            removed_at: v.removed_at.map(|value| value.to_string()),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptAssociationDto {
    pub id: String,
    pub player_id: String,
    pub concept_id: String,
    pub entity_kind: String,
    pub entity_id: String,
    pub association_code: String,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}
impl From<ConceptAssociation> for ConceptAssociationDto {
    fn from(v: ConceptAssociation) -> Self {
        Self {
            id: v.id.to_string(),
            player_id: v.player_id.to_string(),
            concept_id: v.concept_id.to_string(),
            entity_kind: v.entity_kind.as_str().into(),
            entity_id: v.entity_id,
            association_code: v.association_code,
            is_active: v.is_active,
            metadata_json: v.metadata_json,
            created_at: v.created_at.to_string(),
            updated_at: v.updated_at.to_string(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityRevisionDto {
    pub id: String,
    pub player_id: String,
    pub target_kind: String,
    pub target_id: String,
    pub revision_number: u32,
    pub recorded_at: String,
    pub author_player_id: Option<String>,
    pub reason: Option<String>,
    pub snapshot_json: String,
    pub metadata_json: String,
}
impl From<EntityRevision> for EntityRevisionDto {
    fn from(v: EntityRevision) -> Self {
        Self {
            id: v.id.to_string(),
            player_id: v.player_id.to_string(),
            target_kind: v.target_kind.as_str().into(),
            target_id: v.target_id.to_string(),
            revision_number: v.revision_number,
            recorded_at: v.recorded_at.to_string(),
            author_player_id: v.author_player_id.map(|x| x.to_string()),
            reason: v.reason,
            snapshot_json: v.snapshot_json,
            metadata_json: v.metadata_json,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationPreferenceDto {
    pub player_id: String,
    pub entity_kind: String,
    pub entity_id: String,
    pub context: String,
    pub is_visible: bool,
    pub sort_order: i32,
    pub is_pinned: bool,
    pub is_collapsed: Option<bool>,
    pub variant: Option<String>,
    pub density: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}
impl From<PresentationPreference> for PresentationPreferenceDto {
    fn from(v: PresentationPreference) -> Self {
        Self {
            player_id: v.player_id.to_string(),
            entity_kind: v.entity_kind,
            entity_id: v.entity_id.to_string(),
            context: v.context,
            is_visible: v.is_visible,
            sort_order: v.sort_order,
            is_pinned: v.is_pinned,
            is_collapsed: v.is_collapsed,
            variant: v.variant,
            density: v.density,
            metadata_json: v.metadata_json,
            created_at: v.created_at.to_string(),
            updated_at: v.updated_at.to_string(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressSuggestionDto {
    pub id: String,
    pub player_id: String,
    pub concept_id: String,
    pub track_code: String,
    pub proposed_value: f64,
    pub proposed_level: Option<i32>,
    pub reason: Option<String>,
    pub source: String,
    pub status: String,
    pub created_at: String,
    pub resolved_at: Option<String>,
    pub metadata_json: String,
}
impl From<ProgressSuggestion> for ProgressSuggestionDto {
    fn from(v: ProgressSuggestion) -> Self {
        Self {
            id: v.id.to_string(),
            player_id: v.player_id.to_string(),
            concept_id: v.concept_id.to_string(),
            track_code: v.track_code,
            proposed_value: v.proposed_value,
            proposed_level: v.proposed_level,
            reason: v.reason,
            source: v.source,
            status: v.status.as_str().into(),
            created_at: v.created_at.to_string(),
            resolved_at: v.resolved_at.map(|x| x.to_string()),
            metadata_json: v.metadata_json,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptProgressTrackDto {
    pub id: String,
    pub concept_id: String,
    pub track_code: String,
    pub current_value: f64,
    pub level: Option<i32>,
    pub level_name: Option<String>,
    pub progression_label: Option<String>,
    pub control: String,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}
impl From<lr_domain::ConceptProgressTrack> for ConceptProgressTrackDto {
    fn from(v: lr_domain::ConceptProgressTrack) -> Self {
        Self {
            id: v.id.to_string(),
            concept_id: v.concept_id.to_string(),
            track_code: v.track_code,
            current_value: v.current_value,
            level: v.level,
            level_name: v.level_name,
            progression_label: v.progression_label,
            control: v.control.as_str().into(),
            is_active: v.is_active,
            metadata_json: v.metadata_json,
            created_at: v.created_at.to_string(),
            updated_at: v.updated_at.to_string(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleStateDto {
    pub state: String,
}
impl From<LifecycleState> for LifecycleStateDto {
    fn from(v: LifecycleState) -> Self {
        Self {
            state: v.as_str().into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQueryDto {
    pub text: Option<String>,
    pub kind: Option<String>,
    pub player_id: Option<String>,
    pub concept_id: Option<String>,
    pub tag_ids: Vec<String>,
    pub tag_match: String,
    pub type_code: Option<String>,
    pub status: Option<String>,
    pub active: Option<bool>,
    pub target_kind: Option<String>,
    pub from: Option<String>,
    pub through: Option<String>,
    pub context: Option<String>,
    pub include_hidden: bool,
    pub include_archived: bool,
    pub include_trashed: bool,
    pub sort: String,
    pub limit: u32,
    pub offset: u32,
}
impl From<SearchQuery> for SearchQueryDto {
    fn from(v: SearchQuery) -> Self {
        Self {
            text: v.text,
            kind: v.kind.map(|x| x.as_str().into()),
            player_id: v.player_id.map(|x| x.to_string()),
            concept_id: v.concept_id.map(|x| x.to_string()),
            tag_ids: v.tag_ids.into_iter().map(|id| id.to_string()).collect(),
            tag_match: v.tag_match.as_str().into(),
            type_code: v.type_code,
            status: v.status,
            active: v.active,
            target_kind: v.target_kind.map(|value| value.as_str().into()),
            from: v.from.map(|x| x.to_string()),
            through: v.through.map(|x| x.to_string()),
            context: v.context,
            include_hidden: v.include_hidden,
            include_archived: v.include_archived,
            include_trashed: v.include_trashed,
            sort: match v.sort {
                lr_application::SearchSort::Relevance => "relevance",
                lr_application::SearchSort::Newest => "newest",
                lr_application::SearchSort::Oldest => "oldest",
                lr_application::SearchSort::Name => "name",
                lr_application::SearchSort::Progression => "progression",
            }
            .into(),
            limit: v.limit,
            offset: v.offset,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHitDto {
    pub kind: String,
    pub id: String,
    pub player_id: Option<String>,
    pub concept_id: Option<String>,
    pub type_code: Option<String>,
    pub status: Option<String>,
    pub active: Option<bool>,
    pub lifecycle: String,
    pub visible: Option<bool>,
    pub occurred_at: Option<String>,
    pub captured_at: Option<String>,
    pub name: String,
    pub snippet: String,
    pub progression: Option<f64>,
    pub relevance: Option<f64>,
}
impl From<SearchHit> for SearchHitDto {
    fn from(v: SearchHit) -> Self {
        Self {
            kind: v.kind.as_str().into(),
            id: v.id,
            player_id: v.player_id.map(|x| x.to_string()),
            concept_id: v.concept_id.map(|x| x.to_string()),
            type_code: v.type_code,
            status: v.status,
            active: v.active,
            lifecycle: v.lifecycle.as_str().into(),
            visible: v.visible,
            occurred_at: v.occurred_at.map(|x| x.to_string()),
            captured_at: v.captured_at.map(|x| x.to_string()),
            name: v.name,
            snippet: v.snippet,
            progression: v.progression,
            relevance: v.relevance,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDto {
    pub id: String,
    pub player_id: String,
    pub transfer_key: String,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    pub lifecycle: String,
    pub usage_count: u32,
    pub created_at: String,
    pub updated_at: String,
}
impl From<lr_domain::Tag> for TagDto {
    fn from(v: lr_domain::Tag) -> Self {
        Self {
            id: v.id.to_string(),
            player_id: v.player_id.to_string(),
            transfer_key: v.transfer_key,
            name: v.name,
            normalized_name: v.normalized_name,
            description: v.description,
            lifecycle: v.lifecycle.as_str().into(),
            usage_count: v.usage_count,
            created_at: v.created_at.to_string(),
            updated_at: v.updated_at.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagRelationshipDto {
    pub id: String,
    pub player_id: String,
    pub tag_id: String,
    pub target_kind: String,
    pub target_id: String,
    pub added_at: String,
    pub removed_at: Option<String>,
}
impl From<lr_domain::TagRelationship> for TagRelationshipDto {
    fn from(v: lr_domain::TagRelationship) -> Self {
        Self {
            id: v.id.to_string(),
            player_id: v.player_id.to_string(),
            tag_id: v.tag_id.to_string(),
            target_kind: v.target_kind.as_str().into(),
            target_id: v.target_id,
            added_at: v.added_at.to_string(),
            removed_at: v.removed_at.map(|at| at.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggedRecordDto {
    pub relationship: TagRelationshipDto,
    pub tag: TagDto,
}
impl From<lr_domain::TaggedRecord> for TaggedRecordDto {
    fn from(v: lr_domain::TaggedRecord) -> Self {
        Self {
            relationship: v.relationship.into(),
            tag: v.tag.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagTargetReferenceDto {
    pub relationship: TagRelationshipDto,
    pub target_name: String,
    pub target_lifecycle: Option<String>,
}
impl From<lr_domain::TagTargetReference> for TagTargetReferenceDto {
    fn from(v: lr_domain::TagTargetReference) -> Self {
        Self {
            relationship: v.relationship.into(),
            target_name: v.target_name,
            target_lifecycle: v.target_lifecycle.map(|state| state.as_str().into()),
        }
    }
}

/// One declarative panel sent only after portable Concept references are resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceImportPanelDto {
    pub panel_type: String,
    pub title: Option<String>,
    pub variant: String,
    pub density: String,
    pub filter_status: Option<String>,
    pub filter_active: Option<bool>,
    pub filter_type_code: Option<String>,
    pub filter_concept_id: Option<String>,
    /// Destination-local Tag IDs after explicit transfer-reference resolution.
    pub filter_tag_ids: Vec<String>,
    pub filter_tag_match: String,
    pub filter_recent_days: Option<i32>,
    pub filter_timeline_category: Option<String>,
    pub filter_timeline_entity_kind: Option<String>,
    pub filter_timeline_entity_id: Option<String>,
    pub filter_timeline_from: Option<String>,
    pub filter_timeline_through: Option<String>,
    pub sort_by: String,
    pub item_limit: i32,
    pub sort_order: i32,
    pub grid_span: i32,
    pub is_visible: bool,
    pub is_pinned: bool,
    pub is_collapsed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceImportRequestDto {
    pub name: String,
    pub template: String,
    pub panels: Vec<WorkspaceImportPanelDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceImportResultDto {
    pub workspace: WorkspaceDto,
    pub panels: Vec<WorkspacePanelDto>,
}
