//! Application use cases for explicit Player-owned Tags.
use crate::{AppError, Clock, TagStore, WorldStore};
use lr_domain::{
    normalize_tag_name, EntityId, Iso8601Timestamp, LifecycleState, Tag, TagMatchMode,
    TagRelationship, TagTargetKind, TagTargetReference, TaggedRecord,
};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_TAG_LIST: u32 = 200;

pub struct TagService<S, C> {
    store: S,
    clock: C,
    sequence: AtomicU64,
}
impl<S, C> TagService<S, C>
where
    S: WorldStore + TagStore,
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
    fn player(&self, player_id: &str) -> Result<EntityId, AppError> {
        let id = EntityId::new(player_id)?;
        self.store
            .get_player(&id)?
            .ok_or_else(|| AppError::Internal("Player not found".into()))?;
        Ok(id)
    }
    fn owned_tag(&self, player: &EntityId, tag_id: &str) -> Result<Tag, AppError> {
        let id = EntityId::new(tag_id)?;
        let tag = self
            .store
            .get_tag(&id)?
            .ok_or_else(|| AppError::Internal("Tag not found".into()))?;
        if &tag.player_id != player {
            return Err(lr_domain::DomainError::invalid_value(
                "Tag",
                "belongs to another Player world",
            )
            .into());
        }
        Ok(tag)
    }
    pub fn create_tag(
        &self,
        player_id: &str,
        name: &str,
        description: Option<String>,
    ) -> Result<Tag, AppError> {
        let player = self.player(player_id)?;
        let normalized = normalize_tag_name(name.trim());
        if normalized.is_empty() {
            return Err(lr_domain::DomainError::invalid_value(
                "Tag name",
                "must contain 1-80 characters",
            )
            .into());
        }
        if self
            .store
            .find_tag_by_normalized_name(&player, &normalized, None)?
            .is_some()
        {
            return Err(lr_domain::DomainError::invalid_value(
                "Tag name",
                "a Tag with this normalized name already exists in this Player world",
            )
            .into());
        }
        let now = self.now()?;
        let value = Tag::new(
            self.id("tag")?,
            player,
            self.id("tag-ref")?.to_string(),
            name,
            description,
            now,
        )?;
        self.store.insert_tag(&value)?;
        self.owned_tag(&value.player_id, value.id.as_str())
    }
    pub fn rename_tag(
        &self,
        player_id: &str,
        tag_id: &str,
        name: &str,
        description: Option<String>,
    ) -> Result<Tag, AppError> {
        let player = self.player(player_id)?;
        let mut value = self.owned_tag(&player, tag_id)?;
        let normalized = normalize_tag_name(name.trim());
        if normalized.is_empty() {
            return Err(lr_domain::DomainError::invalid_value(
                "Tag name",
                "must contain 1-80 characters",
            )
            .into());
        }
        if self
            .store
            .find_tag_by_normalized_name(&player, &normalized, Some(&value.id))?
            .is_some()
        {
            return Err(lr_domain::DomainError::invalid_value(
                "Tag name",
                "a Tag with this normalized name already exists in this Player world",
            )
            .into());
        }
        value.rename(name, description, self.now()?)?;
        self.store.update_tag(&value)?;
        self.owned_tag(&player, value.id.as_str())
    }
    pub fn list_tags(
        &self,
        player_id: &str,
        search: Option<&str>,
        include_archived: bool,
        include_trashed: bool,
    ) -> Result<Vec<Tag>, AppError> {
        let player = self.player(player_id)?;
        if search.is_some_and(|value| value.chars().count() > 256) {
            return Err(lr_domain::DomainError::invalid_value(
                "Tag search",
                "must be at most 256 characters",
            )
            .into());
        }
        Ok(self.store.list_tags(
            &player,
            search,
            include_archived,
            include_trashed,
            MAX_TAG_LIST,
        )?)
    }
    pub fn attach_tag(
        &self,
        player_id: &str,
        tag_id: &str,
        target_kind: TagTargetKind,
        target_id: &str,
    ) -> Result<TagRelationship, AppError> {
        let player = self.player(player_id)?;
        let tag = self.owned_tag(&player, tag_id)?;
        if tag.lifecycle != LifecycleState::Active {
            return Err(lr_domain::DomainError::invalid_value(
                "Tag assignment",
                "only an active Tag may be newly assigned",
            )
            .into());
        }
        let target = EntityId::new(target_id)?;
        if !self
            .store
            .tag_target_belongs_to_player(&player, target_kind, target.as_str())?
        {
            return Err(lr_domain::DomainError::invalid_value(
                "Tag target",
                "target does not exist in this Player world or is not a supported current record",
            )
            .into());
        }
        let value = TagRelationship::new(
            self.id("tagrel")?,
            player,
            tag.id,
            target_kind,
            target.as_str(),
            self.now()?,
        )?;
        self.store.insert_tag_relationship(&value)?;
        Ok(value)
    }
    pub fn detach_tag(
        &self,
        player_id: &str,
        relationship_id: &str,
    ) -> Result<TagRelationship, AppError> {
        let player = self.player(player_id)?;
        let relationship = self.store.remove_tag_relationship(
            &player,
            &EntityId::new(relationship_id)?,
            &self.now()?,
        )?;
        Ok(relationship)
    }
    pub fn tags_for_target(
        &self,
        player_id: &str,
        target_kind: TagTargetKind,
        target_id: &str,
    ) -> Result<Vec<TaggedRecord>, AppError> {
        let player = self.player(player_id)?;
        let target = EntityId::new(target_id)?;
        if !self
            .store
            .tag_target_belongs_to_player(&player, target_kind, target.as_str())?
        {
            return Err(lr_domain::DomainError::invalid_value(
                "Tag target",
                "target does not exist in this Player world or is not supported",
            )
            .into());
        }
        Ok(self
            .store
            .list_tagged_records(&player, target_kind, target.as_str())?)
    }
    pub fn targets_for_tag(
        &self,
        player_id: &str,
        tag_id: &str,
        include_removed: bool,
    ) -> Result<Vec<TagTargetReference>, AppError> {
        let player = self.player(player_id)?;
        let tag = self.owned_tag(&player, tag_id)?;
        let _ = tag;
        Ok(self.store.list_tag_targets(
            &player,
            &EntityId::new(tag_id)?,
            include_removed,
            MAX_TAG_LIST,
        )?)
    }
    pub fn set_lifecycle(
        &self,
        player_id: &str,
        tag_id: &str,
        state: LifecycleState,
        reason: Option<&str>,
    ) -> Result<Tag, AppError> {
        let player = self.player(player_id)?;
        let tag = self.owned_tag(&player, tag_id)?;
        let now = self.now()?;
        self.store
            .set_tag_lifecycle(&player, &tag.id, state, &now, &now, reason)?;
        self.owned_tag(&player, tag.id.as_str())
    }

    /// A convenience for Workspace/Search filter ownership validation.
    pub fn validate_filter_tags(
        &self,
        player_id: &str,
        tag_ids: &[EntityId],
    ) -> Result<bool, AppError> {
        let player = self.player(player_id)?;
        Ok(self
            .store
            .tag_ids_belong_to_player(&player, tag_ids, true)?)
    }

    pub fn default_match_mode(&self) -> TagMatchMode {
        TagMatchMode::Any
    }
}
