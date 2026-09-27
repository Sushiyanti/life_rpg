//! Focused persistence contract for Phase 3.6 world semantics.
use crate::error::StorageError;
use lr_domain::{
    AssociatedEntityKind, ConceptAssociation, ContentAttachment, ContentTargetKind, Effect,
    EffectHistoryEntry, EntityId, EntityRevision, Iso8601Timestamp, LifecycleState,
    PresentationPreference, ProgressSuggestion, QuestBranch, QuestSession, QuestStage,
    RevisionTargetKind, SessionEffect, Workspace, WorkspacePanel,
};

/// A validated declarative panel requested by a workspace transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePanelImport {
    pub panel_type: String,
    pub title: Option<String>,
    pub variant: String,
    pub density: String,
    pub filter_status: Option<String>,
    pub filter_active: Option<bool>,
    pub filter_type_code: Option<String>,
    pub filter_concept_id: Option<String>,
    pub filter_recent_days: Option<i32>,
    pub sort_by: String,
    pub item_limit: i32,
    pub sort_order: i32,
    pub grid_span: i32,
    pub is_visible: bool,
    pub is_pinned: bool,
    pub is_collapsed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectWrite {
    pub type_code: String,
    pub name: String,
    pub description: Option<String>,
    /// `None` explicitly targets the Player; otherwise the same-world Concept.
    pub target_concept_id: Option<String>,
    pub intensity: i32,
    /// On creation, omitted means the current recorded time; on edit it keeps
    /// the existing start time.
    pub started_at: Option<String>,
    /// `None` explicitly means that no expiry is recorded.
    pub expires_at: Option<String>,
}

pub trait SemanticsStore: Send + Sync {
    fn insert_stage(&self, value: &QuestStage) -> Result<(), StorageError>;
    fn get_stage(&self, id: &EntityId) -> Result<Option<QuestStage>, StorageError>;
    fn list_stages(&self, quest_id: &EntityId) -> Result<Vec<QuestStage>, StorageError>;
    fn insert_branch(&self, value: &QuestBranch) -> Result<(), StorageError>;
    fn list_branches(&self, stage_id: &EntityId) -> Result<Vec<QuestBranch>, StorageError>;
    fn insert_session(&self, value: &QuestSession) -> Result<(), StorageError>;
    fn get_session(&self, id: &EntityId) -> Result<Option<QuestSession>, StorageError>;
    fn update_session(&self, value: &QuestSession) -> Result<(), StorageError>;
    fn list_sessions(
        &self,
        player_id: &EntityId,
        quest_id: Option<&EntityId>,
        stage_id: Option<&EntityId>,
    ) -> Result<Vec<QuestSession>, StorageError>;
    fn get_branch(&self, id: &EntityId) -> Result<Option<QuestBranch>, StorageError>;
    fn create_effect_with_history(
        &self,
        effect: &Effect,
        history: &[EffectHistoryEntry],
        session_link: Option<&SessionEffect>,
    ) -> Result<(), StorageError>;
    fn update_effect_with_history(
        &self,
        effect: &Effect,
        history: &[EffectHistoryEntry],
    ) -> Result<(), StorageError>;
    fn deactivate_effect_with_history(
        &self,
        effect: &Effect,
        history: &[EffectHistoryEntry],
        session_link: Option<&SessionEffect>,
    ) -> Result<(), StorageError>;
    fn list_effect_history(
        &self,
        player_id: &EntityId,
        effect_id: &EntityId,
    ) -> Result<Vec<EffectHistoryEntry>, StorageError>;
    fn link_effect_to_session(
        &self,
        link: &SessionEffect,
        history: &EffectHistoryEntry,
    ) -> Result<(), StorageError>;
    fn unlink_effect_from_session(
        &self,
        player_id: &EntityId,
        link_id: &EntityId,
        removed_at: &Iso8601Timestamp,
        history: &EffectHistoryEntry,
    ) -> Result<(), StorageError>;
    fn list_session_effects(
        &self,
        player_id: &EntityId,
        session_id: &EntityId,
        include_removed: bool,
    ) -> Result<Vec<SessionEffect>, StorageError>;
    fn attach_content(&self, value: &ContentAttachment) -> Result<(), StorageError>;
    fn list_content_attachments(
        &self,
        kind: ContentTargetKind,
        target_id: &EntityId,
    ) -> Result<Vec<ContentAttachment>, StorageError>;
    fn insert_association(&self, value: &ConceptAssociation) -> Result<(), StorageError>;
    fn get_association(&self, id: &EntityId) -> Result<Option<ConceptAssociation>, StorageError>;
    fn update_association(&self, value: &ConceptAssociation) -> Result<(), StorageError>;
    fn list_associations(
        &self,
        concept_id: &EntityId,
        entity_kind: Option<AssociatedEntityKind>,
        entity_id: Option<&str>,
    ) -> Result<Vec<ConceptAssociation>, StorageError>;
    fn list_associations_for_entity(
        &self,
        entity_kind: AssociatedEntityKind,
        entity_id: &str,
    ) -> Result<Vec<ConceptAssociation>, StorageError>;
    fn append_revision(&self, value: &EntityRevision) -> Result<(), StorageError>;
    fn list_revisions(
        &self,
        kind: RevisionTargetKind,
        target_id: &EntityId,
    ) -> Result<Vec<EntityRevision>, StorageError>;
    fn get_revision(&self, id: &EntityId) -> Result<Option<EntityRevision>, StorageError>;
    fn restore_revision(
        &self,
        revision_id: &EntityId,
        now: &Iso8601Timestamp,
        reason: Option<&str>,
    ) -> Result<EntityRevision, StorageError>;
    fn set_lifecycle(
        &self,
        kind: RevisionTargetKind,
        target_id: &EntityId,
        player_id: &EntityId,
        state: LifecycleState,
        occurred_at: &Iso8601Timestamp,
        captured_at: &Iso8601Timestamp,
        reason: Option<&str>,
    ) -> Result<(), StorageError>;
    fn get_lifecycle(
        &self,
        kind: RevisionTargetKind,
        target_id: &EntityId,
    ) -> Result<LifecycleState, StorageError>;
    fn set_presentation(&self, value: &PresentationPreference) -> Result<(), StorageError>;
    fn set_presentation_visibility(
        &self,
        player_id: &EntityId,
        entity_kind: &str,
        entity_id: &EntityId,
        context: &str,
        is_visible: bool,
        updated_at: &Iso8601Timestamp,
    ) -> Result<(), StorageError>;
    fn list_presentation(
        &self,
        player_id: &EntityId,
        context: &str,
    ) -> Result<Vec<PresentationPreference>, StorageError>;
    fn create_workspace(&self, value: &Workspace) -> Result<(), StorageError>;
    fn import_workspace(
        &self,
        workspace: &Workspace,
        panels: &[WorkspacePanel],
    ) -> Result<(), StorageError>;
    fn list_workspaces(&self, player_id: &EntityId) -> Result<Vec<Workspace>, StorageError>;
    fn set_default_workspace(
        &self,
        player_id: &EntityId,
        workspace_id: &EntityId,
        updated_at: &Iso8601Timestamp,
    ) -> Result<(), StorageError>;
    fn rename_workspace(
        &self,
        player_id: &EntityId,
        workspace_id: &EntityId,
        name: &str,
        updated_at: &Iso8601Timestamp,
    ) -> Result<(), StorageError>;
    fn delete_workspace(
        &self,
        player_id: &EntityId,
        workspace_id: &EntityId,
    ) -> Result<(), StorageError>;
    fn save_workspace_panel(
        &self,
        player_id: &EntityId,
        value: &WorkspacePanel,
    ) -> Result<(), StorageError>;
    fn delete_workspace_panel(
        &self,
        player_id: &EntityId,
        workspace_id: &EntityId,
        panel_id: &EntityId,
    ) -> Result<(), StorageError>;
    fn list_workspace_panels(
        &self,
        player_id: &EntityId,
        workspace_id: &EntityId,
    ) -> Result<Vec<WorkspacePanel>, StorageError>;
    fn insert_suggestion(&self, value: &ProgressSuggestion) -> Result<(), StorageError>;
    fn get_suggestion(&self, id: &EntityId) -> Result<Option<ProgressSuggestion>, StorageError>;
    fn list_suggestions(
        &self,
        concept_id: &EntityId,
        include_resolved: bool,
    ) -> Result<Vec<ProgressSuggestion>, StorageError>;
    fn resolve_suggestion(
        &self,
        id: &EntityId,
        player_id: &EntityId,
        accepted: bool,
        at: &Iso8601Timestamp,
    ) -> Result<ProgressSuggestion, StorageError>;
}
impl<T: SemanticsStore + ?Sized> SemanticsStore for std::sync::Arc<T> {
    fn insert_stage(&self, v: &QuestStage) -> Result<(), StorageError> {
        (**self).insert_stage(v)
    }
    fn get_stage(&self, id: &EntityId) -> Result<Option<QuestStage>, StorageError> {
        (**self).get_stage(id)
    }
    fn list_stages(&self, id: &EntityId) -> Result<Vec<QuestStage>, StorageError> {
        (**self).list_stages(id)
    }
    fn insert_branch(&self, v: &QuestBranch) -> Result<(), StorageError> {
        (**self).insert_branch(v)
    }
    fn list_branches(&self, id: &EntityId) -> Result<Vec<QuestBranch>, StorageError> {
        (**self).list_branches(id)
    }
    fn insert_session(&self, v: &QuestSession) -> Result<(), StorageError> {
        (**self).insert_session(v)
    }
    fn get_session(&self, id: &EntityId) -> Result<Option<QuestSession>, StorageError> {
        (**self).get_session(id)
    }
    fn update_session(&self, v: &QuestSession) -> Result<(), StorageError> {
        (**self).update_session(v)
    }
    fn list_sessions(
        &self,
        p: &EntityId,
        q: Option<&EntityId>,
        s: Option<&EntityId>,
    ) -> Result<Vec<QuestSession>, StorageError> {
        (**self).list_sessions(p, q, s)
    }
    fn get_branch(&self, id: &EntityId) -> Result<Option<QuestBranch>, StorageError> {
        (**self).get_branch(id)
    }
    fn create_effect_with_history(
        &self,
        e: &Effect,
        h: &[EffectHistoryEntry],
        l: Option<&SessionEffect>,
    ) -> Result<(), StorageError> {
        (**self).create_effect_with_history(e, h, l)
    }
    fn update_effect_with_history(
        &self,
        e: &Effect,
        h: &[EffectHistoryEntry],
    ) -> Result<(), StorageError> {
        (**self).update_effect_with_history(e, h)
    }
    fn deactivate_effect_with_history(
        &self,
        e: &Effect,
        h: &[EffectHistoryEntry],
        l: Option<&SessionEffect>,
    ) -> Result<(), StorageError> {
        (**self).deactivate_effect_with_history(e, h, l)
    }
    fn list_effect_history(
        &self,
        p: &EntityId,
        e: &EntityId,
    ) -> Result<Vec<EffectHistoryEntry>, StorageError> {
        (**self).list_effect_history(p, e)
    }
    fn link_effect_to_session(
        &self,
        l: &SessionEffect,
        h: &EffectHistoryEntry,
    ) -> Result<(), StorageError> {
        (**self).link_effect_to_session(l, h)
    }
    fn unlink_effect_from_session(
        &self,
        p: &EntityId,
        l: &EntityId,
        at: &Iso8601Timestamp,
        h: &EffectHistoryEntry,
    ) -> Result<(), StorageError> {
        (**self).unlink_effect_from_session(p, l, at, h)
    }
    fn list_session_effects(
        &self,
        p: &EntityId,
        s: &EntityId,
        removed: bool,
    ) -> Result<Vec<SessionEffect>, StorageError> {
        (**self).list_session_effects(p, s, removed)
    }
    fn attach_content(&self, v: &ContentAttachment) -> Result<(), StorageError> {
        (**self).attach_content(v)
    }
    fn list_content_attachments(
        &self,
        k: ContentTargetKind,
        id: &EntityId,
    ) -> Result<Vec<ContentAttachment>, StorageError> {
        (**self).list_content_attachments(k, id)
    }
    fn insert_association(&self, v: &ConceptAssociation) -> Result<(), StorageError> {
        (**self).insert_association(v)
    }
    fn get_association(&self, id: &EntityId) -> Result<Option<ConceptAssociation>, StorageError> {
        (**self).get_association(id)
    }
    fn update_association(&self, v: &ConceptAssociation) -> Result<(), StorageError> {
        (**self).update_association(v)
    }
    fn list_associations(
        &self,
        c: &EntityId,
        k: Option<AssociatedEntityKind>,
        id: Option<&str>,
    ) -> Result<Vec<ConceptAssociation>, StorageError> {
        (**self).list_associations(c, k, id)
    }
    fn list_associations_for_entity(
        &self,
        k: AssociatedEntityKind,
        id: &str,
    ) -> Result<Vec<ConceptAssociation>, StorageError> {
        (**self).list_associations_for_entity(k, id)
    }
    fn append_revision(&self, v: &EntityRevision) -> Result<(), StorageError> {
        (**self).append_revision(v)
    }
    fn list_revisions(
        &self,
        k: RevisionTargetKind,
        id: &EntityId,
    ) -> Result<Vec<EntityRevision>, StorageError> {
        (**self).list_revisions(k, id)
    }
    fn get_revision(&self, id: &EntityId) -> Result<Option<EntityRevision>, StorageError> {
        (**self).get_revision(id)
    }
    fn restore_revision(
        &self,
        id: &EntityId,
        at: &Iso8601Timestamp,
        r: Option<&str>,
    ) -> Result<EntityRevision, StorageError> {
        (**self).restore_revision(id, at, r)
    }
    fn set_lifecycle(
        &self,
        k: RevisionTargetKind,
        id: &EntityId,
        p: &EntityId,
        s: LifecycleState,
        o: &Iso8601Timestamp,
        c: &Iso8601Timestamp,
        r: Option<&str>,
    ) -> Result<(), StorageError> {
        (**self).set_lifecycle(k, id, p, s, o, c, r)
    }
    fn get_lifecycle(
        &self,
        k: RevisionTargetKind,
        id: &EntityId,
    ) -> Result<LifecycleState, StorageError> {
        (**self).get_lifecycle(k, id)
    }
    fn set_presentation(&self, v: &PresentationPreference) -> Result<(), StorageError> {
        (**self).set_presentation(v)
    }
    fn set_presentation_visibility(
        &self,
        player_id: &EntityId,
        entity_kind: &str,
        entity_id: &EntityId,
        context: &str,
        is_visible: bool,
        updated_at: &Iso8601Timestamp,
    ) -> Result<(), StorageError> {
        (**self).set_presentation_visibility(
            player_id,
            entity_kind,
            entity_id,
            context,
            is_visible,
            updated_at,
        )
    }
    fn list_presentation(
        &self,
        p: &EntityId,
        c: &str,
    ) -> Result<Vec<PresentationPreference>, StorageError> {
        (**self).list_presentation(p, c)
    }
    fn create_workspace(&self, v: &Workspace) -> Result<(), StorageError> {
        (**self).create_workspace(v)
    }
    fn import_workspace(
        &self,
        workspace: &Workspace,
        panels: &[WorkspacePanel],
    ) -> Result<(), StorageError> {
        (**self).import_workspace(workspace, panels)
    }
    fn list_workspaces(&self, p: &EntityId) -> Result<Vec<Workspace>, StorageError> {
        (**self).list_workspaces(p)
    }
    fn set_default_workspace(
        &self,
        p: &EntityId,
        w: &EntityId,
        at: &Iso8601Timestamp,
    ) -> Result<(), StorageError> {
        (**self).set_default_workspace(p, w, at)
    }
    fn rename_workspace(
        &self,
        p: &EntityId,
        w: &EntityId,
        n: &str,
        at: &Iso8601Timestamp,
    ) -> Result<(), StorageError> {
        (**self).rename_workspace(p, w, n, at)
    }
    fn delete_workspace(&self, p: &EntityId, w: &EntityId) -> Result<(), StorageError> {
        (**self).delete_workspace(p, w)
    }
    fn save_workspace_panel(
        &self,
        player: &EntityId,
        v: &WorkspacePanel,
    ) -> Result<(), StorageError> {
        (**self).save_workspace_panel(player, v)
    }
    fn delete_workspace_panel(
        &self,
        player: &EntityId,
        w: &EntityId,
        p: &EntityId,
    ) -> Result<(), StorageError> {
        (**self).delete_workspace_panel(player, w, p)
    }
    fn list_workspace_panels(
        &self,
        player: &EntityId,
        w: &EntityId,
    ) -> Result<Vec<WorkspacePanel>, StorageError> {
        (**self).list_workspace_panels(player, w)
    }
    fn insert_suggestion(&self, v: &ProgressSuggestion) -> Result<(), StorageError> {
        (**self).insert_suggestion(v)
    }
    fn get_suggestion(&self, id: &EntityId) -> Result<Option<ProgressSuggestion>, StorageError> {
        (**self).get_suggestion(id)
    }
    fn list_suggestions(
        &self,
        id: &EntityId,
        include: bool,
    ) -> Result<Vec<ProgressSuggestion>, StorageError> {
        (**self).list_suggestions(id, include)
    }
    fn resolve_suggestion(
        &self,
        id: &EntityId,
        p: &EntityId,
        a: bool,
        at: &Iso8601Timestamp,
    ) -> Result<ProgressSuggestion, StorageError> {
        (**self).resolve_suggestion(id, p, a, at)
    }
}
