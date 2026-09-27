//! SQLite adapters for typed Concepts and the compact world search index.
use crate::sqlite_store::SqliteHealthStore;
use lr_application::{
    ConceptStore, SearchEntityKind, SearchHit, SearchQuery, SearchSort, SearchStore, StorageError,
};
use lr_domain::{
    Concept, ConceptEntityKind, ConceptEntityLink, ConceptProgressEntry, ConceptProgressTrack,
    ConceptRelationship, ConceptStateSnapshot, DateValue, EntityId, Iso8601Timestamp,
    ProgressSemantics, ProgressTrackDefinition, TypeRef,
};
use rusqlite::{params, Row};
fn op(e: impl std::fmt::Display) -> StorageError {
    StorageError::Operation(e.to_string())
}
fn id(s: String) -> Result<EntityId, StorageError> {
    EntityId::new(s).map_err(op)
}
fn ts(s: String) -> Result<Iso8601Timestamp, StorageError> {
    Iso8601Timestamp::parse(s).map_err(op)
}
fn date(s: String) -> Result<DateValue, StorageError> {
    DateValue::parse(s).map_err(op)
}
fn concept(r: &Row<'_>) -> Result<Concept, StorageError> {
    Ok(Concept {
        id: id(r.get(0).map_err(op)?)?,
        player_id: id(r.get(1).map_err(op)?)?,
        transfer_key: r.get(10).map_err(op)?,
        concept_type: TypeRef::new(
            r.get::<_, String>(2).map_err(op)?,
            r.get::<_, String>(3).map_err(op)?,
        )
        .map_err(op)?,
        name: r.get(4).map_err(op)?,
        description: r.get(5).map_err(op)?,
        is_active: r.get::<_, i64>(6).map_err(op)? != 0,
        metadata_json: r.get(7).map_err(op)?,
        created_at: ts(r.get(8).map_err(op)?)?,
        updated_at: ts(r.get(9).map_err(op)?)?,
    })
}
fn relationship(r: &Row<'_>) -> Result<ConceptRelationship, StorageError> {
    Ok(ConceptRelationship {
        id: id(r.get(0).map_err(op)?)?,
        player_id: id(r.get(1).map_err(op)?)?,
        source_concept_id: id(r.get(2).map_err(op)?)?,
        target_concept_id: id(r.get(3).map_err(op)?)?,
        relationship_type: TypeRef::new("concept_relationship", r.get::<_, String>(4).map_err(op)?)
            .map_err(op)?,
        is_active: r.get::<_, i64>(5).map_err(op)? != 0,
        metadata_json: r.get(6).map_err(op)?,
        created_at: ts(r.get(7).map_err(op)?)?,
        updated_at: ts(r.get(8).map_err(op)?)?,
    })
}
fn definition(r: &Row<'_>) -> Result<ProgressTrackDefinition, StorageError> {
    let code: String = r.get(0).map_err(op)?;
    Ok(ProgressTrackDefinition {
        id: id(format!("track-def-{code}"))?,
        code,
        name: r.get(1).map_err(op)?,
        description: r.get(2).map_err(op)?,
        semantics: ProgressSemantics::parse(&r.get::<_, String>(3).map_err(op)?).map_err(op)?,
        minimum: r.get(4).map_err(op)?,
        maximum: r.get(5).map_err(op)?,
        is_active: r.get::<_, i64>(6).map_err(op)? != 0,
        metadata_json: r.get(7).map_err(op)?,
        created_at: ts(r.get(8).map_err(op)?)?,
        updated_at: ts(r.get(9).map_err(op)?)?,
    })
}
fn track(r: &Row<'_>) -> Result<ConceptProgressTrack, StorageError> {
    Ok(ConceptProgressTrack {
        id: id(r.get(0).map_err(op)?)?,
        concept_id: id(r.get(1).map_err(op)?)?,
        track_code: r.get(2).map_err(op)?,
        current_value: r.get(3).map_err(op)?,
        level: r.get(4).map_err(op)?,
        level_name: r.get(9).map_err(op)?,
        progression_label: r.get(10).map_err(op)?,
        control: lr_domain::ProgressControl::parse(&r.get::<_, String>(11).map_err(op)?)
            .map_err(op)?,
        is_active: r.get::<_, i64>(5).map_err(op)? != 0,
        metadata_json: r.get(6).map_err(op)?,
        created_at: ts(r.get(7).map_err(op)?)?,
        updated_at: ts(r.get(8).map_err(op)?)?,
    })
}
fn history(r: &Row<'_>) -> Result<ConceptProgressEntry, StorageError> {
    Ok(ConceptProgressEntry {
        id: id(r.get(0).map_err(op)?)?,
        concept_id: id(r.get(1).map_err(op)?)?,
        track_code: r.get(2).map_err(op)?,
        previous_value: r.get(3).map_err(op)?,
        current_value: r.get(4).map_err(op)?,
        level: r.get(5).map_err(op)?,
        occurred_at: ts(r.get(6).map_err(op)?)?,
        captured_at: ts(r.get(7).map_err(op)?)?,
        metadata_json: r.get(8).map_err(op)?,
    })
}
fn link(r: &Row<'_>) -> Result<ConceptEntityLink, StorageError> {
    Ok(ConceptEntityLink {
        concept_id: id(r.get(0).map_err(op)?)?,
        player_id: id(r.get(1).map_err(op)?)?,
        entity_kind: ConceptEntityKind::parse(&r.get::<_, String>(2).map_err(op)?).map_err(op)?,
        entity_id: r.get(3).map_err(op)?,
        created_at: ts(r.get(4).map_err(op)?)?,
    })
}
impl ConceptStore for SqliteHealthStore {
    fn insert_concept(&self, c: &Concept) -> Result<(), StorageError> {
        self.with_conn(|db|{db.execute("INSERT INTO concepts(id,player_id,concept_type_namespace,concept_type_code,name,description,is_active,metadata_json,created_at,updated_at,transfer_key) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![c.id.as_str(),c.player_id.as_str(),c.concept_type.namespace,c.concept_type.code,c.name,c.description,c.is_active as i64,c.metadata_json,c.created_at.as_str(),c.updated_at.as_str(),c.transfer_key]).map_err(op)?;Ok(())})
    }
    fn get_concept(&self, key: &EntityId) -> Result<Option<Concept>, StorageError> {
        self.with_conn(|db|{let mut s=db.prepare("SELECT id,player_id,concept_type_namespace,concept_type_code,name,description,is_active,metadata_json,created_at,updated_at,transfer_key FROM concepts WHERE id=?1").map_err(op)?;let mut rows=s.query([key.as_str()]).map_err(op)?;rows.next().map_err(op)?.map(concept).transpose()})
    }
    fn update_concept(&self, c: &Concept) -> Result<(), StorageError> {
        self.with_conn(|db|{let n=db.execute("UPDATE concepts SET name=?2,description=?3,is_active=?4,metadata_json=?5,updated_at=?6 WHERE id=?1 AND player_id=?7",params![c.id.as_str(),c.name,c.description,c.is_active as i64,c.metadata_json,c.updated_at.as_str(),c.player_id.as_str()]).map_err(op)?;if n==1{Ok(())}else{Err(op("Concept not found or owner mismatch"))}})
    }
    fn list_concepts(&self, p: &EntityId) -> Result<Vec<Concept>, StorageError> {
        self.with_conn(|db|{let mut s=db.prepare("SELECT id,player_id,concept_type_namespace,concept_type_code,name,description,is_active,metadata_json,created_at,updated_at,transfer_key FROM concepts WHERE player_id=?1 ORDER BY is_active DESC,name COLLATE NOCASE,id").map_err(op)?;let mut rows=s.query([p.as_str()]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(concept(r)?)}Ok(out)})
    }
    fn list_concept_relationship_types(&self) -> Result<Vec<String>, StorageError> {
        self.with_conn(|db| {
            let mut s = db
                .prepare(
                    "SELECT code FROM concept_relationship_types WHERE is_active=1 ORDER BY code",
                )
                .map_err(op)?;
            let out = s
                .query_map([], |r| r.get(0))
                .map_err(op)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(op)?;
            Ok(out)
        })
    }
    fn insert_concept_relationship(&self, v: &ConceptRelationship) -> Result<(), StorageError> {
        self.with_conn(|db|{db.execute("INSERT INTO concept_relationships(id,player_id,source_concept_id,target_concept_id,relationship_code,is_active,metadata_json,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![v.id.as_str(),v.player_id.as_str(),v.source_concept_id.as_str(),v.target_concept_id.as_str(),v.relationship_type.code,v.is_active as i64,v.metadata_json,v.created_at.as_str(),v.updated_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn update_concept_relationship(&self, v: &ConceptRelationship) -> Result<(), StorageError> {
        self.with_conn(|db|{let n=db.execute("UPDATE concept_relationships SET is_active=?2,metadata_json=?3,updated_at=?4 WHERE id=?1 AND player_id=?5",params![v.id.as_str(),v.is_active as i64,v.metadata_json,v.updated_at.as_str(),v.player_id.as_str()]).map_err(op)?;if n==1{Ok(())}else{Err(op("Concept relationship not found or owner mismatch"))}})
    }
    fn list_concept_relationships(
        &self,
        c: &EntityId,
    ) -> Result<Vec<ConceptRelationship>, StorageError> {
        self.with_conn(|db|{let mut s=db.prepare("SELECT id,player_id,source_concept_id,target_concept_id,relationship_code,is_active,metadata_json,created_at,updated_at FROM concept_relationships WHERE source_concept_id=?1 OR target_concept_id=?1 ORDER BY created_at,id").map_err(op)?;let mut rows=s.query([c.as_str()]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(relationship(r)?)}Ok(out)})
    }
    fn list_progress_track_definitions(
        &self,
    ) -> Result<Vec<ProgressTrackDefinition>, StorageError> {
        self.with_conn(|db|{let mut s=db.prepare("SELECT code,name,description,semantics,minimum,maximum,is_active,metadata_json,created_at,updated_at FROM progress_track_definitions WHERE is_active=1 ORDER BY code").map_err(op)?;let mut rows=s.query([]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(definition(r)?)}Ok(out)})
    }
    fn create_progress_track_definition(
        &self,
        d: &ProgressTrackDefinition,
    ) -> Result<(), StorageError> {
        self.with_conn(|db|{db.execute("INSERT INTO progress_track_definitions(code,name,description,semantics,minimum,maximum,is_active,metadata_json,created_at,updated_at)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![d.code,d.name,d.description,d.semantics.as_str(),d.minimum,d.maximum,d.is_active as i64,d.metadata_json,d.created_at.as_str(),d.updated_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn list_concept_progress(
        &self,
        c: &EntityId,
    ) -> Result<Vec<ConceptProgressTrack>, StorageError> {
        self.with_conn(|db|{let mut s=db.prepare("SELECT id,concept_id,track_code,current_value,level,is_active,metadata_json,created_at,updated_at,level_name,progression_label,control FROM concept_progress_tracks WHERE concept_id=?1 ORDER BY track_code").map_err(op)?;let mut rows=s.query([c.as_str()]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(track(r)?)}Ok(out)})
    }
    fn set_concept_progress_control(
        &self,
        concept_id: &EntityId,
        track_code: &str,
        control: lr_domain::ProgressControl,
        updated_at: &Iso8601Timestamp,
    ) -> Result<(), StorageError> {
        self.with_conn(|db| {
            let changed = db.execute(
                "UPDATE concept_progress_tracks SET control=?3,updated_at=?4 WHERE concept_id=?1 AND track_code=?2",
                params![concept_id.as_str(),track_code,control.as_str(),updated_at.as_str()],
            ).map_err(op)?;
            if changed==1 { Ok(()) } else { Err(op("Concept progress track not found")) }
        })
    }
    fn list_concept_progress_history(
        &self,
        c: &EntityId,
        code: Option<&str>,
    ) -> Result<Vec<ConceptProgressEntry>, StorageError> {
        self.with_conn(|db|{let mut s=db.prepare("SELECT id,concept_id,track_code,previous_value,current_value,level,occurred_at,captured_at,metadata_json FROM concept_progress_history WHERE concept_id=?1 AND (?2 IS NULL OR track_code=?2) ORDER BY occurred_at,id").map_err(op)?;let mut rows=s.query(params![c.as_str(),code]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(history(r)?)}Ok(out)})
    }
    fn link_entity_to_concept(&self, l: &ConceptEntityLink) -> Result<(), StorageError> {
        self.with_conn(|db|{db.execute("INSERT INTO concept_entity_links(concept_id,player_id,entity_kind,entity_id,created_at) VALUES(?1,?2,?3,?4,?5)",params![l.concept_id.as_str(),l.player_id.as_str(),l.entity_kind.as_str(),l.entity_id,l.created_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn unlink_entity_from_concept(
        &self,
        c: &EntityId,
        k: ConceptEntityKind,
        e: &str,
    ) -> Result<(), StorageError> {
        self.with_conn(|db|{db.execute("DELETE FROM concept_entity_links WHERE concept_id=?1 AND entity_kind=?2 AND entity_id=?3",params![c.as_str(),k.as_str(),e]).map_err(op)?;Ok(())})
    }
    fn list_concept_entity_links(
        &self,
        c: &EntityId,
        k: Option<ConceptEntityKind>,
    ) -> Result<Vec<ConceptEntityLink>, StorageError> {
        self.with_conn(|db|{let mut s=db.prepare("SELECT concept_id,player_id,entity_kind,entity_id,created_at FROM concept_entity_links WHERE concept_id=?1 AND (?2 IS NULL OR entity_kind=?2) ORDER BY entity_kind,created_at,entity_id").map_err(op)?;let mut rows=s.query(params![c.as_str(),k.map(ConceptEntityKind::as_str)]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(link(r)?)}Ok(out)})
    }
    fn capture_concept_snapshot(
        &self,
        c: &EntityId,
        d: &DateValue,
        at: &Iso8601Timestamp,
    ) -> Result<ConceptStateSnapshot, StorageError> {
        self.with_conn_mut(|db|{let tx=db.transaction().map_err(op)?;let row=tx.query_row("SELECT player_id,concept_type_code,name,description,is_active,metadata_json FROM concepts WHERE id=?1",[c.as_str()],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,Option<String>>(3)?,r.get::<_,i64>(4)?,r.get::<_,String>(5)?))).map_err(op)?;let tracks={let mut s=tx.prepare("SELECT track_code,current_value,level,is_active,metadata_json,updated_at FROM concept_progress_tracks WHERE concept_id=?1 ORDER BY track_code").map_err(op)?;let values=s.query_map([c.as_str()],|r|Ok(serde_json::json!({"trackCode":r.get::<_,String>(0)?,"currentValue":r.get::<_,f64>(1)?,"level":r.get::<_,Option<i32>>(2)?,"isActive":r.get::<_,i64>(3)?!=0,"metadataJson":r.get::<_,String>(4)?,"updatedAt":r.get::<_,String>(5)?}))).map_err(op)?.collect::<Result<Vec<_>,_>>().map_err(op)?;values};let rels={let mut s=tx.prepare("SELECT source_concept_id,target_concept_id,relationship_code,is_active FROM concept_relationships WHERE is_active=1 AND (source_concept_id=?1 OR target_concept_id=?1) ORDER BY id").map_err(op)?;let values=s.query_map([c.as_str()],|r|Ok(serde_json::json!({"sourceConceptId":r.get::<_,String>(0)?,"targetConceptId":r.get::<_,String>(1)?,"relationshipType":r.get::<_,String>(2)?,"isActive":r.get::<_,i64>(3)?!=0}))).map_err(op)?.collect::<Result<Vec<_>,_>>().map_err(op)?;values};let state=serde_json::json!({"schemaVersion":1,"playerId":row.0,"typeCode":row.1,"name":row.2,"description":row.3,"isActive":row.4!=0,"metadataJson":row.5,"progressTracks":tracks,"relationships":rels});let json=serde_json::to_string(&state).map_err(op)?;let mut result=ConceptStateSnapshot::new(c.clone(),d.clone(),json,at.clone()).map_err(op)?;tx.execute("INSERT INTO concept_state_snapshots(concept_id,snapshot_date,state_json,metadata_json,captured_at) VALUES(?1,?2,?3,'{}',?4)",params![c.as_str(),d.as_str(),result.state_json,at.as_str()]).map_err(op)?;result.id=Some(tx.last_insert_rowid());tx.commit().map_err(op)?;Ok(result)})
    }
    fn list_concept_snapshots(
        &self,
        c: &EntityId,
    ) -> Result<Vec<ConceptStateSnapshot>, StorageError> {
        self.with_conn(|db|{let mut s=db.prepare("SELECT id,concept_id,snapshot_date,state_json,metadata_json,captured_at FROM concept_state_snapshots WHERE concept_id=?1 ORDER BY snapshot_date").map_err(op)?;let mut rows=s.query([c.as_str()]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(ConceptStateSnapshot{id:Some(r.get(0).map_err(op)?),concept_id:id(r.get(1).map_err(op)?)?,snapshot_date:date(r.get(2).map_err(op)?)?,state_json:r.get(3).map_err(op)?,metadata_json:r.get(4).map_err(op)?,captured_at:ts(r.get(5).map_err(op)?)?})}Ok(out)})
    }
}
impl SearchStore for SqliteHealthStore {
    fn search(
        &self,
        q: &SearchQuery,
        at: &Iso8601Timestamp,
    ) -> Result<Vec<SearchHit>, StorageError> {
        q.validate().map_err(op)?;
        self.with_conn(|db|{
            let fts=q.fts_query();
            let match_clause=if fts.is_some(){"world_search_fts MATCH ?1"}else{"1=1"};
            let score_expr=if fts.is_some(){"bm25(world_search_fts)"}else{"0.0"};
            let sort=match q.sort {
                SearchSort::Newest=>"f.occurred_at DESC,f.kind,f.entity_id",
                SearchSort::Oldest=>"f.occurred_at ASC,f.kind,f.entity_id",
                SearchSort::Name=>"f.name COLLATE NOCASE ASC,f.kind,f.entity_id",
                SearchSort::Progression=>"progression DESC,f.occurred_at DESC",
                SearchSort::Relevance if fts.is_some()=>"score ASC,f.occurred_at DESC",
                SearchSort::Relevance=>"f.occurred_at DESC,f.kind,f.entity_id",
            };
            let sql=format!(r#"SELECT f.kind,f.entity_id,f.player_id,
                CASE WHEN f.kind='concept' THEN f.entity_id
                     WHEN f.kind='concept_progress' THEN (SELECT concept_id FROM concept_progress_history h WHERE h.id=f.entity_id)
                     WHEN f.kind='quest_session' THEN (SELECT concept_id FROM quest_sessions s WHERE s.id=f.entity_id)
                     WHEN f.kind='effect' THEN (SELECT target_concept_id FROM effects e WHERE e.id=f.entity_id AND e.target_kind='concept')
                     ELSE COALESCE((SELECT concept_id FROM concept_entity_links l WHERE l.entity_kind=f.kind AND l.entity_id=f.entity_id ORDER BY l.created_at LIMIT 1),
                                   (SELECT concept_id FROM concept_associations a WHERE a.entity_kind=f.kind AND a.entity_id=f.entity_id AND a.is_active=1 ORDER BY a.created_at LIMIT 1)) END AS concept_id,
                f.type_code,f.status,
                CASE WHEN f.kind='effect' THEN CASE WHEN EXISTS(SELECT 1 FROM effects e WHERE e.id=f.entity_id AND e.started_at<=?9 AND (e.expires_at IS NULL OR e.expires_at>?9) AND (e.deactivated_at IS NULL OR e.deactivated_at>?9)) THEN 1 ELSE 0 END ELSE f.active END AS active,
                f.occurred_at,
                CASE WHEN f.kind='transaction' THEN (SELECT captured_at FROM transactions tx WHERE CAST(tx.id AS TEXT)=f.entity_id)
                     WHEN f.kind='concept_progress' THEN (SELECT captured_at FROM concept_progress_history h WHERE h.id=f.entity_id) ELSE NULL END AS captured_at,
                f.name,COALESCE(CASE WHEN ?1 IS NULL THEN substr(f.body,1,240) ELSE snippet(world_search_fts,8,'','',' …',18) END,'') AS snippet,
                CASE WHEN f.kind='concept' THEN (SELECT MAX(current_value) FROM concept_progress_tracks t WHERE t.concept_id=f.entity_id AND t.is_active=1)
                     WHEN f.kind='concept_progress' THEN (SELECT current_value FROM concept_progress_history h WHERE h.id=f.entity_id) ELSE NULL END AS progression,
                {score_expr} AS score,COALESCE(el.state,'active') AS lifecycle,
                CASE WHEN ?13 IS NULL THEN NULL ELSE COALESCE(pp.is_visible,1) END AS visible
                FROM world_search_fts f
                LEFT JOIN entity_lifecycle el ON el.target_kind=f.kind AND el.target_id=f.entity_id
                LEFT JOIN presentation_preferences pp ON pp.player_id=f.player_id AND pp.entity_kind=f.kind AND pp.entity_id=f.entity_id AND pp.context=?13
                WHERE {match_clause}
                  AND (?2 IS NULL OR f.kind=?2) AND (?3 IS NULL OR f.player_id=?3)
                  AND (?4 IS NULL OR f.type_code=?4) AND (?5 IS NULL OR f.status=?5)
                  AND (?6 IS NULL OR f.occurred_at>=?6) AND (?7 IS NULL OR f.occurred_at<=?7)
                  AND (?8 IS NULL OR (CASE WHEN f.kind='effect' THEN CASE WHEN EXISTS(SELECT 1 FROM effects e WHERE e.id=f.entity_id AND e.started_at<=?9 AND (e.expires_at IS NULL OR e.expires_at>?9) AND (e.deactivated_at IS NULL OR e.deactivated_at>?9)) THEN 1 ELSE 0 END ELSE f.active END)=?8)
                  AND (?10 IS NULL OR ((f.kind='concept' AND (f.entity_id=?10 OR EXISTS(SELECT 1 FROM concept_relationships r WHERE r.source_concept_id=f.entity_id AND r.target_concept_id=?10 AND r.is_active=1) OR EXISTS(SELECT 1 FROM concept_relationships r WHERE r.target_concept_id=f.entity_id AND r.source_concept_id=?10 AND r.is_active=1)))
                     OR (f.kind='concept_progress' AND EXISTS(SELECT 1 FROM concept_progress_history h WHERE h.id=f.entity_id AND h.concept_id=?10))
                     OR (f.kind='quest_session' AND EXISTS(SELECT 1 FROM quest_sessions s WHERE s.id=f.entity_id AND s.concept_id=?10))
                     OR EXISTS(SELECT 1 FROM concept_entity_links l WHERE l.entity_kind=f.kind AND l.entity_id=f.entity_id AND l.concept_id=?10)
                     OR EXISTS(SELECT 1 FROM concept_associations a WHERE a.entity_kind=f.kind AND a.entity_id=f.entity_id AND a.concept_id=?10 AND a.is_active=1)
                     OR (f.kind='effect' AND EXISTS(SELECT 1 FROM effects e WHERE e.id=f.entity_id AND e.target_kind='concept' AND e.target_concept_id=?10))))
                  AND (COALESCE(el.state,'active')='active' OR (COALESCE(el.state,'active')='archived' AND ?15=1) OR (COALESCE(el.state,'active')='trashed' AND ?16=1))
                  AND (?13 IS NULL OR ?14=1 OR COALESCE(pp.is_visible,1)=1)
                ORDER BY {sort} LIMIT ?11 OFFSET ?12"#);
            let mut stmt=db.prepare(&sql).map_err(op)?;
            let mut rows=stmt.query(params![fts,q.kind.map(SearchEntityKind::as_str),q.player_id.as_ref().map(EntityId::as_str),q.type_code,q.status,q.from.as_ref().map(Iso8601Timestamp::as_str),q.through.as_ref().map(Iso8601Timestamp::as_str),q.active.map(i64::from),at.as_str(),q.concept_id.as_ref().map(EntityId::as_str),q.limit,q.offset,q.context,q.include_hidden as i64,q.include_archived as i64,q.include_trashed as i64]).map_err(op)?;
            let mut out=Vec::new();
            while let Some(r)=rows.next().map_err(op)?{
                let kind_raw:String=r.get(0).map_err(op)?;
                let kind=SearchEntityKind::parse(&kind_raw).ok_or_else(||op("unknown search entity kind"))?;
                let lifecycle_raw:String=r.get(13).map_err(op)?;
                out.push(SearchHit{
                    kind,id:r.get(1).map_err(op)?,player_id:r.get::<_,Option<String>>(2).map_err(op)?.map(id).transpose()?,concept_id:r.get::<_,Option<String>>(3).map_err(op)?.map(id).transpose()?,
                    type_code:r.get(4).map_err(op)?,status:r.get(5).map_err(op)?,active:r.get::<_,Option<i64>>(6).map_err(op)?.map(|v|v!=0),lifecycle:lr_domain::LifecycleState::parse(&lifecycle_raw).map_err(op)?,visible:r.get::<_,Option<i64>>(14).map_err(op)?.map(|v|v!=0),
                    occurred_at:r.get::<_,Option<String>>(7).map_err(op)?.map(ts).transpose()?,captured_at:r.get::<_,Option<String>>(8).map_err(op)?.map(ts).transpose()?,name:r.get(9).map_err(op)?,snippet:r.get(10).map_err(op)?,progression:r.get(11).map_err(op)?,relevance:r.get(12).map_err(op)?,
                });
            }
            Ok(out)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lr_application::{
        Clock, Comparison, ConceptService, EventKind, NumericSubject, RuleAction, RuleCondition,
        SearchEntityKind, SearchSort, SemanticsService, TextComparison, TextSubject, WorldService,
        WorldStore,
    };
    use std::sync::Arc;
    const T0: &str = "2026-09-27T10:00:00Z";
    struct FrozenClock;
    impl Clock for FrozenClock {
        fn now_rfc3339(&self) -> String {
            T0.into()
        }
        fn now_unix_nanos(&self) -> u128 {
            44
        }
    }
    fn store() -> SqliteHealthStore {
        SqliteHealthStore::open_in_memory(T0)
    }
    #[test]
    fn concepts_progress_rules_snapshots_links_effect_targets_and_search_work_together() {
        let store = Arc::new(store());
        let world = WorldService::new(store.clone(), FrozenClock);
        let player = world.create_player("Ada", None).unwrap();
        let concepts = ConceptService::new(store.clone(), FrozenClock);
        let python = concepts
            .create_concept(
                player.id.as_str(),
                "subject",
                "Python",
                Some("Programming language".into()),
            )
            .unwrap();
        let health = concepts
            .create_concept(player.id.as_str(), "life_area", "Health", None)
            .unwrap();
        let rel = concepts
            .relate(python.id.as_str(), health.id.as_str(), "depends_on")
            .unwrap();
        assert_eq!(rel.relationship_type.code, "depends_on");
        assert!(
            !concepts
                .set_relationship_active(python.id.as_str(), rel.id.as_str(), false)
                .unwrap()
                .is_active
        );
        assert!(
            concepts
                .set_relationship_active(python.id.as_str(), rel.id.as_str(), true)
                .unwrap()
                .is_active
        );
        assert!(
            !concepts
                .set_concept_active(health.id.as_str(), false)
                .unwrap()
                .is_active
        );
        assert!(
            concepts
                .set_concept_active(health.id.as_str(), true)
                .unwrap()
                .is_active
        );
        let quest = world
            .create_quest(
                player.id.as_str(),
                "main",
                "Learn Python",
                None,
                None,
                None,
                None,
                Some(0),
            )
            .unwrap();
        concepts
            .link_entity(
                python.id.as_str(),
                ConceptEntityKind::Quest,
                quest.id.as_str(),
            )
            .unwrap();
        concepts
            .set_progress(health.id.as_str(), "familiarity", 0.0, None, None)
            .unwrap();
        concepts
            .set_progress_control(
                health.id.as_str(),
                "familiarity",
                lr_domain::ProgressControl::RuleControlled,
            )
            .unwrap();
        let rule = RuleCondition::TextCompare {
            subject: TextSubject::ConceptTrackCode,
            comparison: TextComparison::Equal,
            value: "progress".into(),
        };
        world
            .create_rule(
                "increase familiarity",
                None,
                0,
                EventKind::ConceptProgressChanged,
                rule,
                vec![RuleAction::SetConceptProgress {
                    concept_id: health.id.to_string(),
                    track_code: "familiarity".into(),
                    value: 5.0,
                    level: None,
                }],
            )
            .unwrap();
        concepts
            .set_progress(
                python.id.as_str(),
                "progress",
                25.0,
                None,
                Some("2026-09-27T09:00:00Z"),
            )
            .unwrap();
        concepts
            .set_progress(
                python.id.as_str(),
                "progress",
                30.0,
                None,
                Some("2026-09-27T09:30:00Z"),
            )
            .unwrap();
        assert_eq!(
            concepts.progress_tracks(health.id.as_str()).unwrap()[0].current_value,
            5.0
        );
        assert_eq!(
            concepts
                .progress_history(python.id.as_str(), None)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            concepts
                .progress_history(health.id.as_str(), None)
                .unwrap()
                .len(),
            3
        );
        assert_eq!(world.list_rule_executions(100).unwrap().len(), 4);
        let history = concepts
            .progress_history(python.id.as_str(), Some("progress"))
            .unwrap();
        assert!(history[0].occurred_at < history[0].captured_at);
        assert_ne!(history[0].occurred_at, history[1].occurred_at);
        let snapshot = concepts
            .capture_snapshot(python.id.as_str(), "2026-09-27")
            .unwrap();
        assert!(snapshot.state_json.contains("progressTracks"));
        assert!(
            concepts
                .capture_snapshot(python.id.as_str(), "2026-09-27")
                .is_err(),
            "daily snapshots reject duplicate dates"
        );
        assert_eq!(
            concepts
                .detail(python.id.as_str())
                .unwrap()
                .unwrap()
                .related_entities
                .len(),
            1
        );
        let query = SearchQuery {
            text: Some("learn python".into()),
            kind: Some(SearchEntityKind::Quest),
            concept_id: Some(python.id.clone()),
            sort: SearchSort::Relevance,
            ..Default::default()
        };
        let hits = concepts.search(&query).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, quest.id.as_str());
        assert_eq!(hits[0].concept_id.as_ref(), Some(&python.id));
        let active = SearchQuery {
            kind: Some(SearchEntityKind::Quest),
            active: Some(true),
            ..Default::default()
        };
        let active_hits = concepts.search(&active).unwrap();
        assert_eq!(active_hits.len(), 1);
        assert_eq!(active_hits[0].id, quest.id.as_str());
        let filtered = SearchQuery {
            type_code: Some("main".into()),
            status: Some("open".into()),
            from: Some(Iso8601Timestamp::parse("2026-09-27T09:00:00Z").unwrap()),
            through: Some(Iso8601Timestamp::parse(T0).unwrap()),
            sort: SearchSort::Name,
            limit: 1,
            ..Default::default()
        };
        let typed = concepts.search(&filtered).unwrap();
        assert_eq!(typed.len(), 1);
        assert_eq!(typed[0].id, quest.id.as_str());
        let paged = SearchQuery {
            type_code: Some("main".into()),
            status: Some("open".into()),
            limit: 1,
            offset: 1,
            ..Default::default()
        };
        assert!(concepts.search(&paged).unwrap().is_empty());
        let related = SearchQuery {
            kind: Some(SearchEntityKind::Concept),
            concept_id: Some(python.id.clone()),
            ..Default::default()
        };
        let related_hits = concepts.search(&related).unwrap();
        assert!(
            related_hits.iter().any(|h| h.id == health.id.as_str()),
            "active Concept relationships drive related-Concept filtering"
        );
        let query = SearchQuery {
            text: Some("python".into()),
            kind: Some(SearchEntityKind::Concept),
            ..Default::default()
        };
        assert_eq!(concepts.search(&query).unwrap()[0].id, python.id.as_str());
        let mut effect = lr_domain::Effect::new(
            EntityId::new("concept-effect").unwrap(),
            player.id.clone(),
            TypeRef::effect("condition").unwrap(),
            "Focused on Python",
            Iso8601Timestamp::parse(T0).unwrap(),
        )
        .unwrap();
        effect.target_concept(python.id.clone());
        store.insert_effect(&effect).unwrap();
        assert_eq!(
            store.list_effects(&player.id, Some(T0)).unwrap()[0].target_concept_id,
            Some(python.id.clone())
        );
        let effect_query = SearchQuery {
            text: Some("focused python".into()),
            kind: Some(SearchEntityKind::Effect),
            concept_id: Some(python.id.clone()),
            active: Some(true),
            ..Default::default()
        };
        assert_eq!(concepts.search(&effect_query).unwrap().len(), 1);
        let bad_link = ConceptEntityLink {
            concept_id: python.id.clone(),
            player_id: player.id.clone(),
            entity_kind: ConceptEntityKind::Quest,
            entity_id: "not-a-real-quest".into(),
            created_at: Iso8601Timestamp::parse(T0).unwrap(),
        };
        assert!(
            store.link_entity_to_concept(&bad_link).is_err(),
            "missing polymorphic targets are rejected"
        );
        let other_player = world.create_player("Grace", None).unwrap();
        let other_concept = concepts
            .create_concept(other_player.id.as_str(), "subject", "Other World", None)
            .unwrap();
        assert!(
            concepts
                .relate(python.id.as_str(), other_concept.id.as_str(), "related_to")
                .is_err(),
            "cross-player Concept relationships are rejected"
        );
        let deletion = store
            .with_conn(
                |db| Ok(db.execute("DELETE FROM concepts WHERE id=?1", [python.id.as_str()])),
            )
            .unwrap();
        assert!(
            deletion.is_err(),
            "Concepts with immutable history/snapshots or targets cannot be cascade-deleted"
        );
        let event_time = Iso8601Timestamp::parse("2026-09-27T08:14:00Z").unwrap();
        let capture_time = Iso8601Timestamp::parse(T0).unwrap();
        let transaction = lr_domain::Transaction::new(
            player.id.clone(),
            TypeRef::transaction("xp").unwrap(),
            "xp",
            7,
            event_time.clone(),
        )
        .unwrap()
        .with_capture_time(capture_time.clone())
        .unwrap();
        let mut updated_player = world.get_player(player.id.as_str()).unwrap().unwrap();
        updated_player.apply_xp(7, capture_time.clone()).unwrap();
        let tx = store.award_xp(&updated_player, &transaction).unwrap();
        assert_eq!(tx.occurred_at, event_time);
        assert_eq!(tx.captured_at, Some(capture_time));
    }
    #[test]
    fn failed_rule_action_rolls_back_root_concept_progress_and_history() {
        let store = Arc::new(store());
        let world = WorldService::new(store.clone(), FrozenClock);
        let player = world.create_player("Ada", None).unwrap();
        let concepts = ConceptService::new(store.clone(), FrozenClock);
        let c = concepts
            .create_concept(player.id.as_str(), "subject", "Python", None)
            .unwrap();
        let rule = RuleCondition::NumberCompare {
            subject: NumericSubject::CurrentProgress,
            comparison: Comparison::Greater,
            value: 10.0,
        };
        world
            .create_rule(
                "invalid follow-up",
                None,
                0,
                EventKind::ConceptProgressChanged,
                rule,
                vec![RuleAction::SetConceptProgress {
                    concept_id: c.id.to_string(),
                    track_code: "confidence".into(),
                    value: 101.0,
                    level: None,
                }],
            )
            .unwrap();
        assert!(concepts
            .set_progress(c.id.as_str(), "progress", 25.0, None, None)
            .is_err());
        assert!(concepts.progress_tracks(c.id.as_str()).unwrap().is_empty());
        assert!(concepts
            .progress_history(c.id.as_str(), None)
            .unwrap()
            .is_empty());
        assert!(world
            .list_rule_executions(100)
            .unwrap()
            .iter()
            .any(|r| r.status == "failed"));
    }
    #[test]
    fn search_query_fails_closed_on_limits_and_sanitizes_fts_operators() {
        let query = SearchQuery {
            text: Some("python OR * (health)".into()),
            ..Default::default()
        };
        assert_eq!(
            query.fts_query().unwrap(),
            "\"python\" AND \"or\" AND \"health\""
        );
        let mut bad = SearchQuery::default();
        bad.limit = 201;
        assert!(bad.validate().is_err());
    }

    #[test]
    fn concepts_progress_snapshots_and_search_survive_database_reopen() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let concept_id;
        {
            let store = Arc::new(SqliteHealthStore::open_file(file.path(), T0));
            let world = WorldService::new(store.clone(), FrozenClock);
            let player = world.create_player("Ada", None).unwrap();
            let concepts = ConceptService::new(store.clone(), FrozenClock);
            let concept = concepts
                .create_concept(player.id.as_str(), "project", "Apollo Project", None)
                .unwrap();
            concept_id = concept.id.clone();
            concepts
                .set_progress(
                    concept.id.as_str(),
                    "experience",
                    350.0,
                    Some(2),
                    Some("2026-09-27T09:00:00Z"),
                )
                .unwrap();
            concepts
                .capture_snapshot(concept.id.as_str(), "2026-09-27")
                .unwrap();
        }
        let reopened = Arc::new(SqliteHealthStore::open_file(file.path(), T0));
        let concept = lr_application::ConceptStore::get_concept(&reopened, &concept_id)
            .unwrap()
            .unwrap();
        assert_eq!(concept.name, "Apollo Project");
        assert_eq!(
            lr_application::ConceptStore::list_concept_progress(&reopened, &concept_id).unwrap()[0]
                .current_value,
            350.0
        );
        assert_eq!(
            lr_application::ConceptStore::list_concept_progress_history(
                &reopened,
                &concept_id,
                None
            )
            .unwrap()
            .len(),
            1
        );
        assert_eq!(
            lr_application::ConceptStore::list_concept_snapshots(&reopened, &concept_id)
                .unwrap()
                .len(),
            1
        );
        let query = SearchQuery {
            text: Some("apollo".into()),
            kind: Some(SearchEntityKind::Concept),
            ..Default::default()
        };
        assert_eq!(
            lr_application::SearchStore::search(
                &reopened,
                &query,
                &Iso8601Timestamp::parse(T0).unwrap()
            )
            .unwrap()
            .len(),
            1
        );
    }

    #[test]
    fn quest_sessions_are_searchable_by_status_time_and_recorded_concept() {
        let store = Arc::new(store());
        let world = WorldService::new(store.clone(), FrozenClock);
        let player = world.create_player("Ada", None).unwrap();
        let concepts = ConceptService::new(store.clone(), FrozenClock);
        let concept = concepts
            .create_concept(player.id.as_str(), "subject", "Reading", None)
            .unwrap();
        let semantics = SemanticsService::new(store.clone(), FrozenClock);
        let session = semantics
            .start_session(
                player.id.as_str(),
                None,
                None,
                None,
                None,
                Some(concept.id.as_str()),
                Some("2026-09-27T09:00:00Z"),
            )
            .unwrap();
        let query = SearchQuery {
            kind: Some(SearchEntityKind::QuestSession),
            player_id: Some(player.id.clone()),
            concept_id: Some(concept.id.clone()),
            status: Some("in_progress".into()),
            from: Some(Iso8601Timestamp::parse("2026-09-27T08:00:00Z").unwrap()),
            active: Some(true),
            ..Default::default()
        };
        let hits = concepts.search(&query).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, session.id.as_str());
        assert_eq!(hits[0].concept_id.as_ref(), Some(&concept.id));

        semantics
            .finish_session(
                session.id.as_str(),
                Some("2026-09-27T10:00:00Z"),
                lr_domain::SessionStatus::Interrupted,
                None,
                None,
            )
            .unwrap();
        let updated = SearchQuery {
            status: Some("interrupted".into()),
            active: None,
            ..query
        };
        let hits = concepts.search(&updated).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, session.id.as_str());
        assert_eq!(hits[0].status.as_deref(), Some("interrupted"));
        assert_eq!(hits[0].active, Some(true));
    }
}
