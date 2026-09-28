//! SQLite adapter for explicit Player-owned Tags and their closed target vocabulary.
use crate::sqlite_store::SqliteHealthStore;
use lr_application::{StorageError, TagStore};
use lr_domain::*;
use rusqlite::{params, OptionalExtension, Row};

fn op(error: impl std::fmt::Display) -> StorageError {
    StorageError::Operation(error.to_string())
}
fn decode_error(error: impl std::fmt::Display) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error.to_string(),
        )),
    )
}
fn eid(value: String) -> rusqlite::Result<EntityId> {
    EntityId::new(value).map_err(decode_error)
}
fn ts(value: String) -> rusqlite::Result<Iso8601Timestamp> {
    Iso8601Timestamp::parse(value).map_err(decode_error)
}
fn opt_ts(value: Option<String>) -> rusqlite::Result<Option<Iso8601Timestamp>> {
    value.map(ts).transpose()
}

const TAG_COLUMNS: &str = "t.id,t.player_id,t.transfer_key,t.name,t.normalized_name,t.description,t.created_at,t.updated_at,COALESCE(el.state,'active'),(SELECT COUNT(*) FROM tag_relationships tr WHERE tr.player_id=t.player_id AND tr.tag_id=t.id AND tr.removed_at IS NULL)";
fn read_tag(row: &Row<'_>) -> rusqlite::Result<Tag> {
    Ok(Tag {
        id: eid(row.get(0)?)?,
        player_id: eid(row.get(1)?)?,
        transfer_key: row.get(2)?,
        name: row.get(3)?,
        normalized_name: row.get(4)?,
        description: row.get(5)?,
        created_at: ts(row.get(6)?)?,
        updated_at: ts(row.get(7)?)?,
        lifecycle: LifecycleState::parse(&row.get::<_, String>(8)?).map_err(decode_error)?,
        usage_count: u32::try_from(row.get::<_, i64>(9)?).map_err(decode_error)?,
    })
}
fn read_relationship(row: &Row<'_>) -> rusqlite::Result<TagRelationship> {
    Ok(TagRelationship {
        id: eid(row.get(0)?)?,
        player_id: eid(row.get(1)?)?,
        tag_id: eid(row.get(2)?)?,
        target_kind: TagTargetKind::parse(&row.get::<_, String>(3)?).map_err(decode_error)?,
        target_id: row.get(4)?,
        added_at: ts(row.get(5)?)?,
        removed_at: opt_ts(row.get(6)?)?,
    })
}
fn read_tagged_record(row: &Row<'_>) -> rusqlite::Result<TaggedRecord> {
    let relationship = read_relationship(row)?;
    let tag = Tag {
        id: eid(row.get(7)?)?,
        player_id: eid(row.get(8)?)?,
        transfer_key: row.get(9)?,
        name: row.get(10)?,
        normalized_name: row.get(11)?,
        description: row.get(12)?,
        lifecycle: LifecycleState::parse(&row.get::<_, String>(13)?).map_err(decode_error)?,
        usage_count: u32::try_from(row.get::<_, i64>(14)?).map_err(decode_error)?,
        created_at: ts(row.get(15)?)?,
        updated_at: ts(row.get(16)?)?,
    };
    Ok(TaggedRecord { relationship, tag })
}
fn read_target_reference(row: &Row<'_>) -> rusqlite::Result<TagTargetReference> {
    let target_lifecycle = row
        .get::<_, Option<String>>(8)?
        .map(|value| LifecycleState::parse(&value).map_err(decode_error))
        .transpose()?;
    Ok(TagTargetReference {
        relationship: read_relationship(row)?,
        target_name: row.get(7)?,
        target_lifecycle,
    })
}

impl TagStore for SqliteHealthStore {
    fn insert_tag(&self, value: &Tag) -> Result<(), StorageError> {
        self.with_conn(|db| {
            db.execute(
                "INSERT INTO tags(id,player_id,transfer_key,name,normalized_name,description,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                params![value.id.as_str(),value.player_id.as_str(),value.transfer_key,value.name,value.normalized_name,value.description,value.created_at.as_str(),value.updated_at.as_str()],
            ).map_err(op)?;
            Ok(())
        })
    }
    fn get_tag(&self, id: &EntityId) -> Result<Option<Tag>, StorageError> {
        self.with_conn(|db| {
            let sql = format!("SELECT {TAG_COLUMNS} FROM tags t LEFT JOIN entity_lifecycle el ON el.target_kind='tag' AND el.target_id=t.id WHERE t.id=?1");
            db.query_row(&sql, [id.as_str()], read_tag).optional().map_err(op)
        })
    }
    fn find_tag_by_normalized_name(
        &self,
        player: &EntityId,
        normalized: &str,
        excluding: Option<&EntityId>,
    ) -> Result<Option<Tag>, StorageError> {
        self.with_conn(|db| {
            let sql = format!("SELECT {TAG_COLUMNS} FROM tags t LEFT JOIN entity_lifecycle el ON el.target_kind='tag' AND el.target_id=t.id WHERE t.player_id=?1 AND t.normalized_name=?2 AND (?3 IS NULL OR t.id<>?3) LIMIT 1");
            db.query_row(&sql, params![player.as_str(), normalized, excluding.map(EntityId::as_str)], read_tag).optional().map_err(op)
        })
    }
    fn update_tag(&self, value: &Tag) -> Result<(), StorageError> {
        self.with_conn(|db| {
            let changed = db.execute(
                "UPDATE tags SET name=?3,normalized_name=?4,description=?5,updated_at=?6 WHERE id=?1 AND player_id=?2",
                params![value.id.as_str(),value.player_id.as_str(),value.name,value.normalized_name,value.description,value.updated_at.as_str()],
            ).map_err(op)?;
            if changed != 1 { return Err(op("Tag not found in Player world")); }
            Ok(())
        })
    }
    fn list_tags(
        &self,
        player: &EntityId,
        search: Option<&str>,
        include_archived: bool,
        include_trashed: bool,
        limit: u32,
    ) -> Result<Vec<Tag>, StorageError> {
        self.with_conn(|db| {
            let sql = format!("SELECT {TAG_COLUMNS} FROM tags t LEFT JOIN entity_lifecycle el ON el.target_kind='tag' AND el.target_id=t.id WHERE t.player_id=?1 AND (?2 IS NULL OR instr(lower(t.name||' '||COALESCE(t.description,'')),lower(?2))>0) AND (COALESCE(el.state,'active')='active' OR (el.state='archived' AND ?3=1) OR (el.state='trashed' AND ?4=1)) ORDER BY t.normalized_name,t.name COLLATE NOCASE,t.id LIMIT ?5");
            let mut statement = db.prepare(&sql).map_err(op)?;
            let rows = statement.query_map(params![player.as_str(),search,include_archived as i64,include_trashed as i64,limit.clamp(1,200)], read_tag).map_err(op)?;
            rows.collect::<Result<Vec<_>,_>>().map_err(op)
        })
    }
    fn tag_target_belongs_to_player(
        &self,
        player: &EntityId,
        kind: TagTargetKind,
        target_id: &str,
    ) -> Result<bool, StorageError> {
        self.with_conn(|db| {
            let sql = match kind {
                TagTargetKind::Quest => "SELECT EXISTS(SELECT 1 FROM quests WHERE id=?1 AND player_id=?2)",
                TagTargetKind::QuestStage => "SELECT EXISTS(SELECT 1 FROM quest_stages WHERE id=?1 AND player_id=?2)",
                TagTargetKind::QuestBranch => "SELECT EXISTS(SELECT 1 FROM quest_branches WHERE id=?1 AND player_id=?2)",
                TagTargetKind::QuestSession => "SELECT EXISTS(SELECT 1 FROM quest_sessions WHERE id=?1 AND player_id=?2)",
                TagTargetKind::SkillTree => "SELECT EXISTS(SELECT 1 FROM skill_trees WHERE id=?1 AND player_id=?2)",
                TagTargetKind::Skill => "SELECT EXISTS(SELECT 1 FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE s.id=?1 AND t.player_id=?2)",
                TagTargetKind::Concept => "SELECT EXISTS(SELECT 1 FROM concepts WHERE id=?1 AND player_id=?2)",
                TagTargetKind::Effect => "SELECT EXISTS(SELECT 1 FROM effects WHERE id=?1 AND player_id=?2)",
                TagTargetKind::NarrativeEntry => "SELECT EXISTS(SELECT 1 FROM narrative_entries WHERE id=?1 AND player_id=?2)",
                TagTargetKind::Comment => "SELECT EXISTS(SELECT 1 FROM comments WHERE CAST(id AS TEXT)=?1 AND author_player_id=?2)",
            };
            db.query_row(sql, params![target_id,player.as_str()], |row| row.get::<_,i64>(0)).map(|value| value != 0).map_err(op)
        })
    }
    fn insert_tag_relationship(&self, value: &TagRelationship) -> Result<(), StorageError> {
        self.with_conn(|db| {
            db.execute(
                "INSERT INTO tag_relationships(id,player_id,tag_id,target_kind,target_id,added_at,removed_at) VALUES(?1,?2,?3,?4,?5,?6,NULL)",
                params![value.id.as_str(),value.player_id.as_str(),value.tag_id.as_str(),value.target_kind.as_str(),value.target_id,value.added_at.as_str()],
            ).map_err(op)?;
            Ok(())
        })
    }
    fn remove_tag_relationship(
        &self,
        player: &EntityId,
        relationship_id: &EntityId,
        removed_at: &Iso8601Timestamp,
    ) -> Result<TagRelationship, StorageError> {
        self.with_conn_mut(|db| {
            let tx = db.transaction().map_err(op)?;
            let changed = tx.execute("UPDATE tag_relationships SET removed_at=?3 WHERE id=?1 AND player_id=?2 AND removed_at IS NULL", params![relationship_id.as_str(),player.as_str(),removed_at.as_str()]).map_err(op)?;
            if changed != 1 { return Err(op("active Tag relationship not found in Player world")); }
            let value = tx.query_row("SELECT id,player_id,tag_id,target_kind,target_id,added_at,removed_at FROM tag_relationships WHERE id=?1", [relationship_id.as_str()], read_relationship).map_err(op)?;
            tx.commit().map_err(op)?;
            Ok(value)
        })
    }
    fn list_tagged_records(
        &self,
        player: &EntityId,
        kind: TagTargetKind,
        target_id: &str,
    ) -> Result<Vec<TaggedRecord>, StorageError> {
        self.with_conn(|db| {
            let sql = format!("SELECT r.id,r.player_id,r.tag_id,r.target_kind,r.target_id,r.added_at,r.removed_at,t.id,t.player_id,t.transfer_key,t.name,t.normalized_name,t.description,COALESCE(el.state,'active'),(SELECT COUNT(*) FROM tag_relationships tr WHERE tr.player_id=t.player_id AND tr.tag_id=t.id AND tr.removed_at IS NULL),t.created_at,t.updated_at FROM tag_relationships r JOIN tags t ON t.id=r.tag_id AND t.player_id=r.player_id LEFT JOIN entity_lifecycle el ON el.target_kind='tag' AND el.target_id=t.id WHERE r.player_id=?1 AND r.target_kind=?2 AND r.target_id=?3 AND r.removed_at IS NULL ORDER BY t.normalized_name,t.id");
            let mut statement = db.prepare(&sql).map_err(op)?;
            let rows = statement.query_map(params![player.as_str(),kind.as_str(),target_id],read_tagged_record).map_err(op)?;
            rows.collect::<Result<Vec<_>,_>>().map_err(op)
        })
    }
    fn list_tag_targets(
        &self,
        player: &EntityId,
        tag_id: &EntityId,
        include_removed: bool,
        limit: u32,
    ) -> Result<Vec<TagTargetReference>, StorageError> {
        self.with_conn(|db| {
            let sql = "SELECT r.id,r.player_id,r.tag_id,r.target_kind,r.target_id,r.added_at,r.removed_at,
              COALESCE(CASE r.target_kind
               WHEN 'quest' THEN (SELECT title FROM quests WHERE id=r.target_id AND player_id=r.player_id)
               WHEN 'quest_stage' THEN (SELECT title FROM quest_stages WHERE id=r.target_id AND player_id=r.player_id)
               WHEN 'quest_branch' THEN (SELECT title FROM quest_branches WHERE id=r.target_id AND player_id=r.player_id)
               WHEN 'quest_session' THEN (SELECT 'Session · '||status FROM quest_sessions WHERE id=r.target_id AND player_id=r.player_id)
               WHEN 'skill_tree' THEN (SELECT name FROM skill_trees WHERE id=r.target_id AND player_id=r.player_id)
               WHEN 'skill' THEN (SELECT s.name FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE s.id=r.target_id AND t.player_id=r.player_id)
               WHEN 'concept' THEN (SELECT name FROM concepts WHERE id=r.target_id AND player_id=r.player_id)
               WHEN 'effect' THEN (SELECT name FROM effects WHERE id=r.target_id AND player_id=r.player_id)
               WHEN 'narrative_entry' THEN (SELECT title FROM narrative_entries WHERE id=r.target_id AND player_id=r.player_id)
               WHEN 'comment' THEN (SELECT substr(body,1,120) FROM comments WHERE CAST(id AS TEXT)=r.target_id AND author_player_id=r.player_id)
              END,'[record no longer available]') AS target_name,
              (SELECT state FROM entity_lifecycle el WHERE el.target_kind=r.target_kind AND el.target_id=r.target_id AND el.player_id=r.player_id) AS target_lifecycle
              FROM tag_relationships r WHERE r.player_id=?1 AND r.tag_id=?2 AND (?3=1 OR r.removed_at IS NULL)
              ORDER BY r.added_at DESC,r.id DESC LIMIT ?4";
            let mut statement = db.prepare(sql).map_err(op)?;
            let rows = statement.query_map(params![player.as_str(),tag_id.as_str(),include_removed as i64,limit.clamp(1,200)],read_target_reference).map_err(op)?;
            rows.collect::<Result<Vec<_>,_>>().map_err(op)
        })
    }
    fn set_tag_lifecycle(
        &self,
        player: &EntityId,
        tag_id: &EntityId,
        state: LifecycleState,
        occurred: &Iso8601Timestamp,
        captured: &Iso8601Timestamp,
        reason: Option<&str>,
    ) -> Result<(), StorageError> {
        if occurred > captured {
            return Err(op("lifecycle capture cannot precede occurrence"));
        }
        self.with_conn_mut(|db| {
            let tx = db.transaction().map_err(op)?;
            let exists: i64 = tx.query_row("SELECT COUNT(*) FROM tags WHERE id=?1 AND player_id=?2",params![tag_id.as_str(),player.as_str()],|row|row.get(0)).map_err(op)?;
            if exists != 1 { return Err(op("Tag not found in Player world")); }
            let previous: Option<String> = tx.query_row("SELECT state FROM entity_lifecycle WHERE target_kind='tag' AND target_id=?1 AND player_id=?2",params![tag_id.as_str(),player.as_str()],|row|row.get(0)).optional().map_err(op)?;
            if previous.as_deref().unwrap_or("active") == state.as_str() { tx.commit().map_err(op)?; return Ok(()); }
            tx.execute("INSERT INTO entity_lifecycle(target_kind,target_id,player_id,state,updated_at) VALUES('tag',?1,?2,?3,?4) ON CONFLICT(target_kind,target_id) DO UPDATE SET player_id=excluded.player_id,state=excluded.state,updated_at=excluded.updated_at",params![tag_id.as_str(),player.as_str(),state.as_str(),captured.as_str()]).map_err(op)?;
            tx.execute("INSERT INTO entity_lifecycle_history(id,target_kind,target_id,player_id,previous_state,current_state,occurred_at,captured_at,reason) VALUES(lower(hex(randomblob(16))),'tag',?1,?2,?3,?4,?5,?6,?7)",params![tag_id.as_str(),player.as_str(),previous,state.as_str(),occurred.as_str(),captured.as_str(),reason]).map_err(op)?;
            tx.commit().map_err(op)
        })
    }
    fn tag_ids_belong_to_player(
        &self,
        player: &EntityId,
        ids: &[EntityId],
        require_active: bool,
    ) -> Result<bool, StorageError> {
        if ids.is_empty() {
            return Ok(true);
        }
        let serialized =
            serde_json::to_string(&ids.iter().map(EntityId::as_str).collect::<Vec<_>>())
                .map_err(op)?;
        self.with_conn(|db| {
            let count: i64 = db.query_row(
                "SELECT COUNT(DISTINCT t.id) FROM json_each(?2) j JOIN tags t ON t.id=j.value AND t.player_id=?1 LEFT JOIN entity_lifecycle el ON el.target_kind='tag' AND el.target_id=t.id WHERE (?3=0 OR COALESCE(el.state,'active')='active')",
                params![player.as_str(),serialized,require_active as i64],|row|row.get(0),
            ).map_err(op)?;
            Ok(count == ids.len() as i64)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lr_application::{
        Clock, SearchEntityKind, SearchQuery, SearchStore, TagService, WorldService, WorldStore,
    };
    use std::sync::Arc;

    const T0: &str = "2026-09-28T12:00:00Z";
    struct FrozenClock;
    impl Clock for FrozenClock {
        fn now_rfc3339(&self) -> String {
            T0.into()
        }
        fn now_unix_nanos(&self) -> u128 {
            77
        }
    }

    #[test]
    fn tags_are_owned_explicit_historical_and_searchable_without_mutating_targets() {
        let store = Arc::new(SqliteHealthStore::open_in_memory(T0));
        let world = WorldService::new(store.clone(), FrozenClock);
        let player = world.create_player("Ada", None).unwrap();
        let other_player = world.create_player("Grace", None).unwrap();
        let quest = world
            .create_quest(
                player.id.as_str(),
                "main",
                "Learn Rust",
                None,
                None,
                None,
                None,
                Some(0),
            )
            .unwrap();
        let other_quest = world
            .create_quest(
                other_player.id.as_str(),
                "main",
                "Other world quest",
                None,
                None,
                None,
                None,
                Some(0),
            )
            .unwrap();
        let tree = world
            .create_skill_tree(player.id.as_str(), "programming", "Programming")
            .unwrap();
        let skill = world
            .add_skill(tree.id.as_str(), "core", "Rust", None)
            .unwrap();
        let tags = TagService::new(store.clone(), FrozenClock);

        let label = tags
            .create_tag(
                player.id.as_str(),
                "  Learning  ",
                Some(" organize study".into()),
            )
            .unwrap();
        assert_eq!(label.normalized_name, "learning");
        assert!(
            tags.create_tag(player.id.as_str(), "learning", None)
                .is_err(),
            "normalized duplicates are rejected within a Player world"
        );
        assert!(
            tags.attach_tag(
                player.id.as_str(),
                label.id.as_str(),
                TagTargetKind::Quest,
                other_quest.id.as_str()
            )
            .is_err(),
            "cross-Player target assignment is rejected"
        );
        assert!(
            tags.attach_tag(
                player.id.as_str(),
                label.id.as_str(),
                TagTargetKind::Quest,
                "missing-quest"
            )
            .is_err(),
            "unknown targets are rejected"
        );

        let quest_fact = tags
            .attach_tag(
                player.id.as_str(),
                label.id.as_str(),
                TagTargetKind::Quest,
                quest.id.as_str(),
            )
            .unwrap();
        let skill_fact = tags
            .attach_tag(
                player.id.as_str(),
                label.id.as_str(),
                TagTargetKind::Skill,
                skill.id.as_str(),
            )
            .unwrap();
        assert_ne!(quest_fact.id, skill_fact.id);
        assert!(
            tags.attach_tag(
                player.id.as_str(),
                label.id.as_str(),
                TagTargetKind::Quest,
                quest.id.as_str()
            )
            .is_err(),
            "an active relationship cannot be duplicated"
        );
        let renamed = tags
            .rename_tag(
                player.id.as_str(),
                label.id.as_str(),
                "Study",
                Some("renamed without changing identity".into()),
            )
            .unwrap();
        assert_eq!(renamed.id, label.id);
        assert_eq!(renamed.transfer_key, label.transfer_key);
        assert_eq!(renamed.usage_count, 2);

        let removed = tags
            .detach_tag(player.id.as_str(), quest_fact.id.as_str())
            .unwrap();
        assert!(removed.removed_at.is_some());
        let reattached = tags
            .attach_tag(
                player.id.as_str(),
                label.id.as_str(),
                TagTargetKind::Quest,
                quest.id.as_str(),
            )
            .unwrap();
        assert_ne!(
            reattached.id, quest_fact.id,
            "reattachment is a new immutable relationship fact"
        );
        let history = tags
            .targets_for_tag(player.id.as_str(), label.id.as_str(), true)
            .unwrap();
        assert_eq!(history.len(), 3);
        assert!(history
            .iter()
            .any(|item| item.relationship.id == quest_fact.id
                && item.relationship.removed_at.is_some()));
        assert_eq!(
            tags.targets_for_tag(player.id.as_str(), label.id.as_str(), false)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            tags.tags_for_target(player.id.as_str(), TagTargetKind::Quest, quest.id.as_str())
                .unwrap()
                .len(),
            1
        );

        let second = tags
            .create_tag(player.id.as_str(), "Priority", None)
            .unwrap();
        tags.attach_tag(
            player.id.as_str(),
            second.id.as_str(),
            TagTargetKind::Quest,
            quest.id.as_str(),
        )
        .unwrap();
        let time = Iso8601Timestamp::parse(T0).unwrap();
        let all = SearchQuery {
            player_id: Some(player.id.clone()),
            kind: Some(SearchEntityKind::Quest),
            tag_ids: vec![label.id.clone(), second.id.clone()],
            tag_match: TagMatchMode::All,
            ..Default::default()
        };
        let any = SearchQuery {
            tag_match: TagMatchMode::Any,
            ..all.clone()
        };
        assert_eq!(
            store
                .search(&all, &time)
                .unwrap()
                .iter()
                .map(|hit| hit.id.as_str())
                .collect::<Vec<_>>(),
            vec![quest.id.as_str()]
        );
        assert!(store
            .search(&any, &time)
            .unwrap()
            .iter()
            .any(|hit| hit.id == quest.id.as_str()));
        let skill_query = SearchQuery {
            kind: Some(SearchEntityKind::Skill),
            tag_ids: vec![label.id.clone()],
            ..any.clone()
        };
        assert_eq!(
            store.search(&skill_query, &time).unwrap().len(),
            1,
            "the same Tag can organize multiple target kinds"
        );
        let bounded = SearchQuery {
            limit: 1,
            offset: 1,
            ..any.clone()
        };
        assert!(store.search(&bounded, &time).unwrap().is_empty());

        let before = world.store().get_quest(&quest.id).unwrap().unwrap();
        let archived = tags
            .set_lifecycle(
                player.id.as_str(),
                label.id.as_str(),
                LifecycleState::Archived,
                Some("archive organization only"),
            )
            .unwrap();
        assert_eq!(archived.lifecycle, LifecycleState::Archived);
        assert!(
            tags.attach_tag(
                player.id.as_str(),
                label.id.as_str(),
                TagTargetKind::Quest,
                quest.id.as_str()
            )
            .is_err(),
            "archived Tags cannot acquire new assignments"
        );
        let active_only = tags
            .list_tags(player.id.as_str(), None, false, false)
            .unwrap();
        assert!(!active_only.iter().any(|tag| tag.id == label.id));
        assert_eq!(
            tags.tags_for_target(player.id.as_str(), TagTargetKind::Quest, quest.id.as_str())
                .unwrap()
                .len(),
            2,
            "archiving a Tag leaves existing relationship facts and targets intact"
        );
        assert!(
            store.search(&all, &time).unwrap().is_empty(),
            "inactive Tag records do not satisfy current Search filters"
        );
        let restored = tags
            .set_lifecycle(
                player.id.as_str(),
                label.id.as_str(),
                LifecycleState::Active,
                Some("restore organization"),
            )
            .unwrap();
        assert_eq!(restored.lifecycle, LifecycleState::Active);
        assert_eq!(
            tags.list_tags(player.id.as_str(), None, false, false)
                .unwrap()
                .len(),
            2
        );
        let after = world.store().get_quest(&quest.id).unwrap().unwrap();
        assert_eq!(
            after, before,
            "Tag lifecycle and assignment operations do not mutate the target Quest"
        );
        let lifecycle_facts: i64 = store.with_conn(|db| db.query_row("SELECT COUNT(*) FROM entity_lifecycle_history WHERE target_kind='tag' AND target_id=?1", [label.id.as_str()], |row| row.get(0)).map_err(|error| lr_application::StorageError::Operation(error.to_string()))).unwrap();
        assert_eq!(
            lifecycle_facts, 2,
            "archive and restore remain auditable lifecycle facts"
        );
        let other_world_hits = store
            .search(
                &SearchQuery {
                    player_id: Some(other_player.id.clone()),
                    kind: Some(SearchEntityKind::Quest),
                    tag_ids: vec![label.id.clone()],
                    ..Default::default()
                },
                &time,
            )
            .unwrap();
        assert!(
            other_world_hits.is_empty(),
            "foreign Player Tag references cannot expose tagged rows in another Player scope"
        );
        let _ = skill_fact;
    }
}
