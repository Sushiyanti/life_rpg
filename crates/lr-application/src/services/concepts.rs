//! Concept domain use cases and read models; Concepts remain distinct from referenced entities.
use crate::{
    AppError, Clock, ConceptStore, SearchHit, SearchQuery, SearchStore, SemanticsStore, WorldStore,
};
use lr_domain::{
    Concept, ConceptAssociation, ConceptEntityKind, ConceptEntityLink, ConceptProgressEntry,
    ConceptProgressTrack, ConceptRelationship, ConceptStateSnapshot, ContentAttachment,
    ContentTargetKind, DateValue, EntityId, Iso8601Timestamp, ProgressSemantics,
    ProgressTrackDefinition, TypeRef,
};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, PartialEq)]
pub struct ConceptDetail {
    pub concept: Concept,
    pub progress_tracks: Vec<ConceptProgressTrack>,
    pub relationships: Vec<ConceptRelationship>,
    pub related_entities: Vec<ConceptEntityLink>,
    pub associations: Vec<ConceptAssociation>,
    pub content: Vec<ContentAttachment>,
    /// Typed global-search rows are the bounded cross-entity projection for future detail UI.
    pub related_search_results: Vec<SearchHit>,
    pub progress_history: Vec<ConceptProgressEntry>,
    pub snapshots: Vec<ConceptStateSnapshot>,
}
pub struct ConceptService<S, C> {
    store: S,
    clock: C,
    sequence: AtomicU64,
}
impl<S, C> ConceptService<S, C>
where
    S: ConceptStore + SearchStore + SemanticsStore + WorldStore,
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
    fn new_id(&self, prefix: &str) -> Result<EntityId, AppError> {
        let now = self.clock.now_unix_nanos();
        let sequence = self.sequence.fetch_add(1, Ordering::SeqCst);
        Ok(EntityId::new(format!("{prefix}-{now:x}-{sequence:x}"))?)
    }
    fn required_concept(&self, id: &str) -> Result<Concept, AppError> {
        self.store
            .get_concept(&EntityId::new(id)?)?
            .ok_or_else(|| AppError::Internal("Concept not found".into()))
    }
    pub fn create_concept(
        &self,
        player_id: &str,
        type_code: &str,
        name: &str,
        description: Option<String>,
    ) -> Result<Concept, AppError> {
        let player_id = EntityId::new(player_id)?;
        self.store
            .get_player(&player_id)?
            .ok_or_else(|| AppError::Internal("player not found".into()))?;
        let definition = self
            .store
            .list_type_definitions(Some("concept"))?
            .into_iter()
            .find(|d| d.type_ref.code == type_code && d.is_active)
            .ok_or_else(|| {
                AppError::Domain(lr_domain::DomainError::invalid_value(
                    "concept type",
                    "type code is missing or inactive",
                ))
            })?;
        let mut value = Concept::new(
            self.new_id("concept")?,
            player_id,
            definition.type_ref,
            name,
            format!("concept-ref-v1-{}", uuid::Uuid::new_v4()),
            self.now()?,
        )?;
        value.description = description.filter(|s| !s.trim().is_empty());
        self.store.insert_concept(&value)?;
        Ok(value)
    }
    pub fn list_concepts(&self, player_id: &str) -> Result<Vec<Concept>, AppError> {
        Ok(self.store.list_concepts(&EntityId::new(player_id)?)?)
    }
    pub fn get_concept(&self, id: &str) -> Result<Option<Concept>, AppError> {
        Ok(self.store.get_concept(&EntityId::new(id)?)?)
    }
    pub fn set_concept_active(&self, id: &str, active: bool) -> Result<Concept, AppError> {
        let mut value = self.required_concept(id)?;
        value.is_active = active;
        value.updated_at = self.now()?;
        self.store.update_concept(&value)?;
        Ok(value)
    }
    pub fn relationship_types(&self) -> Result<Vec<String>, AppError> {
        Ok(self.store.list_concept_relationship_types()?)
    }
    pub fn relate(
        &self,
        source_id: &str,
        target_id: &str,
        relationship_code: &str,
    ) -> Result<ConceptRelationship, AppError> {
        let source = self.required_concept(source_id)?;
        let target = self.required_concept(target_id)?;
        if source.player_id != target.player_id {
            return Err(lr_domain::DomainError::invalid_value(
                "Concept relationship",
                "both Concepts must belong to the same Player",
            )
            .into());
        }
        if !self
            .store
            .list_concept_relationship_types()?
            .iter()
            .any(|code| code == relationship_code)
        {
            return Err(lr_domain::DomainError::invalid_value(
                "Concept relationship",
                "relationship type is missing or inactive",
            )
            .into());
        }
        let value = ConceptRelationship::new(
            self.new_id("concept-rel")?,
            source.player_id,
            source.id,
            target.id,
            TypeRef::new("concept_relationship", relationship_code)?,
            self.now()?,
        )?;
        self.store.insert_concept_relationship(&value)?;
        Ok(value)
    }
    pub fn relationship_list(
        &self,
        concept_id: &str,
    ) -> Result<Vec<ConceptRelationship>, AppError> {
        Ok(self
            .store
            .list_concept_relationships(&EntityId::new(concept_id)?)?)
    }
    pub fn set_relationship_active(
        &self,
        concept_id: &str,
        relationship_id: &str,
        active: bool,
    ) -> Result<ConceptRelationship, AppError> {
        let concept = self.required_concept(concept_id)?;
        let key = EntityId::new(relationship_id)?;
        let mut relationship = self
            .store
            .list_concept_relationships(&concept.id)?
            .into_iter()
            .find(|r| r.id == key)
            .ok_or_else(|| AppError::Internal("Concept relationship not found".into()))?;
        relationship.is_active = active;
        relationship.updated_at = self.now()?;
        self.store.update_concept_relationship(&relationship)?;
        Ok(relationship)
    }
    pub fn progress_definitions(&self) -> Result<Vec<ProgressTrackDefinition>, AppError> {
        Ok(self.store.list_progress_track_definitions()?)
    }
    pub fn define_progress_track(
        &self,
        code: &str,
        name: &str,
        semantics: ProgressSemantics,
        minimum: Option<f64>,
        maximum: Option<f64>,
    ) -> Result<ProgressTrackDefinition, AppError> {
        let value = ProgressTrackDefinition::new(
            self.new_id("progress-definition")?,
            code,
            name,
            semantics,
            minimum,
            maximum,
            self.now()?,
        )?;
        self.store.create_progress_track_definition(&value)?;
        Ok(value)
    }
    pub fn progress_tracks(&self, concept_id: &str) -> Result<Vec<ConceptProgressTrack>, AppError> {
        Ok(self
            .store
            .list_concept_progress(&EntityId::new(concept_id)?)?)
    }
    /// Rules may change only tracks explicitly delegated by the Player.
    pub fn set_progress_control(
        &self,
        concept_id: &str,
        track_code: &str,
        control: lr_domain::ProgressControl,
    ) -> Result<ConceptProgressTrack, AppError> {
        let concept = self.required_concept(concept_id)?;
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
    /// `occurred_at` is when the real-world change happened; `captured_at` is this installation's observation time.
    pub fn set_progress(
        &self,
        concept_id: &str,
        track_code: &str,
        value: f64,
        level: Option<i32>,
        occurred_at: Option<&str>,
    ) -> Result<ConceptProgressTrack, AppError> {
        let concept = self.required_concept(concept_id)?;
        let definition = self
            .store
            .list_progress_track_definitions()?
            .into_iter()
            .find(|d| d.code == track_code && d.is_active)
            .ok_or_else(|| {
                AppError::Domain(lr_domain::DomainError::invalid_value(
                    "progress track",
                    "definition is missing or inactive",
                ))
            })?;
        let captured_at = self.now()?;
        let occurred_at = occurred_at
            .map(|t| Iso8601Timestamp::parse(t.to_owned()))
            .transpose()?
            .unwrap_or_else(|| captured_at.clone());
        let prior = self
            .store
            .list_concept_progress(&concept.id)?
            .into_iter()
            .find(|t| t.track_code == track_code);
        let (track, expected_previous) = match prior {
            Some(mut current) => {
                let previous = current.current_value;
                current.change(&definition, value, level, captured_at.clone())?;
                (current, Some(previous))
            }
            None => (
                ConceptProgressTrack::new(
                    self.new_id("concept-track")?,
                    concept.id.clone(),
                    &definition,
                    value,
                    level,
                    captured_at.clone(),
                )?,
                None,
            ),
        };
        let history = ConceptProgressEntry::new(
            self.new_id("concept-progress")?,
            &track,
            expected_previous,
            occurred_at,
            captured_at.clone(),
        )?;
        let event = crate::rules::RuleEvent::ConceptProgressChanged {
            player_id: concept.player_id.to_string(),
            concept_id: concept.id.to_string(),
            concept_type: concept.concept_type.code.clone(),
            track_code: track.track_code.clone(),
            previous_value: expected_previous,
            current_value: track.current_value,
            level: track.level,
        };
        let operation = crate::rules::RuleOperation::ConceptProgress {
            player_id: concept.player_id,
            source: crate::rules::ProgressMutationSource::Manual,
            track: track.clone(),
            expected_previous,
            history,
        };
        let chain_id = self.new_id("rule-chain")?.to_string();
        crate::services::rule_engine::execute(
            &self.store,
            vec![event],
            vec![operation],
            chain_id,
            captured_at,
            || Ok(self.new_id("rule-exec")?.to_string()),
        )?;
        Ok(track)
    }
    pub fn progress_history(
        &self,
        concept_id: &str,
        track_code: Option<&str>,
    ) -> Result<Vec<ConceptProgressEntry>, AppError> {
        Ok(self
            .store
            .list_concept_progress_history(&EntityId::new(concept_id)?, track_code)?)
    }
    pub fn link_entity(
        &self,
        concept_id: &str,
        kind: ConceptEntityKind,
        entity_id: &str,
    ) -> Result<ConceptEntityLink, AppError> {
        let concept = self.required_concept(concept_id)?;
        let value = ConceptEntityLink {
            concept_id: concept.id,
            player_id: concept.player_id,
            entity_kind: kind,
            entity_id: entity_id.to_owned(),
            created_at: self.now()?,
        };
        self.store.link_entity_to_concept(&value)?;
        Ok(value)
    }
    pub fn unlink_entity(
        &self,
        concept_id: &str,
        kind: ConceptEntityKind,
        entity_id: &str,
    ) -> Result<(), AppError> {
        self.store
            .unlink_entity_from_concept(&EntityId::new(concept_id)?, kind, entity_id)?;
        Ok(())
    }
    pub fn detail(&self, id: &str) -> Result<Option<ConceptDetail>, AppError> {
        let key = EntityId::new(id)?;
        let Some(concept) = self.store.get_concept(&key)? else {
            return Ok(None);
        };
        let mut query = SearchQuery::default();
        query.concept_id = Some(key.clone());
        query.limit = 200;
        query.include_hidden = true;
        query.include_archived = true;
        query.include_trashed = true;
        let related_search_results = self.store.search(&query, &self.now()?)?;
        Ok(Some(ConceptDetail {
            progress_tracks: self.store.list_concept_progress(&key)?,
            relationships: self.store.list_concept_relationships(&key)?,
            related_entities: self.store.list_concept_entity_links(&key, None)?,
            associations: self.store.list_associations(&key, None, None)?,
            content: self.store.list_content_attachments(
                ContentTargetKind::Concept,
                &key,
                false,
            )?,
            related_search_results,
            progress_history: self.store.list_concept_progress_history(&key, None)?,
            snapshots: self.store.list_concept_snapshots(&key)?,
            concept,
        }))
    }
    pub fn capture_snapshot(&self, id: &str, date: &str) -> Result<ConceptStateSnapshot, AppError> {
        let id = EntityId::new(id)?;
        let date = DateValue::parse(date.to_owned())?;
        Ok(self
            .store
            .capture_concept_snapshot(&id, &date, &self.now()?)?)
    }
    pub fn search(&self, query: &SearchQuery) -> Result<Vec<SearchHit>, AppError> {
        query
            .validate()
            .map_err(|e| lr_domain::DomainError::invalid_value("search query", e))?;
        Ok(self.store.search(query, &self.now()?)?)
    }
}
