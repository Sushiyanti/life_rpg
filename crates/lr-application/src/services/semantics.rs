//! Phase 3.6 use cases. This is an application API only; it deliberately adds no UI.
use crate::{
    rules::{ProgressMutationSource, RuleEvent, RuleOperation},
    AppError, Clock, ConceptStore, EffectWrite, SearchQuery, SearchStore, SemanticsStore,
    WorldStore,
};
use lr_domain::{
    AssociatedEntityKind, ConceptAssociation, ContentAttachment, ContentTargetKind, Effect,
    EffectHistoryEntry, EffectHistoryKind, EntityId, EntityRevision, Iso8601Timestamp,
    LifecycleState, PresentationPreference, ProgressSuggestion, QuestBranch, QuestSession,
    QuestStage, RevisionTargetKind, SessionEffect, SessionEffectRole, SessionStatus, TypeRef,
    Workspace, WorkspacePanel,
};
use std::{
    collections::HashSet,
    sync::atomic::{AtomicU64, Ordering},
};

fn effect_snapshot_value(effect: &Effect) -> serde_json::Value {
    serde_json::json!({
        "id": effect.id.as_str(),
        "playerId": effect.player_id.as_str(),
        "typeCode": effect.effect_type.code,
        "name": effect.name,
        "description": effect.description,
        "targetKind": if effect.target_concept_id.is_some() { "concept" } else { "player" },
        "targetConceptId": effect.target_concept_id.as_ref().map(EntityId::as_str),
        "intensity": effect.intensity,
        "startedAt": effect.started_at.as_str(),
        "expiresAt": effect.expires_at.as_ref().map(Iso8601Timestamp::as_str),
        "deactivatedAt": effect.deactivated_at.as_ref().map(Iso8601Timestamp::as_str),
    })
}

fn effect_snapshot(effect: &Effect) -> String {
    effect_snapshot_value(effect).to_string()
}

fn session_effect_snapshot(effect: &Effect, link: &SessionEffect) -> String {
    serde_json::json!({
        "effect": effect_snapshot_value(effect),
        "relationship": {
            "id": link.id.as_str(),
            "sessionId": link.session_id.as_str(),
            "effectId": link.effect_id.as_str(),
            "role": link.role.as_str(),
            "addedAt": link.added_at.as_str(),
            "removedAt": link.removed_at.as_ref().map(Iso8601Timestamp::as_str),
        }
    })
    .to_string()
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuestActivityDetail {
    pub stages: Vec<QuestStage>,
    pub branches: Vec<QuestBranch>,
    pub sessions: Vec<QuestSession>,
    pub content: Vec<ContentAttachment>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct QuestDetail {
    pub quest: lr_domain::Quest,
    pub activity: QuestActivityDetail,
    pub concept_associations: Vec<ConceptAssociation>,
    pub content_entries: Vec<lr_domain::NarrativeEntry>,
    pub comments: Vec<lr_domain::Comment>,
    pub transactions: Vec<lr_domain::Transaction>,
    pub related_search_results: Vec<crate::SearchHit>,
    pub revisions: Vec<EntityRevision>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct SkillDetail {
    pub skill: lr_domain::Skill,
    pub tree: Option<lr_domain::SkillTree>,
    pub children: Vec<lr_domain::Skill>,
    pub concept_associations: Vec<ConceptAssociation>,
    pub content: Vec<ContentAttachment>,
    pub content_entries: Vec<lr_domain::NarrativeEntry>,
    pub comments: Vec<lr_domain::Comment>,
    pub effects: Vec<lr_domain::Effect>,
    pub snapshots: Vec<lr_domain::SkillStateSnapshot>,
    pub revisions: Vec<EntityRevision>,
    pub related_search_results: Vec<crate::SearchHit>,
}
pub struct SemanticsService<S, C> {
    store: S,
    clock: C,
    sequence: AtomicU64,
}
impl<S, C> SemanticsService<S, C>
where
    S: WorldStore + ConceptStore + SearchStore + SemanticsStore,
    C: Clock,
{
    pub fn new(store: S, clock: C) -> Self {
        Self {
            store,
            clock,
            sequence: AtomicU64::new(0),
        }
    }
    fn now(&self) -> Result<Iso8601Timestamp, AppError> {
        Ok(Iso8601Timestamp::parse(self.clock.now_rfc3339())?)
    }
    fn id(&self, prefix: &str) -> Result<EntityId, AppError> {
        let n = self.sequence.fetch_add(1, Ordering::SeqCst);
        Ok(EntityId::new(format!(
            "{prefix}-{:x}-{n:x}",
            self.clock.now_unix_nanos()
        ))?)
    }
    fn concept(&self, id: &str) -> Result<lr_domain::Concept, AppError> {
        self.store
            .get_concept(&EntityId::new(id)?)?
            .ok_or_else(|| AppError::Internal("Concept not found".into()))
    }
    pub fn create_stage(
        &self,
        player_id: &str,
        quest_id: &str,
        title: &str,
        sort_order: i32,
    ) -> Result<QuestStage, AppError> {
        let p = EntityId::new(player_id)?;
        let q = EntityId::new(quest_id)?;
        let quest = self
            .store
            .get_quest(&q)?
            .ok_or_else(|| AppError::Internal("Quest not found".into()))?;
        if quest.player_id != p {
            return Err(lr_domain::DomainError::invalid_value(
                "Quest Stage",
                "Quest belongs to another Player",
            )
            .into());
        }
        let stage = QuestStage::new(self.id("stage")?, p, q, title, sort_order, self.now()?)?;
        self.store.insert_stage(&stage)?;
        Ok(stage)
    }
    pub fn list_stages(&self, quest_id: &str) -> Result<Vec<QuestStage>, AppError> {
        Ok(self.store.list_stages(&EntityId::new(quest_id)?)?)
    }
    pub fn create_branch(
        &self,
        stage_id: &str,
        title: &str,
        sort_order: i32,
    ) -> Result<QuestBranch, AppError> {
        let id = EntityId::new(stage_id)?;
        let stage = self
            .store
            .get_stage(&id)?
            .ok_or_else(|| AppError::Internal("Quest Stage not found".into()))?;
        let b = QuestBranch::new(
            self.id("branch")?,
            stage.player_id,
            stage.quest_id,
            stage.id,
            title,
            sort_order,
            self.now()?,
        )?;
        self.store.insert_branch(&b)?;
        Ok(b)
    }
    pub fn list_branches(&self, stage_id: &str) -> Result<Vec<QuestBranch>, AppError> {
        Ok(self.store.list_branches(&EntityId::new(stage_id)?)?)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn start_session(
        &self,
        player_id: &str,
        quest_id: Option<&str>,
        stage_id: Option<&str>,
        branch_id: Option<&str>,
        skill_id: Option<&str>,
        concept_id: Option<&str>,
        started_at: Option<&str>,
    ) -> Result<QuestSession, AppError> {
        let p = EntityId::new(player_id)?;
        self.store
            .get_player(&p)?
            .ok_or_else(|| AppError::Internal("Player not found".into()))?;
        let optional = |v: Option<&str>| -> Result<Option<EntityId>, AppError> {
            v.map(EntityId::new).transpose().map_err(Into::into)
        };
        let at = started_at
            .map(Iso8601Timestamp::parse)
            .transpose()?
            .unwrap_or(self.now()?);
        let session = QuestSession::new(
            self.id("session")?,
            p,
            optional(quest_id)?,
            optional(stage_id)?,
            optional(branch_id)?,
            optional(skill_id)?,
            optional(concept_id)?,
            at,
            self.now()?,
        )?;
        self.store.insert_session(&session)?;
        Ok(session)
    }
    pub fn finish_session(
        &self,
        id: &str,
        ended_at: Option<&str>,
        status: SessionStatus,
        result: Option<String>,
        notes: Option<String>,
    ) -> Result<QuestSession, AppError> {
        let id = EntityId::new(id)?;
        let mut session = self
            .store
            .get_session(&id)?
            .ok_or_else(|| AppError::Internal("Quest Session not found".into()))?;
        let end = ended_at
            .map(Iso8601Timestamp::parse)
            .transpose()?
            .unwrap_or(self.now()?);
        let now = self.now()?;
        session.finish(end, status, now)?;
        session.result = result;
        session.notes = notes;
        self.store.update_session(&session)?;
        Ok(session)
    }
    pub fn list_sessions(
        &self,
        player_id: &str,
        quest_id: Option<&str>,
        stage_id: Option<&str>,
    ) -> Result<Vec<QuestSession>, AppError> {
        let p = EntityId::new(player_id)?;
        let q = quest_id.map(EntityId::new).transpose()?;
        let s = stage_id.map(EntityId::new).transpose()?;
        Ok(self.store.list_sessions(&p, q.as_ref(), s.as_ref())?)
    }
    pub fn get_stage(&self, stage_id: &str) -> Result<Option<QuestStage>, AppError> {
        Ok(self.store.get_stage(&EntityId::new(stage_id)?)?)
    }
    pub fn get_branch(&self, branch_id: &str) -> Result<Option<QuestBranch>, AppError> {
        Ok(self.store.get_branch(&EntityId::new(branch_id)?)?)
    }
    fn effect_event(
        &self,
        effect: &Effect,
        session_id: Option<EntityId>,
        kind: EffectHistoryKind,
        recorded_at: Iso8601Timestamp,
        previous: Option<String>,
        current: String,
    ) -> Result<EffectHistoryEntry, AppError> {
        Ok(EffectHistoryEntry {
            id: self.id("effect-history")?,
            player_id: effect.player_id.clone(),
            effect_id: effect.id.clone(),
            session_id,
            kind,
            recorded_at,
            previous_state_json: previous,
            current_state_json: current,
        })
    }
    fn effect_for_player(
        &self,
        player_id: &EntityId,
        effect_id: &EntityId,
    ) -> Result<Effect, AppError> {
        self.store
            .list_effects(player_id, None)?
            .into_iter()
            .find(|item| &item.id == effect_id)
            .ok_or_else(|| AppError::Internal("Effect not found in Player world".into()))
    }
    fn validate_effect_type(&self, code: &str) -> Result<TypeRef, AppError> {
        let available = self.store.list_type_definitions(Some("effect"))?;
        if !available
            .iter()
            .any(|item| item.is_active && item.type_ref.code == code)
        {
            return Err(lr_domain::DomainError::invalid_value(
                "Effect type",
                "must be an active registered Effect type",
            )
            .into());
        }
        Ok(TypeRef::effect(code.to_owned())?)
    }
    fn session_for_player(
        &self,
        player_id: &EntityId,
        session_id: &str,
    ) -> Result<QuestSession, AppError> {
        let id = EntityId::new(session_id)?;
        let session = self
            .store
            .get_session(&id)?
            .ok_or_else(|| AppError::Internal("Session not found".into()))?;
        if &session.player_id != player_id {
            return Err(lr_domain::DomainError::invalid_value(
                "Session",
                "belongs to another Player",
            )
            .into());
        }
        Ok(session)
    }
    pub fn list_effect_types(&self) -> Result<Vec<lr_domain::TypeDefinition>, AppError> {
        Ok(self
            .store
            .list_type_definitions(Some("effect"))?
            .into_iter()
            .filter(|item| item.is_active)
            .collect())
    }
    pub fn create_effect(
        &self,
        player_id: &str,
        value: EffectWrite,
        session_id: Option<&str>,
    ) -> Result<Effect, AppError> {
        let player = self
            .store
            .get_player(&EntityId::new(player_id)?)?
            .ok_or_else(|| AppError::Internal("Player not found".into()))?;
        let effect_type = self.validate_effect_type(&value.type_code)?;
        let now = self.now()?;
        let started_at = value
            .started_at
            .as_deref()
            .map(Iso8601Timestamp::parse)
            .transpose()?
            .unwrap_or_else(|| now.clone());
        let mut effect = Effect::new(
            self.id("effect")?,
            player.id.clone(),
            effect_type,
            value.name,
            started_at.clone(),
        )?;
        effect.description = value.description.filter(|text| !text.trim().is_empty());
        effect.intensity = value.intensity;
        effect.expires_at = value
            .expires_at
            .as_deref()
            .map(Iso8601Timestamp::parse)
            .transpose()?;
        if effect
            .expires_at
            .as_ref()
            .is_some_and(|end| end < &effect.started_at)
        {
            return Err(lr_domain::DomainError::invalid_value(
                "Effect expiry",
                "cannot be before the recorded start",
            )
            .into());
        }
        if let Some(concept_id) = value.target_concept_id {
            let concept = self.concept(&concept_id)?;
            if concept.player_id != player.id {
                return Err(lr_domain::DomainError::invalid_value(
                    "Effect target",
                    "Concept belongs to another Player",
                )
                .into());
            }
            effect.target_concept(concept.id);
        }
        let session = session_id
            .map(|id| self.session_for_player(&player.id, id))
            .transpose()?;
        let snapshot = effect_snapshot(&effect);
        let mut history = vec![self.effect_event(
            &effect,
            session.as_ref().map(|s| s.id.clone()),
            EffectHistoryKind::Created,
            now.clone(),
            None,
            snapshot.clone(),
        )?];
        let link = if let Some(s) = session {
            Some(SessionEffect {
                id: self.id("session-effect")?,
                player_id: player.id.clone(),
                session_id: s.id,
                effect_id: effect.id.clone(),
                role: SessionEffectRole::Applied,
                added_at: now.clone(),
                removed_at: None,
            })
        } else {
            None
        };
        if let Some(link) = &link {
            let linked_snapshot = session_effect_snapshot(&effect, link);
            history.push(self.effect_event(
                &effect,
                Some(link.session_id.clone()),
                EffectHistoryKind::SessionLinked,
                now,
                None,
                linked_snapshot,
            )?);
        }
        self.store
            .create_effect_with_history(&effect, &history, link.as_ref())?;
        Ok(effect)
    }
    pub fn update_effect(
        &self,
        player_id: &str,
        effect_id: &str,
        value: EffectWrite,
    ) -> Result<Effect, AppError> {
        let player_id = EntityId::new(player_id)?;
        let id = EntityId::new(effect_id)?;
        let old = self.effect_for_player(&player_id, &id)?;
        let mut changed = old.clone();
        changed.effect_type = self.validate_effect_type(&value.type_code)?;
        changed.name = value.name.trim().to_owned();
        if changed.name.is_empty() {
            return Err(
                lr_domain::DomainError::invalid_value("Effect name", "must not be blank").into(),
            );
        }
        changed.description = value.description.filter(|text| !text.trim().is_empty());
        changed.intensity = value.intensity;
        changed.target_concept_id = match value.target_concept_id {
            Some(concept_id) => {
                let concept = self.concept(&concept_id)?;
                if concept.player_id != player_id {
                    return Err(lr_domain::DomainError::invalid_value(
                        "Effect target",
                        "Concept belongs to another Player",
                    )
                    .into());
                }
                Some(concept.id)
            }
            None => None,
        };
        if let Some(start) = value.started_at.as_deref() {
            changed.started_at = Iso8601Timestamp::parse(start)?;
        }
        changed.expires_at = value
            .expires_at
            .as_deref()
            .map(Iso8601Timestamp::parse)
            .transpose()?;
        if changed
            .expires_at
            .as_ref()
            .is_some_and(|end| end < &changed.started_at)
        {
            return Err(lr_domain::DomainError::invalid_value(
                "Effect expiry",
                "cannot be before the recorded start",
            )
            .into());
        }
        let details_changed = old.effect_type != changed.effect_type
            || old.name != changed.name
            || old.description != changed.description
            || old.target_concept_id != changed.target_concept_id
            || old.intensity != changed.intensity
            || old.started_at != changed.started_at;
        let expiry_changed = old.expires_at != changed.expires_at;
        if !details_changed && !expiry_changed {
            return Err(
                lr_domain::DomainError::invalid_value("Effect", "no changes to save").into(),
            );
        }
        let now = self.now()?;
        changed.updated_at = now.clone();
        let previous = effect_snapshot(&old);
        let current = effect_snapshot(&changed);
        let mut events = Vec::new();
        if details_changed {
            events.push(self.effect_event(
                &changed,
                None,
                EffectHistoryKind::DetailsChanged,
                now.clone(),
                Some(previous.clone()),
                current.clone(),
            )?);
        }
        if expiry_changed {
            events.push(self.effect_event(
                &changed,
                None,
                EffectHistoryKind::ExpiryChanged,
                now,
                Some(previous),
                current,
            )?);
        }
        self.store.update_effect_with_history(&changed, &events)?;
        Ok(changed)
    }
    pub fn deactivate_effect(
        &self,
        player_id: &str,
        effect_id: &str,
        session_id: Option<&str>,
    ) -> Result<Effect, AppError> {
        let player_id = EntityId::new(player_id)?;
        let id = EntityId::new(effect_id)?;
        let old = self.effect_for_player(&player_id, &id)?;
        let session = session_id
            .map(|sid| self.session_for_player(&player_id, sid))
            .transpose()?;
        let now = self.now()?;
        let mut changed = old.clone();
        changed.deactivate(now.clone())?;
        let previous = effect_snapshot(&old);
        let current = effect_snapshot(&changed);
        let mut history = vec![self.effect_event(
            &changed,
            session.as_ref().map(|s| s.id.clone()),
            EffectHistoryKind::ManuallyDeactivated,
            now.clone(),
            Some(previous.clone()),
            current.clone(),
        )?];
        let link = if let Some(s) = session {
            Some(SessionEffect {
                id: self.id("session-effect")?,
                player_id: player_id.clone(),
                session_id: s.id,
                effect_id: id.clone(),
                role: SessionEffectRole::Removed,
                added_at: now.clone(),
                removed_at: None,
            })
        } else {
            None
        };
        if let Some(link) = &link {
            let linked_snapshot = session_effect_snapshot(&changed, link);
            history.push(self.effect_event(
                &changed,
                Some(link.session_id.clone()),
                EffectHistoryKind::SessionLinked,
                now,
                None,
                linked_snapshot,
            )?);
        }
        self.store
            .deactivate_effect_with_history(&changed, &history, link.as_ref())?;
        Ok(changed)
    }
    pub fn effect_history(
        &self,
        player_id: &str,
        effect_id: &str,
    ) -> Result<Vec<EffectHistoryEntry>, AppError> {
        let player_id = EntityId::new(player_id)?;
        let effect_id = EntityId::new(effect_id)?;
        self.effect_for_player(&player_id, &effect_id)?;
        Ok(self.store.list_effect_history(&player_id, &effect_id)?)
    }
    pub fn link_effect_to_session(
        &self,
        player_id: &str,
        session_id: &str,
        effect_id: &str,
        role: SessionEffectRole,
    ) -> Result<SessionEffect, AppError> {
        let player_id = EntityId::new(player_id)?;
        let session = self.session_for_player(&player_id, session_id)?;
        let effect_id = EntityId::new(effect_id)?;
        let effect = self.effect_for_player(&player_id, &effect_id)?;
        if self
            .store
            .list_session_effects(&player_id, &session.id, false)?
            .iter()
            .any(|link| link.effect_id == effect_id && link.role == role)
        {
            return Err(lr_domain::DomainError::invalid_value(
                "Session Effect",
                "this role is already linked to the Session",
            )
            .into());
        }
        let now = self.now()?;
        let link = SessionEffect {
            id: self.id("session-effect")?,
            player_id: player_id.clone(),
            session_id: session.id.clone(),
            effect_id: effect.id.clone(),
            role,
            added_at: now.clone(),
            removed_at: None,
        };
        let snapshot = session_effect_snapshot(&effect, &link);
        let history = self.effect_event(
            &effect,
            Some(session.id),
            EffectHistoryKind::SessionLinked,
            now,
            None,
            snapshot,
        )?;
        self.store.link_effect_to_session(&link, &history)?;
        Ok(link)
    }
    pub fn unlink_effect_from_session(
        &self,
        player_id: &str,
        session_id: &str,
        link_id: &str,
    ) -> Result<SessionEffect, AppError> {
        let player_id = EntityId::new(player_id)?;
        let session = self.session_for_player(&player_id, session_id)?;
        let link_id = EntityId::new(link_id)?;
        let link = self
            .store
            .list_session_effects(&player_id, &session.id, true)?
            .into_iter()
            .find(|item| item.id == link_id && item.removed_at.is_none())
            .ok_or_else(|| {
                AppError::Internal("Active Session–Effect relationship not found".into())
            })?;
        let effect = self.effect_for_player(&player_id, &link.effect_id)?;
        let now = self.now()?;
        let previous_relation = session_effect_snapshot(&effect, &link);
        let mut removed_link = link.clone();
        removed_link.removed_at = Some(now.clone());
        let current_relation = session_effect_snapshot(&effect, &removed_link);
        let history = self.effect_event(
            &effect,
            Some(session.id.clone()),
            EffectHistoryKind::SessionUnlinked,
            now.clone(),
            Some(previous_relation),
            current_relation,
        )?;
        self.store
            .unlink_effect_from_session(&player_id, &link_id, &now, &history)?;
        let mut removed = link;
        removed.removed_at = Some(now);
        Ok(removed)
    }
    pub fn session_effects(
        &self,
        player_id: &str,
        session_id: &str,
        include_removed: bool,
    ) -> Result<Vec<SessionEffect>, AppError> {
        let player_id = EntityId::new(player_id)?;
        let session = self.session_for_player(&player_id, session_id)?;
        Ok(self
            .store
            .list_session_effects(&player_id, &session.id, include_removed)?)
    }
    pub fn attach_content(
        &self,
        player_id: &str,
        content_id: &str,
        target_kind: ContentTargetKind,
        target_id: &str,
        role: &str,
    ) -> Result<ContentAttachment, AppError> {
        let p = EntityId::new(player_id)?;
        let content = EntityId::new(content_id)?;
        let content_entry = self
            .store
            .get_narrative_entry(&content)?
            .ok_or_else(|| AppError::Internal("Content record not found".into()))?;
        if content_entry.player_id != p || !content_entry.is_active {
            return Err(lr_domain::DomainError::invalid_value(
                "Content attachment",
                "Content must be active and belong to the Player",
            )
            .into());
        }
        if self
            .store
            .get_lifecycle(RevisionTargetKind::NarrativeEntry, &content)?
            != LifecycleState::Active
        {
            return Err(lr_domain::DomainError::invalid_value(
                "Content attachment",
                "archived or trashed Content cannot be newly attached",
            )
            .into());
        }
        let target = EntityId::new(target_id)?;
        self.ensure_content_target_for_player(&p, target_kind, &target)?;
        if !self
            .store
            .list_content_attachment_roles()?
            .iter()
            .any(|code| code == role)
        {
            return Err(lr_domain::DomainError::invalid_value(
                "Content role",
                "must be an active registered content relationship role",
            )
            .into());
        }
        let v = ContentAttachment::new(
            self.id("content-rel")?,
            content,
            p,
            target_kind,
            target,
            role,
            0,
            self.now()?,
        )?;
        self.store.attach_content(&v)?;
        Ok(v)
    }
    pub fn content_for(
        &self,
        player_id: &str,
        kind: ContentTargetKind,
        target_id: &str,
        include_removed: bool,
    ) -> Result<Vec<ContentAttachment>, AppError> {
        let player = EntityId::new(player_id)?;
        let target = EntityId::new(target_id)?;
        self.ensure_content_target_for_player(&player, kind, &target)?;
        Ok(self
            .store
            .list_content_attachments(kind, &target, include_removed)?)
    }
    pub fn remove_content_attachment(
        &self,
        player_id: &str,
        relationship_id: &str,
    ) -> Result<(), AppError> {
        self.store.remove_content_attachment(
            &EntityId::new(player_id)?,
            &EntityId::new(relationship_id)?,
            &self.now()?,
        )?;
        Ok(())
    }
    pub fn content_attachment_roles(&self) -> Result<Vec<String>, AppError> {
        Ok(self.store.list_content_attachment_roles()?)
    }
    pub fn content_relationships(
        &self,
        player_id: &str,
        content_id: &str,
        include_removed: bool,
    ) -> Result<Vec<ContentAttachment>, AppError> {
        let player = EntityId::new(player_id)?;
        let content = self
            .store
            .get_narrative_entry(&EntityId::new(content_id)?)?
            .ok_or_else(|| AppError::Internal("Content record not found".into()))?;
        if content.player_id != player {
            return Err(lr_domain::DomainError::invalid_value(
                "Content record",
                "belongs to another Player",
            )
            .into());
        }
        Ok(self
            .store
            .list_content_relationships(&content.id, include_removed)?)
    }
    fn ensure_content_target_for_player(
        &self,
        player: &EntityId,
        kind: ContentTargetKind,
        target: &EntityId,
    ) -> Result<(), AppError> {
        let same_world = match kind {
            ContentTargetKind::Player => self
                .store
                .get_player(target)?
                .is_some_and(|value| value.id == *player),
            ContentTargetKind::Quest => self
                .store
                .get_quest(target)?
                .is_some_and(|value| value.player_id == *player),
            ContentTargetKind::Stage => self
                .store
                .get_stage(target)?
                .is_some_and(|value| value.player_id == *player),
            ContentTargetKind::Branch => self
                .store
                .get_branch(target)?
                .is_some_and(|value| value.player_id == *player),
            ContentTargetKind::Session => self
                .store
                .get_session(target)?
                .is_some_and(|value| value.player_id == *player),
            ContentTargetKind::Skill => match self.store.get_skill(target)? {
                Some(value) => self
                    .store
                    .get_skill_tree(&value.skill_tree_id)?
                    .is_some_and(|tree| tree.player_id == *player),
                None => false,
            },
            ContentTargetKind::SkillTree => self
                .store
                .get_skill_tree(target)?
                .is_some_and(|value| value.player_id == *player),
            ContentTargetKind::Concept => self
                .store
                .get_concept(target)?
                .is_some_and(|value| value.player_id == *player),
            ContentTargetKind::Effect => self
                .store
                .list_effects(player, None)?
                .iter()
                .any(|value| value.id == *target),
        };
        if same_world {
            Ok(())
        } else {
            Err(lr_domain::DomainError::invalid_value(
                "Content target",
                "must exist in the Player world",
            )
            .into())
        }
    }
    pub fn associate(
        &self,
        concept_id: &str,
        kind: AssociatedEntityKind,
        entity_id: &str,
        role: &str,
    ) -> Result<ConceptAssociation, AppError> {
        let c = self.concept(concept_id)?;
        let v = ConceptAssociation::new(
            self.id("association")?,
            c.player_id,
            c.id,
            kind,
            entity_id,
            role,
            self.now()?,
        )?;
        self.store.insert_association(&v)?;
        Ok(v)
    }
    pub fn associations(
        &self,
        concept_id: &str,
        kind: Option<AssociatedEntityKind>,
        entity_id: Option<&str>,
    ) -> Result<Vec<ConceptAssociation>, AppError> {
        Ok(self
            .store
            .list_associations(&EntityId::new(concept_id)?, kind, entity_id)?)
    }
    pub fn set_association_active(
        &self,
        association_id: &str,
        active: bool,
    ) -> Result<ConceptAssociation, AppError> {
        let id = EntityId::new(association_id)?;
        let mut v = self
            .store
            .get_association(&id)?
            .ok_or_else(|| AppError::Internal("Concept association not found".into()))?;
        v.is_active = active;
        v.updated_at = self.now()?;
        self.store.update_association(&v)?;
        Ok(v)
    }
    pub fn set_lifecycle(
        &self,
        kind: RevisionTargetKind,
        target_id: &str,
        player_id: &str,
        state: LifecycleState,
        reason: Option<&str>,
    ) -> Result<(), AppError> {
        let target = EntityId::new(target_id)?;
        let player = EntityId::new(player_id)?;
        let now = self.now()?;
        let occurred = now.clone();
        self.store
            .set_lifecycle(kind, &target, &player, state, &occurred, &now, reason)?;
        Ok(())
    }
    pub fn lifecycle(
        &self,
        kind: RevisionTargetKind,
        target_id: &str,
    ) -> Result<LifecycleState, AppError> {
        Ok(self.store.get_lifecycle(kind, &EntityId::new(target_id)?)?)
    }
    pub fn revisions(
        &self,
        kind: RevisionTargetKind,
        target_id: &str,
    ) -> Result<Vec<EntityRevision>, AppError> {
        Ok(self
            .store
            .list_revisions(kind, &EntityId::new(target_id)?)?)
    }
    pub fn restore_revision(
        &self,
        id: &str,
        reason: Option<&str>,
    ) -> Result<EntityRevision, AppError> {
        Ok(self
            .store
            .restore_revision(&EntityId::new(id)?, &self.now()?, reason)?)
    }
    pub fn set_presentation(
        &self,
        player_id: &str,
        entity_kind: &str,
        entity_id: &str,
        context: &str,
        is_visible: bool,
        sort_order: i32,
        is_pinned: bool,
        is_collapsed: Option<bool>,
        variant: Option<String>,
        density: Option<String>,
    ) -> Result<PresentationPreference, AppError> {
        let now = self.now()?;
        let mut p = PresentationPreference::new(
            EntityId::new(player_id)?,
            entity_kind,
            EntityId::new(entity_id)?,
            context,
            now.clone(),
        )?;
        p.is_visible = is_visible;
        p.sort_order = sort_order;
        p.is_pinned = is_pinned;
        p.is_collapsed = is_collapsed;
        p.variant = variant;
        p.density = density;
        p.updated_at = now;
        self.store.set_presentation(&p)?;
        Ok(p)
    }
    /// Change contextual visibility without replacing other saved presentation fields.
    pub fn set_presentation_visibility(
        &self,
        player_id: &str,
        entity_kind: &str,
        entity_id: &str,
        context: &str,
        is_visible: bool,
    ) -> Result<(), AppError> {
        let now = self.now()?;
        let preference = PresentationPreference::new(
            EntityId::new(player_id)?,
            entity_kind,
            EntityId::new(entity_id)?,
            context,
            now.clone(),
        )?;
        self.store.set_presentation_visibility(
            &preference.player_id,
            &preference.entity_kind,
            &preference.entity_id,
            &preference.context,
            is_visible,
            &now,
        )?;
        Ok(())
    }
    /// Delegate a track to automation only after an explicit Player choice.
    pub fn set_progress_control(
        &self,
        concept_id: &str,
        track_code: &str,
        control: lr_domain::ProgressControl,
    ) -> Result<lr_domain::ConceptProgressTrack, AppError> {
        let concept = self.concept(concept_id)?;
        let mut track = self
            .store
            .list_concept_progress(&concept.id)?
            .into_iter()
            .find(|t| t.track_code == track_code)
            .ok_or_else(|| AppError::Internal("Concept progress track not found".into()))?;
        let now = self.now()?;
        track.set_control(control, now.clone());
        self.store
            .set_concept_progress_control(&concept.id, track_code, control, &now)?;
        Ok(track)
    }
    pub fn presentation(
        &self,
        player_id: &str,
        context: &str,
    ) -> Result<Vec<PresentationPreference>, AppError> {
        Ok(self
            .store
            .list_presentation(&EntityId::new(player_id)?, context)?)
    }
    pub fn create_workspace(
        &self,
        player_id: &str,
        name: &str,
        template: &str,
        is_default: bool,
    ) -> Result<Workspace, AppError> {
        let player_id = EntityId::new(player_id)?;
        self.store
            .get_player(&player_id)?
            .ok_or_else(|| AppError::Internal("Player not found".into()))?;
        let mut workspace = Workspace::new(
            self.id("workspace")?,
            player_id,
            name,
            template,
            self.now()?,
        )?;
        workspace.is_default = is_default;
        self.store.create_workspace(&workspace)?;
        Ok(workspace)
    }
    /// Import presentation settings through the same domain validation and owner checks as interactive edits.
    /// Persistence stores the workspace and every panel in one transaction.
    pub fn import_workspace(
        &self,
        player_id: &str,
        name: &str,
        template: &str,
        imports: Vec<crate::WorkspacePanelImport>,
    ) -> Result<(Workspace, Vec<WorkspacePanel>), AppError> {
        if imports.len() > 100 {
            return Err(lr_domain::DomainError::invalid_value(
                "workspace import",
                "a workspace may contain at most 100 panels",
            )
            .into());
        }
        let player_id = EntityId::new(player_id)?;
        self.store
            .get_player(&player_id)?
            .ok_or_else(|| AppError::Internal("Player not found".into()))?;
        let now = self.now()?;
        let workspace = Workspace::new(
            self.id("workspace")?,
            player_id.clone(),
            name,
            template,
            now.clone(),
        )?;
        let mut panels = Vec::with_capacity(imports.len());
        for (index, import) in imports.into_iter().enumerate() {
            let panel = WorkspacePanel {
                id: self.id("panel")?,
                workspace_id: workspace.id.clone(),
                panel_type: import.panel_type,
                title: import.title,
                variant: import.variant,
                density: import.density,
                filter_status: import.filter_status,
                filter_active: import.filter_active,
                filter_type_code: import.filter_type_code,
                filter_concept_id: import.filter_concept_id.map(EntityId::new).transpose()?,
                filter_recent_days: import.filter_recent_days,
                sort_by: import.sort_by,
                item_limit: import.item_limit,
                sort_order: import.sort_order,
                grid_span: import.grid_span,
                is_visible: import.is_visible,
                is_pinned: import.is_pinned,
                is_collapsed: import.is_collapsed,
                created_at: now.clone(),
                updated_at: now.clone(),
            };
            panel.validate()?;
            if let Some(concept_id) = panel.filter_concept_id.as_ref() {
                let concept = self.store.get_concept(concept_id)?.ok_or_else(|| {
                    AppError::Internal(format!(
                        "Panel {} references a Concept not found in this world",
                        index + 1
                    ))
                })?;
                if concept.player_id != player_id {
                    return Err(lr_domain::DomainError::invalid_value(
                        "workspace Concept filter",
                        "Concept must belong to the destination Player world",
                    )
                    .into());
                }
            }
            panels.push(panel);
        }
        self.store.import_workspace(&workspace, &panels)?;
        Ok((workspace, panels))
    }
    pub fn list_workspaces(&self, player_id: &str) -> Result<Vec<Workspace>, AppError> {
        Ok(self.store.list_workspaces(&EntityId::new(player_id)?)?)
    }
    pub fn set_default_workspace(
        &self,
        player_id: &str,
        workspace_id: &str,
    ) -> Result<(), AppError> {
        self.store.set_default_workspace(
            &EntityId::new(player_id)?,
            &EntityId::new(workspace_id)?,
            &self.now()?,
        )?;
        Ok(())
    }
    pub fn rename_workspace(
        &self,
        player_id: &str,
        workspace_id: &str,
        name: &str,
    ) -> Result<(), AppError> {
        let player_id = EntityId::new(player_id)?;
        let workspace_id = EntityId::new(workspace_id)?;
        let checked = Workspace::new(
            workspace_id.clone(),
            player_id.clone(),
            name,
            "custom",
            self.now()?,
        )?;
        self.store
            .rename_workspace(&player_id, &workspace_id, &checked.name, &self.now()?)?;
        Ok(())
    }
    pub fn delete_workspace(&self, player_id: &str, workspace_id: &str) -> Result<(), AppError> {
        self.store
            .delete_workspace(&EntityId::new(player_id)?, &EntityId::new(workspace_id)?)?;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn save_workspace_panel(
        &self,
        player_id: &str,
        workspace_id: &str,
        panel_id: Option<&str>,
        panel_type: &str,
        title: Option<String>,
        variant: &str,
        density: &str,
        filter_status: Option<&str>,
        filter_active: Option<bool>,
        filter_type_code: Option<&str>,
        filter_concept_id: Option<&str>,
        filter_recent_days: Option<i32>,
        sort_by: &str,
        item_limit: i32,
        sort_order: i32,
        grid_span: i32,
        is_visible: bool,
        is_pinned: bool,
        is_collapsed: bool,
    ) -> Result<WorkspacePanel, AppError> {
        let now = self.now()?;
        let id = panel_id
            .map(EntityId::new)
            .transpose()?
            .unwrap_or(self.id("panel")?);
        let panel = WorkspacePanel {
            id,
            workspace_id: EntityId::new(workspace_id)?,
            panel_type: panel_type.into(),
            title,
            variant: variant.into(),
            density: density.into(),
            filter_status: filter_status.map(str::to_string),
            filter_active,
            filter_type_code: filter_type_code.map(str::to_string),
            filter_concept_id: filter_concept_id.map(EntityId::new).transpose()?,
            filter_recent_days,
            sort_by: sort_by.into(),
            item_limit,
            sort_order,
            grid_span,
            is_visible,
            is_pinned,
            is_collapsed,
            created_at: now.clone(),
            updated_at: now,
        };
        panel.validate()?;
        self.store
            .save_workspace_panel(&EntityId::new(player_id)?, &panel)?;
        Ok(panel)
    }
    pub fn list_workspace_panels(
        &self,
        player_id: &str,
        workspace_id: &str,
    ) -> Result<Vec<WorkspacePanel>, AppError> {
        Ok(self
            .store
            .list_workspace_panels(&EntityId::new(player_id)?, &EntityId::new(workspace_id)?)?)
    }
    pub fn delete_workspace_panel(
        &self,
        player_id: &str,
        workspace_id: &str,
        panel_id: &str,
    ) -> Result<(), AppError> {
        self.store.delete_workspace_panel(
            &EntityId::new(player_id)?,
            &EntityId::new(workspace_id)?,
            &EntityId::new(panel_id)?,
        )?;
        Ok(())
    }
    pub fn suggest_progress(
        &self,
        player_id: &str,
        concept_id: &str,
        track_code: &str,
        value: f64,
        level: Option<i32>,
        reason: Option<String>,
        source: &str,
    ) -> Result<ProgressSuggestion, AppError> {
        let c = self.concept(concept_id)?;
        if c.player_id.as_str() != player_id {
            return Err(lr_domain::DomainError::invalid_value(
                "progress suggestion",
                "Concept belongs to another Player",
            )
            .into());
        }
        let def = self
            .store
            .list_progress_track_definitions()?
            .into_iter()
            .find(|d| d.code == track_code && d.is_active)
            .ok_or_else(|| AppError::Internal("progress definition missing or inactive".into()))?;
        def.validate_value(value)?;
        let s = ProgressSuggestion::new(
            self.id("suggestion")?,
            EntityId::new(player_id)?,
            c.id,
            track_code,
            value,
            level,
            reason,
            source,
            self.now()?,
        )?;
        self.store.insert_suggestion(&s)?;
        Ok(s)
    }
    pub fn suggestions(
        &self,
        concept_id: &str,
        include_resolved: bool,
    ) -> Result<Vec<ProgressSuggestion>, AppError> {
        Ok(self
            .store
            .list_suggestions(&EntityId::new(concept_id)?, include_resolved)?)
    }
    pub fn accept_suggestion(
        &self,
        player_id: &str,
        suggestion_id: &str,
    ) -> Result<ProgressSuggestion, AppError> {
        let player = EntityId::new(player_id)?;
        let suggestion_id = EntityId::new(suggestion_id)?;
        let suggestion = self
            .store
            .get_suggestion(&suggestion_id)?
            .ok_or_else(|| AppError::Internal("progress suggestion not found".into()))?;
        if suggestion.player_id != player
            || suggestion.status != lr_domain::SuggestionStatus::Pending
        {
            return Err(lr_domain::DomainError::invalid_value(
                "progress suggestion",
                "must be pending and belong to this Player",
            )
            .into());
        }
        let concept = self.concept(suggestion.concept_id.as_str())?;
        if concept.player_id != player {
            return Err(lr_domain::DomainError::invalid_value(
                "progress suggestion",
                "Concept owner mismatch",
            )
            .into());
        }
        let definition = self
            .store
            .list_progress_track_definitions()?
            .into_iter()
            .find(|d| d.code == suggestion.track_code && d.is_active)
            .ok_or_else(|| AppError::Internal("progress definition missing or inactive".into()))?;
        definition.validate_value(suggestion.proposed_value)?;
        let previous = self
            .store
            .list_concept_progress(&concept.id)?
            .into_iter()
            .find(|t| t.track_code == suggestion.track_code);
        let now = self.now()?;
        let track = if let Some(previous_track) = previous.as_ref() {
            let mut updated = previous_track.clone();
            updated.change(
                &definition,
                suggestion.proposed_value,
                suggestion.proposed_level,
                now.clone(),
            )?;
            updated
        } else {
            lr_domain::ConceptProgressTrack::new(
                self.id("progress")?,
                concept.id.clone(),
                &definition,
                suggestion.proposed_value,
                suggestion.proposed_level,
                now.clone(),
            )?
        };
        let history = lr_domain::ConceptProgressEntry::new(
            self.id("progress-history")?,
            &track,
            previous.as_ref().map(|t| t.current_value),
            now.clone(),
            now.clone(),
        )?;
        let event = RuleEvent::ConceptProgressChanged {
            player_id: player.to_string(),
            concept_id: concept.id.to_string(),
            concept_type: concept.concept_type.code.clone(),
            track_code: track.track_code.clone(),
            previous_value: previous.as_ref().map(|t| t.current_value),
            current_value: track.current_value,
            level: track.level,
        };
        let operation = RuleOperation::ConceptProgress {
            player_id: player.clone(),
            source: ProgressMutationSource::Manual,
            track,
            expected_previous: previous.map(|t| t.current_value),
            history,
        };
        let resolve = RuleOperation::ResolveProgressSuggestion {
            suggestion_id: suggestion_id.clone(),
            player_id: player.clone(),
            accepted_at: now.clone(),
        };
        crate::services::rule_engine::execute(
            &self.store,
            vec![event],
            vec![operation, resolve],
            self.id("rule-chain")?.to_string(),
            now,
            || self.id("rule-exec").map(|v| v.to_string()),
        )?;
        let mut accepted = suggestion;
        accepted.status = lr_domain::SuggestionStatus::Accepted;
        accepted.resolved_at = Some(self.now()?);
        Ok(accepted)
    }
    pub fn reject_suggestion(
        &self,
        player_id: &str,
        suggestion_id: &str,
    ) -> Result<ProgressSuggestion, AppError> {
        Ok(self.store.resolve_suggestion(
            &EntityId::new(suggestion_id)?,
            &EntityId::new(player_id)?,
            false,
            &self.now()?,
        )?)
    }
    pub fn quest_detail(&self, id: &str) -> Result<Option<QuestDetail>, AppError> {
        let key = EntityId::new(id)?;
        let Some(quest) = self.store.get_quest(&key)? else {
            return Ok(None);
        };
        let stages = self.store.list_stages(&key)?;
        let mut branches = Vec::new();
        let mut sessions = self
            .store
            .list_sessions(&quest.player_id, Some(&key), None)?;
        let mut content =
            self.store
                .list_content_attachments(ContentTargetKind::Quest, &key, false)?;
        let mut associations = self
            .store
            .list_associations_for_entity(AssociatedEntityKind::Quest, key.as_str())?;
        let mut revisions = self.store.list_revisions(RevisionTargetKind::Quest, &key)?;
        for stage in &stages {
            branches.extend(self.store.list_branches(&stage.id)?);
            content.extend(self.store.list_content_attachments(
                ContentTargetKind::Stage,
                &stage.id,
                false,
            )?);
            associations.extend(
                self.store
                    .list_associations_for_entity(AssociatedEntityKind::Stage, stage.id.as_str())?,
            );
            revisions.extend(
                self.store
                    .list_revisions(RevisionTargetKind::QuestStage, &stage.id)?,
            );
        }
        let mut unique_concepts = HashSet::new();
        let mut concept_associations = Vec::new();
        for v in associations {
            if unique_concepts.insert(v.id.to_string()) {
                concept_associations.push(v)
            }
        }
        for branch in &branches {
            content.extend(self.store.list_content_attachments(
                ContentTargetKind::Branch,
                &branch.id,
                false,
            )?);
            concept_associations.extend(
                self.store.list_associations_for_entity(
                    AssociatedEntityKind::Branch,
                    branch.id.as_str(),
                )?,
            );
            revisions.extend(
                self.store
                    .list_revisions(RevisionTargetKind::QuestBranch, &branch.id)?,
            );
        }
        for session in &sessions {
            content.extend(self.store.list_content_attachments(
                ContentTargetKind::Session,
                &session.id,
                false,
            )?);
            concept_associations.extend(self.store.list_associations_for_entity(
                AssociatedEntityKind::Session,
                session.id.as_str(),
            )?);
            revisions.extend(
                self.store
                    .list_revisions(RevisionTargetKind::QuestSession, &session.id)?,
            );
        }
        let mut content_ids = HashSet::new();
        content.retain(|c| content_ids.insert(c.content_id.to_string()));
        let narratives = self.store.list_narrative_entries(&quest.player_id)?;
        let content_entries = narratives
            .into_iter()
            .filter(|n| content_ids.contains(n.id.as_str()))
            .collect();
        let mut hits = Vec::new();
        let mut concepts = HashSet::new();
        for a in &concept_associations {
            if concepts.insert(a.concept_id.to_string()) {
                let mut q = SearchQuery::default();
                q.concept_id = Some(a.concept_id.clone());
                q.limit = 200;
                q.include_hidden = true;
                q.include_archived = true;
                q.include_trashed = true;
                hits.extend(self.store.search(&q, &self.now()?)?);
            }
        }
        hits.sort_by(|a, b| {
            (a.kind.as_str(), a.id.as_str()).cmp(&(b.kind.as_str(), b.id.as_str()))
        });
        hits.dedup_by(|a, b| a.kind == b.kind && a.id == b.id);
        let tx_ids: HashSet<i64> = hits
            .iter()
            .filter(|h| h.kind == crate::SearchEntityKind::Transaction)
            .filter_map(|h| h.id.parse::<i64>().ok())
            .collect();
        let transactions = self
            .store
            .list_transactions(&quest.player_id, 1000)?
            .into_iter()
            .filter(|t| t.id.is_some_and(|id| tx_ids.contains(&id)))
            .collect();
        let activity = QuestActivityDetail {
            stages,
            branches,
            sessions: std::mem::take(&mut sessions),
            content,
        };
        Ok(Some(QuestDetail {
            quest: quest.clone(),
            activity,
            concept_associations,
            content_entries,
            comments: self
                .store
                .list_comments(lr_domain::CommentTargetKind::Quest, &key)?,
            transactions,
            related_search_results: hits,
            revisions,
        }))
    }
    pub fn skill_detail(&self, id: &str) -> Result<Option<SkillDetail>, AppError> {
        let key = EntityId::new(id)?;
        let Some(skill) = self.store.get_skill(&key)? else {
            return Ok(None);
        };
        let tree = self.store.get_skill_tree(&skill.skill_tree_id)?;
        let player_id = tree
            .as_ref()
            .map(|t| t.player_id.clone())
            .ok_or_else(|| AppError::Internal("Skill tree not found".into()))?;
        let all = self.store.list_skills(&skill.skill_tree_id)?;
        let children = all
            .into_iter()
            .filter(|s| s.parent_skill_id.as_ref() == Some(&key))
            .collect();
        let concept_associations = self
            .store
            .list_associations_for_entity(AssociatedEntityKind::Skill, key.as_str())?;
        let content = self
            .store
            .list_content_attachments(ContentTargetKind::Skill, &key, false)?;
        let ids: HashSet<String> = content.iter().map(|c| c.content_id.to_string()).collect();
        let content_entries = self
            .store
            .list_narrative_entries(&player_id)?
            .into_iter()
            .filter(|n| ids.contains(n.id.as_str()))
            .collect();
        let mut related_search_results = Vec::new();
        let mut concepts = HashSet::new();
        for a in &concept_associations {
            if concepts.insert(a.concept_id.to_string()) {
                let mut q = SearchQuery::default();
                q.concept_id = Some(a.concept_id.clone());
                q.limit = 200;
                q.include_hidden = true;
                q.include_archived = true;
                q.include_trashed = true;
                related_search_results.extend(self.store.search(&q, &self.now()?)?);
            }
        }
        related_search_results.sort_by(|a, b| {
            (a.kind.as_str(), a.id.as_str()).cmp(&(b.kind.as_str(), b.id.as_str()))
        });
        related_search_results.dedup_by(|a, b| a.kind == b.kind && a.id == b.id);
        let effect_ids: HashSet<String> = related_search_results
            .iter()
            .filter(|h| h.kind == crate::SearchEntityKind::Effect)
            .map(|h| h.id.clone())
            .collect();
        let effects = self
            .store
            .list_effects(&player_id, None)?
            .into_iter()
            .filter(|e| effect_ids.contains(e.id.as_str()))
            .collect();
        Ok(Some(SkillDetail {
            skill: skill.clone(),
            tree,
            children,
            concept_associations,
            content,
            content_entries,
            comments: self
                .store
                .list_comments(lr_domain::CommentTargetKind::Skill, &key)?,
            effects,
            snapshots: self.store.list_skill_snapshots(&key)?,
            revisions: self.store.list_revisions(RevisionTargetKind::Skill, &key)?,
            related_search_results,
        }))
    }
    pub fn search(&self, query: &SearchQuery) -> Result<Vec<crate::SearchHit>, AppError> {
        Ok(self.store.search(query, &self.now()?)?)
    }
    #[allow(dead_code)]
    fn type_ref(&self, ns: &str, code: &str) -> Result<TypeRef, AppError> {
        Ok(TypeRef::new(ns, code)?)
    }
}
