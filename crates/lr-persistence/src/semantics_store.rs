//! SQLite storage for explicit Phase 3.6 world semantics.
use crate::sqlite_store::SqliteHealthStore;
use lr_application::{SemanticsStore, StorageError};
use lr_domain::*;
use rusqlite::{params, OptionalExtension, Row};
fn op(e: impl std::fmt::Display) -> StorageError {
    StorageError::Operation(e.to_string())
}
fn eid(s: String) -> Result<EntityId, StorageError> {
    EntityId::new(s).map_err(op)
}
fn ts(s: String) -> Result<Iso8601Timestamp, StorageError> {
    Iso8601Timestamp::parse(s).map_err(op)
}
fn opt_id(v: Option<String>) -> Result<Option<EntityId>, StorageError> {
    v.map(eid).transpose()
}
fn opt_ts(v: Option<String>) -> Result<Option<Iso8601Timestamp>, StorageError> {
    v.map(ts).transpose()
}
fn stage(r: &Row<'_>) -> Result<QuestStage, StorageError> {
    Ok(QuestStage {
        id: eid(r.get(0).map_err(op)?)?,
        player_id: eid(r.get(1).map_err(op)?)?,
        quest_id: eid(r.get(2).map_err(op)?)?,
        title: r.get(3).map_err(op)?,
        description: r.get(4).map_err(op)?,
        story: r.get(5).map_err(op)?,
        instructions: r.get(6).map_err(op)?,
        status: StageStatus::parse(&r.get::<_, String>(7).map_err(op)?).map_err(op)?,
        sort_order: r.get(8).map_err(op)?,
        is_active: r.get::<_, i64>(9).map_err(op)? != 0,
        metadata_json: r.get(10).map_err(op)?,
        created_at: ts(r.get(11).map_err(op)?)?,
        updated_at: ts(r.get(12).map_err(op)?)?,
    })
}
fn branch(r: &Row<'_>) -> Result<QuestBranch, StorageError> {
    Ok(QuestBranch {
        id: eid(r.get(0).map_err(op)?)?,
        player_id: eid(r.get(1).map_err(op)?)?,
        quest_id: eid(r.get(2).map_err(op)?)?,
        stage_id: eid(r.get(3).map_err(op)?)?,
        title: r.get(4).map_err(op)?,
        description: r.get(5).map_err(op)?,
        status: BranchStatus::parse(&r.get::<_, String>(6).map_err(op)?).map_err(op)?,
        sort_order: r.get(7).map_err(op)?,
        is_active: r.get::<_, i64>(8).map_err(op)? != 0,
        metadata_json: r.get(9).map_err(op)?,
        created_at: ts(r.get(10).map_err(op)?)?,
        updated_at: ts(r.get(11).map_err(op)?)?,
    })
}
fn session(r: &Row<'_>) -> Result<QuestSession, StorageError> {
    Ok(QuestSession {
        id: eid(r.get(0).map_err(op)?)?,
        player_id: eid(r.get(1).map_err(op)?)?,
        quest_id: opt_id(r.get(2).map_err(op)?)?,
        stage_id: opt_id(r.get(3).map_err(op)?)?,
        branch_id: opt_id(r.get(4).map_err(op)?)?,
        skill_id: opt_id(r.get(5).map_err(op)?)?,
        concept_id: opt_id(r.get(6).map_err(op)?)?,
        started_at: ts(r.get(7).map_err(op)?)?,
        ended_at: opt_ts(r.get(8).map_err(op)?)?,
        status: SessionStatus::parse(&r.get::<_, String>(9).map_err(op)?).map_err(op)?,
        progress_before: r.get(10).map_err(op)?,
        progress_after: r.get(11).map_err(op)?,
        result: r.get(12).map_err(op)?,
        notes: r.get(13).map_err(op)?,
        is_active: r.get::<_, i64>(14).map_err(op)? != 0,
        metadata_json: r.get(15).map_err(op)?,
        created_at: ts(r.get(16).map_err(op)?)?,
        updated_at: ts(r.get(17).map_err(op)?)?,
    })
}
const STAGE_SQL:&str="id,player_id,quest_id,title,description,story,instructions,status,sort_order,is_active,metadata_json,created_at,updated_at";
const BRANCH_SQL:&str="id,player_id,quest_id,stage_id,title,description,status,sort_order,is_active,metadata_json,created_at,updated_at";
const SESSION_SQL:&str="id,player_id,quest_id,stage_id,branch_id,skill_id,concept_id,started_at,ended_at,status,progress_before,progress_after,result,notes,is_active,metadata_json,created_at,updated_at";
fn attachment(r: &Row<'_>) -> Result<ContentAttachment, StorageError> {
    Ok(ContentAttachment {
        content_id: eid(r.get(0).map_err(op)?)?,
        player_id: eid(r.get(1).map_err(op)?)?,
        target_kind: ContentTargetKind::parse(&r.get::<_, String>(2).map_err(op)?).map_err(op)?,
        target_id: eid(r.get(3).map_err(op)?)?,
        role_code: r.get(4).map_err(op)?,
        is_active: r.get::<_, i64>(5).map_err(op)? != 0,
        created_at: ts(r.get(6).map_err(op)?)?,
        updated_at: ts(r.get(7).map_err(op)?)?,
    })
}
fn association(r: &Row<'_>) -> Result<ConceptAssociation, StorageError> {
    Ok(ConceptAssociation {
        id: eid(r.get(0).map_err(op)?)?,
        player_id: eid(r.get(1).map_err(op)?)?,
        concept_id: eid(r.get(2).map_err(op)?)?,
        entity_kind: AssociatedEntityKind::parse(&r.get::<_, String>(3).map_err(op)?)
            .map_err(op)?,
        entity_id: r.get(4).map_err(op)?,
        association_code: r.get(5).map_err(op)?,
        is_active: r.get::<_, i64>(6).map_err(op)? != 0,
        metadata_json: r.get(7).map_err(op)?,
        created_at: ts(r.get(8).map_err(op)?)?,
        updated_at: ts(r.get(9).map_err(op)?)?,
    })
}
fn revision(r: &Row<'_>) -> Result<EntityRevision, StorageError> {
    Ok(EntityRevision {
        id: eid(r.get(0).map_err(op)?)?,
        player_id: eid(r.get(1).map_err(op)?)?,
        target_kind: RevisionTargetKind::parse(&r.get::<_, String>(2).map_err(op)?).map_err(op)?,
        target_id: eid(r.get(3).map_err(op)?)?,
        revision_number: r.get(4).map_err(op)?,
        recorded_at: ts(r.get(5).map_err(op)?)?,
        author_player_id: opt_id(r.get(6).map_err(op)?)?,
        reason: r.get(7).map_err(op)?,
        snapshot_json: r.get(8).map_err(op)?,
        metadata_json: r.get(9).map_err(op)?,
    })
}
fn preference(r: &Row<'_>) -> Result<PresentationPreference, StorageError> {
    Ok(PresentationPreference {
        player_id: eid(r.get(0).map_err(op)?)?,
        entity_kind: r.get(1).map_err(op)?,
        entity_id: eid(r.get(2).map_err(op)?)?,
        context: r.get(3).map_err(op)?,
        is_visible: r.get::<_, i64>(4).map_err(op)? != 0,
        sort_order: r.get(5).map_err(op)?,
        is_pinned: r.get::<_, i64>(6).map_err(op)? != 0,
        is_collapsed: r.get::<_, Option<i64>>(7).map_err(op)?.map(|x| x != 0),
        variant: r.get(8).map_err(op)?,
        density: r.get(9).map_err(op)?,
        metadata_json: r.get(10).map_err(op)?,
        created_at: ts(r.get(11).map_err(op)?)?,
        updated_at: ts(r.get(12).map_err(op)?)?,
    })
}
fn suggestion(r: &Row<'_>) -> Result<ProgressSuggestion, StorageError> {
    Ok(ProgressSuggestion {
        id: eid(r.get(0).map_err(op)?)?,
        player_id: eid(r.get(1).map_err(op)?)?,
        concept_id: eid(r.get(2).map_err(op)?)?,
        track_code: r.get(3).map_err(op)?,
        proposed_value: r.get(4).map_err(op)?,
        proposed_level: r.get(5).map_err(op)?,
        reason: r.get(6).map_err(op)?,
        source: r.get(7).map_err(op)?,
        status: SuggestionStatus::parse(&r.get::<_, String>(8).map_err(op)?).map_err(op)?,
        created_at: ts(r.get(9).map_err(op)?)?,
        resolved_at: opt_ts(r.get(10).map_err(op)?)?,
        metadata_json: r.get(11).map_err(op)?,
    })
}
impl SemanticsStore for SqliteHealthStore {
    fn insert_stage(&self, v: &QuestStage) -> Result<(), StorageError> {
        self.with_conn(|d|{d.execute("INSERT INTO quest_stages(id,player_id,quest_id,title,description,story,instructions,status,sort_order,is_active,metadata_json,created_at,updated_at)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![v.id.as_str(),v.player_id.as_str(),v.quest_id.as_str(),v.title,v.description,v.story,v.instructions,v.status.as_str(),v.sort_order,v.is_active as i64,v.metadata_json,v.created_at.as_str(),v.updated_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn get_stage(&self, id: &EntityId) -> Result<Option<QuestStage>, StorageError> {
        self.with_conn(|d| {
            let sql = format!("SELECT {STAGE_SQL} FROM quest_stages WHERE id=?1");
            let mut s = d.prepare(&sql).map_err(op)?;
            let mut r = s.query([id.as_str()]).map_err(op)?;
            match r.next().map_err(op)? {
                Some(v) => Ok(Some(stage(v)?)),
                None => Ok(None),
            }
        })
    }
    fn list_stages(&self, q: &EntityId) -> Result<Vec<QuestStage>, StorageError> {
        self.with_conn(|d| {
            let sql = format!(
                "SELECT {STAGE_SQL} FROM quest_stages WHERE quest_id=?1 ORDER BY sort_order,id"
            );
            let mut s = d.prepare(&sql).map_err(op)?;
            let mut rows = s.query([q.as_str()]).map_err(op)?;
            let mut out = vec![];
            while let Some(r) = rows.next().map_err(op)? {
                out.push(stage(r)?)
            }
            Ok(out)
        })
    }
    fn insert_branch(&self, v: &QuestBranch) -> Result<(), StorageError> {
        self.with_conn(|d|{d.execute("INSERT INTO quest_branches(id,player_id,quest_id,stage_id,title,description,status,sort_order,is_active,metadata_json,created_at,updated_at)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![v.id.as_str(),v.player_id.as_str(),v.quest_id.as_str(),v.stage_id.as_str(),v.title,v.description,v.status.as_str(),v.sort_order,v.is_active as i64,v.metadata_json,v.created_at.as_str(),v.updated_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn list_branches(&self, stage_id: &EntityId) -> Result<Vec<QuestBranch>, StorageError> {
        self.with_conn(|d| {
            let sql = format!(
                "SELECT {BRANCH_SQL} FROM quest_branches WHERE stage_id=?1 ORDER BY sort_order,id"
            );
            let mut s = d.prepare(&sql).map_err(op)?;
            let mut rows = s.query([stage_id.as_str()]).map_err(op)?;
            let mut out = vec![];
            while let Some(r) = rows.next().map_err(op)? {
                out.push(branch(r)?)
            }
            Ok(out)
        })
    }
    fn insert_session(&self, v: &QuestSession) -> Result<(), StorageError> {
        self.with_conn(|d|{d.execute("INSERT INTO quest_sessions(id,player_id,quest_id,stage_id,branch_id,skill_id,concept_id,started_at,ended_at,status,progress_before,progress_after,result,notes,is_active,metadata_json,created_at,updated_at)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",params![v.id.as_str(),v.player_id.as_str(),v.quest_id.as_ref().map(EntityId::as_str),v.stage_id.as_ref().map(EntityId::as_str),v.branch_id.as_ref().map(EntityId::as_str),v.skill_id.as_ref().map(EntityId::as_str),v.concept_id.as_ref().map(EntityId::as_str),v.started_at.as_str(),v.ended_at.as_ref().map(Iso8601Timestamp::as_str),v.status.as_str(),v.progress_before,v.progress_after,v.result,v.notes,v.is_active as i64,v.metadata_json,v.created_at.as_str(),v.updated_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn get_session(&self, id: &EntityId) -> Result<Option<QuestSession>, StorageError> {
        self.with_conn(|d| {
            let sql = format!("SELECT {SESSION_SQL} FROM quest_sessions WHERE id=?1");
            let mut s = d.prepare(&sql).map_err(op)?;
            let mut rows = s.query([id.as_str()]).map_err(op)?;
            match rows.next().map_err(op)? {
                Some(r) => Ok(Some(session(r)?)),
                None => Ok(None),
            }
        })
    }
    fn update_session(&self, v: &QuestSession) -> Result<(), StorageError> {
        self.with_conn(|d|{let n=d.execute("UPDATE quest_sessions SET ended_at=?2,status=?3,progress_before=?4,progress_after=?5,result=?6,notes=?7,is_active=?8,metadata_json=?9,updated_at=?10 WHERE id=?1",params![v.id.as_str(),v.ended_at.as_ref().map(Iso8601Timestamp::as_str),v.status.as_str(),v.progress_before,v.progress_after,v.result,v.notes,v.is_active as i64,v.metadata_json,v.updated_at.as_str()]).map_err(op)?;if n==1{Ok(())}else{Err(op("Quest Session not found"))}})
    }
    fn list_sessions(
        &self,
        p: &EntityId,
        q: Option<&EntityId>,
        st: Option<&EntityId>,
    ) -> Result<Vec<QuestSession>, StorageError> {
        self.with_conn(|d|{let sql=format!("SELECT {SESSION_SQL} FROM quest_sessions WHERE player_id=?1 AND (?2 IS NULL OR quest_id=?2) AND (?3 IS NULL OR stage_id=?3) ORDER BY started_at,id");let mut s=d.prepare(&sql).map_err(op)?;let mut rows=s.query(params![p.as_str(),q.map(EntityId::as_str),st.map(EntityId::as_str)]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(session(r)?)}Ok(out)})
    }
    fn attach_content(&self, v: &ContentAttachment) -> Result<(), StorageError> {
        self.with_conn(|d|{d.execute("INSERT INTO content_attachments(content_id,player_id,target_kind,target_id,role_code,is_active,created_at,updated_at)VALUES(?1,?2,?3,?4,?5,?6,?7,?8)ON CONFLICT(content_id,target_kind,target_id,role_code)DO UPDATE SET is_active=excluded.is_active,updated_at=excluded.updated_at",params![v.content_id.as_str(),v.player_id.as_str(),v.target_kind.as_str(),v.target_id.as_str(),v.role_code,v.is_active as i64,v.created_at.as_str(),v.updated_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn list_content_attachments(
        &self,
        k: ContentTargetKind,
        id: &EntityId,
    ) -> Result<Vec<ContentAttachment>, StorageError> {
        self.with_conn(|d|{let mut s=d.prepare("SELECT content_id,player_id,target_kind,target_id,role_code,is_active,created_at,updated_at FROM content_attachments WHERE target_kind=?1 AND target_id=?2 ORDER BY role_code,created_at").map_err(op)?;let mut rows=s.query(params![k.as_str(),id.as_str()]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(attachment(r)?)}Ok(out)})
    }
    fn insert_association(&self, v: &ConceptAssociation) -> Result<(), StorageError> {
        self.with_conn(|d|{d.execute("INSERT INTO concept_associations(id,player_id,concept_id,entity_kind,entity_id,association_code,is_active,metadata_json,created_at,updated_at)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![v.id.as_str(),v.player_id.as_str(),v.concept_id.as_str(),v.entity_kind.as_str(),v.entity_id,v.association_code,v.is_active as i64,v.metadata_json,v.created_at.as_str(),v.updated_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn get_association(&self, id: &EntityId) -> Result<Option<ConceptAssociation>, StorageError> {
        self.with_conn(|d|{let mut s=d.prepare("SELECT id,player_id,concept_id,entity_kind,entity_id,association_code,is_active,metadata_json,created_at,updated_at FROM concept_associations WHERE id=?1").map_err(op)?;let mut rows=s.query([id.as_str()]).map_err(op)?;match rows.next().map_err(op)?{Some(r)=>Ok(Some(association(r)?)),None=>Ok(None)}})
    }
    fn update_association(&self, v: &ConceptAssociation) -> Result<(), StorageError> {
        self.with_conn(|d|{let n=d.execute("UPDATE concept_associations SET is_active=?2,metadata_json=?3,updated_at=?4 WHERE id=?1",params![v.id.as_str(),v.is_active as i64,v.metadata_json,v.updated_at.as_str()]).map_err(op)?;if n==1{Ok(())}else{Err(op("Concept association not found"))}})
    }
    fn list_associations(
        &self,
        c: &EntityId,
        k: Option<AssociatedEntityKind>,
        e: Option<&str>,
    ) -> Result<Vec<ConceptAssociation>, StorageError> {
        self.with_conn(|d|{let mut s=d.prepare("SELECT id,player_id,concept_id,entity_kind,entity_id,association_code,is_active,metadata_json,created_at,updated_at FROM concept_associations WHERE concept_id=?1 AND (?2 IS NULL OR entity_kind=?2) AND (?3 IS NULL OR entity_id=?3) ORDER BY entity_kind,association_code,created_at").map_err(op)?;let mut rows=s.query(params![c.as_str(),k.map(AssociatedEntityKind::as_str),e]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(association(r)?)}Ok(out)})
    }
    fn list_associations_for_entity(
        &self,
        k: AssociatedEntityKind,
        id: &str,
    ) -> Result<Vec<ConceptAssociation>, StorageError> {
        self.with_conn(|d|{let mut s=d.prepare("SELECT id,player_id,concept_id,entity_kind,entity_id,association_code,is_active,metadata_json,created_at,updated_at FROM concept_associations WHERE entity_kind=?1 AND entity_id=?2 ORDER BY concept_id,association_code").map_err(op)?;let mut rows=s.query(params![k.as_str(),id]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(association(r)?)}Ok(out)})
    }
    fn append_revision(&self, v: &EntityRevision) -> Result<(), StorageError> {
        self.with_conn(|d|{d.execute("INSERT INTO entity_revisions(id,player_id,target_kind,target_id,revision_number,recorded_at,author_player_id,reason,snapshot_json,metadata_json)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![v.id.as_str(),v.player_id.as_str(),v.target_kind.as_str(),v.target_id.as_str(),v.revision_number,v.recorded_at.as_str(),v.author_player_id.as_ref().map(EntityId::as_str),v.reason,v.snapshot_json,v.metadata_json]).map_err(op)?;Ok(())})
    }
    fn list_revisions(
        &self,
        k: RevisionTargetKind,
        id: &EntityId,
    ) -> Result<Vec<EntityRevision>, StorageError> {
        self.with_conn(|d|{let mut s=d.prepare("SELECT id,player_id,target_kind,target_id,revision_number,recorded_at,author_player_id,reason,snapshot_json,metadata_json FROM entity_revisions WHERE target_kind=?1 AND target_id=?2 ORDER BY revision_number").map_err(op)?;let mut rows=s.query(params![k.as_str(),id.as_str()]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(revision(r)?)}Ok(out)})
    }
    fn get_revision(&self, id: &EntityId) -> Result<Option<EntityRevision>, StorageError> {
        self.with_conn(|d|{let mut s=d.prepare("SELECT id,player_id,target_kind,target_id,revision_number,recorded_at,author_player_id,reason,snapshot_json,metadata_json FROM entity_revisions WHERE id=?1").map_err(op)?;let mut rows=s.query([id.as_str()]).map_err(op)?;match rows.next().map_err(op)?{Some(r)=>Ok(Some(revision(r)?)),None=>Ok(None)}})
    }
    fn restore_revision(
        &self,
        id: &EntityId,
        now: &Iso8601Timestamp,
        reason: Option<&str>,
    ) -> Result<EntityRevision, StorageError> {
        self.with_conn_mut(|db|{let tx=db.transaction().map_err(op)?;let rev={let mut s=tx.prepare("SELECT id,player_id,target_kind,target_id,revision_number,recorded_at,author_player_id,reason,snapshot_json,metadata_json FROM entity_revisions WHERE id=?1").map_err(op)?;let mut rows=s.query([id.as_str()]).map_err(op)?;match rows.next().map_err(op)?{Some(r)=>revision(r)?,None=>return Err(op("revision not found"))}};let j=rev.snapshot_json.as_str();let at=now.as_str();let target=rev.target_id.as_str();let result=match rev.target_kind{
RevisionTargetKind::Player=>tx.execute("UPDATE players SET name=json_extract(?2,'$.name'),description=json_extract(?2,'$.description'),level=json_extract(?2,'$.level'),level_name=json_extract(?2,'$.level_name'),progression_label=json_extract(?2,'$.progression_label'),current_xp=json_extract(?2,'$.current_xp'),is_active=json_extract(?2,'$.is_active'),metadata_json=json_extract(?2,'$.metadata_json'),updated_at=?3 WHERE id=?1",params![target,j,at]),
RevisionTargetKind::Quest=>tx.execute("UPDATE quests SET parent_quest_id=json_extract(?2,'$.parent_quest_id'),skill_id=json_extract(?2,'$.skill_id'),title=json_extract(?2,'$.title'),description=json_extract(?2,'$.description'),story=json_extract(?2,'$.story'),instructions=json_extract(?2,'$.instructions'),status=json_extract(?2,'$.status'),difficulty=json_extract(?2,'$.difficulty'),progress=json_extract(?2,'$.progress'),xp_reward=json_extract(?2,'$.xp_reward'),due_at=json_extract(?2,'$.due_at'),started_at=json_extract(?2,'$.started_at'),completed_at=json_extract(?2,'$.completed_at'),metadata_json=json_extract(?2,'$.metadata_json'),updated_at=?3 WHERE id=?1",params![target,j,at]),
RevisionTargetKind::SkillTree=>tx.execute("UPDATE skill_trees SET name=json_extract(?2,'$.name'),description=json_extract(?2,'$.description'),story=json_extract(?2,'$.story'),instructions=json_extract(?2,'$.instructions'),is_active=json_extract(?2,'$.is_active'),metadata_json=json_extract(?2,'$.metadata_json'),updated_at=?3 WHERE id=?1",params![target,j,at]),
RevisionTargetKind::Skill=>tx.execute("UPDATE skills SET parent_skill_id=json_extract(?2,'$.parent_skill_id'),name=json_extract(?2,'$.name'),description=json_extract(?2,'$.description'),story=json_extract(?2,'$.story'),instructions=json_extract(?2,'$.instructions'),level=json_extract(?2,'$.level'),level_name=json_extract(?2,'$.level_name'),progression_label=json_extract(?2,'$.progression_label'),current_xp=json_extract(?2,'$.current_xp'),invested_minutes=json_extract(?2,'$.invested_minutes'),status=json_extract(?2,'$.status'),started_at=json_extract(?2,'$.started_at'),completed_at=json_extract(?2,'$.completed_at'),metadata_json=json_extract(?2,'$.metadata_json'),updated_at=?3 WHERE id=?1",params![target,j,at]),
RevisionTargetKind::Concept=>tx.execute("UPDATE concepts SET name=json_extract(?2,'$.name'),description=json_extract(?2,'$.description'),is_active=json_extract(?2,'$.is_active'),metadata_json=json_extract(?2,'$.metadata_json'),updated_at=?3 WHERE id=?1",params![target,j,at]),
RevisionTargetKind::QuestStage=>tx.execute("UPDATE quest_stages SET title=json_extract(?2,'$.title'),description=json_extract(?2,'$.description'),story=json_extract(?2,'$.story'),instructions=json_extract(?2,'$.instructions'),status=json_extract(?2,'$.status'),sort_order=json_extract(?2,'$.sort_order'),is_active=json_extract(?2,'$.is_active'),metadata_json=json_extract(?2,'$.metadata_json'),updated_at=?3 WHERE id=?1",params![target,j,at]),
RevisionTargetKind::QuestBranch=>tx.execute("UPDATE quest_branches SET title=json_extract(?2,'$.title'),description=json_extract(?2,'$.description'),status=json_extract(?2,'$.status'),sort_order=json_extract(?2,'$.sort_order'),is_active=json_extract(?2,'$.is_active'),metadata_json=json_extract(?2,'$.metadata_json'),updated_at=?3 WHERE id=?1",params![target,j,at]),
RevisionTargetKind::QuestSession=>tx.execute("UPDATE quest_sessions SET ended_at=json_extract(?2,'$.ended_at'),status=json_extract(?2,'$.status'),progress_before=json_extract(?2,'$.progress_before'),progress_after=json_extract(?2,'$.progress_after'),result=json_extract(?2,'$.result'),notes=json_extract(?2,'$.notes'),is_active=json_extract(?2,'$.is_active'),metadata_json=json_extract(?2,'$.metadata_json'),updated_at=?3 WHERE id=?1",params![target,j,at]),
RevisionTargetKind::NarrativeEntry=>tx.execute("UPDATE narrative_entries SET kind_code=json_extract(?2,'$.kind_code'),title=json_extract(?2,'$.title'),content=json_extract(?2,'$.content'),author=json_extract(?2,'$.author'),source_kind=json_extract(?2,'$.source_kind'),source_id=json_extract(?2,'$.source_id'),is_active=json_extract(?2,'$.is_active'),metadata_json=json_extract(?2,'$.metadata_json'),updated_at=?3 WHERE id=?1",params![target,j,at]),
RevisionTargetKind::ConceptProgress=>tx.execute("UPDATE concept_progress_tracks SET current_value=json_extract(?2,'$.current_value'),level=json_extract(?2,'$.level'),level_name=json_extract(?2,'$.level_name'),progression_label=json_extract(?2,'$.progression_label'),is_active=json_extract(?2,'$.is_active'),metadata_json=json_extract(?2,'$.metadata_json'),updated_at=?3 WHERE id=?1",params![target,j,at]),};let changed=result.map_err(op)?;if changed!=1{return Err(op("revision target no longer exists"));}tx.execute("INSERT INTO entity_revision_restores(id,revision_id,player_id,restored_at,reason)VALUES(lower(hex(randomblob(16))),?1,?2,?3,?4)",params![id.as_str(),rev.player_id.as_str(),at,reason]).map_err(op)?;tx.commit().map_err(op)?;Ok(rev)})
    }
    fn set_lifecycle(
        &self,
        k: RevisionTargetKind,
        id: &EntityId,
        p: &EntityId,
        state: LifecycleState,
        occurred: &Iso8601Timestamp,
        captured: &Iso8601Timestamp,
        reason: Option<&str>,
    ) -> Result<(), StorageError> {
        if occurred > captured {
            return Err(op("lifecycle capture cannot precede occurrence"));
        }
        self.with_conn_mut(|db|{let tx=db.transaction().map_err(op)?;let exists:i64=match k{RevisionTargetKind::Player=>tx.query_row("SELECT count(*) FROM players WHERE id=?1 AND id=?2",params![id.as_str(),p.as_str()],|r|r.get(0)),RevisionTargetKind::Quest=>tx.query_row("SELECT count(*) FROM quests WHERE id=?1 AND player_id=?2",params![id.as_str(),p.as_str()],|r|r.get(0)),RevisionTargetKind::SkillTree=>tx.query_row("SELECT count(*) FROM skill_trees WHERE id=?1 AND player_id=?2",params![id.as_str(),p.as_str()],|r|r.get(0)),RevisionTargetKind::Skill=>tx.query_row("SELECT count(*) FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE s.id=?1 AND t.player_id=?2",params![id.as_str(),p.as_str()],|r|r.get(0)),RevisionTargetKind::Concept=>tx.query_row("SELECT count(*) FROM concepts WHERE id=?1 AND player_id=?2",params![id.as_str(),p.as_str()],|r|r.get(0)),RevisionTargetKind::QuestStage=>tx.query_row("SELECT count(*) FROM quest_stages WHERE id=?1 AND player_id=?2",params![id.as_str(),p.as_str()],|r|r.get(0)),RevisionTargetKind::QuestBranch=>tx.query_row("SELECT count(*) FROM quest_branches WHERE id=?1 AND player_id=?2",params![id.as_str(),p.as_str()],|r|r.get(0)),RevisionTargetKind::QuestSession=>tx.query_row("SELECT count(*) FROM quest_sessions WHERE id=?1 AND player_id=?2",params![id.as_str(),p.as_str()],|r|r.get(0)),RevisionTargetKind::NarrativeEntry=>tx.query_row("SELECT count(*) FROM narrative_entries WHERE id=?1 AND player_id=?2",params![id.as_str(),p.as_str()],|r|r.get(0)),RevisionTargetKind::ConceptProgress=>tx.query_row("SELECT count(*) FROM concept_progress_tracks t JOIN concepts c ON c.id=t.concept_id WHERE t.id=?1 AND c.player_id=?2",params![id.as_str(),p.as_str()],|r|r.get(0))}.map_err(op)?;if exists!=1{return Err(op("lifecycle target not found in Player world"));}let previous:Option<String>=tx.query_row("SELECT state FROM entity_lifecycle WHERE target_kind=?1 AND target_id=?2",params![k.as_str(),id.as_str()],|r|r.get(0)).optional().map_err(op)?;if previous.as_deref()==Some(state.as_str()){tx.commit().map_err(op)?;return Ok(());}tx.execute("INSERT INTO entity_lifecycle(target_kind,target_id,player_id,state,updated_at)VALUES(?1,?2,?3,?4,?5)ON CONFLICT(target_kind,target_id)DO UPDATE SET state=excluded.state,updated_at=excluded.updated_at",params![k.as_str(),id.as_str(),p.as_str(),state.as_str(),captured.as_str()]).map_err(op)?;tx.execute("INSERT INTO entity_lifecycle_history(id,target_kind,target_id,player_id,previous_state,current_state,occurred_at,captured_at,reason)VALUES(lower(hex(randomblob(16))),?1,?2,?3,?4,?5,?6,?7,?8)",params![k.as_str(),id.as_str(),p.as_str(),previous,state.as_str(),occurred.as_str(),captured.as_str(),reason]).map_err(op)?;tx.commit().map_err(op)?;Ok(())})
    }
    fn get_lifecycle(
        &self,
        k: RevisionTargetKind,
        id: &EntityId,
    ) -> Result<LifecycleState, StorageError> {
        self.with_conn(|d| {
            let raw: Option<String> = d
                .query_row(
                    "SELECT state FROM entity_lifecycle WHERE target_kind=?1 AND target_id=?2",
                    params![k.as_str(), id.as_str()],
                    |r| r.get(0),
                )
                .optional()
                .map_err(op)?;
            LifecycleState::parse(raw.as_deref().unwrap_or("active")).map_err(op)
        })
    }
    fn set_presentation(&self, v: &PresentationPreference) -> Result<(), StorageError> {
        self.with_conn(|d|{d.execute("INSERT INTO presentation_preferences(player_id,entity_kind,entity_id,context,is_visible,sort_order,is_pinned,is_collapsed,variant,density,metadata_json,created_at,updated_at)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)ON CONFLICT(player_id,entity_kind,entity_id,context)DO UPDATE SET is_visible=excluded.is_visible,sort_order=excluded.sort_order,is_pinned=excluded.is_pinned,is_collapsed=excluded.is_collapsed,variant=excluded.variant,density=excluded.density,metadata_json=excluded.metadata_json,updated_at=excluded.updated_at",params![v.player_id.as_str(),v.entity_kind,v.entity_id.as_str(),v.context,v.is_visible as i64,v.sort_order,v.is_pinned as i64,v.is_collapsed.map(i64::from),v.variant,v.density,v.metadata_json,v.created_at.as_str(),v.updated_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn list_presentation(
        &self,
        p: &EntityId,
        c: &str,
    ) -> Result<Vec<PresentationPreference>, StorageError> {
        self.with_conn(|d|{let mut s=d.prepare("SELECT player_id,entity_kind,entity_id,context,is_visible,sort_order,is_pinned,is_collapsed,variant,density,metadata_json,created_at,updated_at FROM presentation_preferences WHERE player_id=?1 AND context=?2 ORDER BY is_pinned DESC,sort_order,entity_id").map_err(op)?;let mut rows=s.query(params![p.as_str(),c]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(preference(r)?)}Ok(out)})
    }
    fn insert_suggestion(&self, v: &ProgressSuggestion) -> Result<(), StorageError> {
        self.with_conn(|d|{d.execute("INSERT INTO progress_suggestions(id,player_id,concept_id,track_code,proposed_value,proposed_level,reason,source,status,created_at,resolved_at,metadata_json)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![v.id.as_str(),v.player_id.as_str(),v.concept_id.as_str(),v.track_code,v.proposed_value,v.proposed_level,v.reason,v.source,v.status.as_str(),v.created_at.as_str(),v.resolved_at.as_ref().map(Iso8601Timestamp::as_str),v.metadata_json]).map_err(op)?;Ok(())})
    }
    fn get_suggestion(&self, id: &EntityId) -> Result<Option<ProgressSuggestion>, StorageError> {
        self.with_conn(|d| {
            let mut s=d.prepare("SELECT id,player_id,concept_id,track_code,proposed_value,proposed_level,reason,source,status,created_at,resolved_at,metadata_json FROM progress_suggestions WHERE id=?1").map_err(op)?;
            let mut rows=s.query([id.as_str()]).map_err(op)?;
            match rows.next().map_err(op)? { Some(r)=>Ok(Some(suggestion(r)?)),None=>Ok(None) }
        })
    }
    fn list_suggestions(
        &self,
        c: &EntityId,
        include: bool,
    ) -> Result<Vec<ProgressSuggestion>, StorageError> {
        self.with_conn(|d|{let mut s=d.prepare("SELECT id,player_id,concept_id,track_code,proposed_value,proposed_level,reason,source,status,created_at,resolved_at,metadata_json FROM progress_suggestions WHERE concept_id=?1 AND (?2=1 OR status='pending') ORDER BY created_at,id").map_err(op)?;let mut rows=s.query(params![c.as_str(),include as i64]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(suggestion(r)?)}Ok(out)})
    }
    fn resolve_suggestion(
        &self,
        id: &EntityId,
        p: &EntityId,
        accepted: bool,
        at: &Iso8601Timestamp,
    ) -> Result<ProgressSuggestion, StorageError> {
        self.with_conn_mut(|db|{let tx=db.transaction().map_err(op)?;let current={let mut s=tx.prepare("SELECT id,player_id,concept_id,track_code,proposed_value,proposed_level,reason,source,status,created_at,resolved_at,metadata_json FROM progress_suggestions WHERE id=?1 AND player_id=?2").map_err(op)?;let mut rows=s.query(params![id.as_str(),p.as_str()]).map_err(op)?;match rows.next().map_err(op)?{Some(r)=>suggestion(r)?,None=>return Err(op("suggestion not found"))}};if current.status!=SuggestionStatus::Pending{return Err(op("suggestion is already resolved"));}if accepted{let ok:i64=tx.query_row("SELECT count(*) FROM concept_progress_history WHERE concept_id=?1 AND track_code=?2 AND current_value=?3 AND occurred_at>=?4",params![current.concept_id.as_str(),current.track_code,current.proposed_value,current.created_at.as_str()],|r|r.get(0)).map_err(op)?;if ok==0{return Err(op("suggestion cannot be accepted until an explicit progress change is recorded"));}}tx.execute("UPDATE progress_suggestions SET status=?2,resolved_at=?3 WHERE id=?1",params![id.as_str(),if accepted{"accepted"}else{"rejected"},at.as_str()]).map_err(op)?;tx.commit().map_err(op)?;let mut out=current;out.status=if accepted{SuggestionStatus::Accepted}else{SuggestionStatus::Rejected};out.resolved_at=Some(at.clone());Ok(out)})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lr_application::{
        Clock, ConceptService, EventKind, RuleAction, RuleCondition, SearchEntityKind, SearchSort,
        SemanticsService, TextComparison, TextSubject, WorldService, WorldStore,
    };
    use std::sync::Arc;
    const T0: &str = "2026-09-27T10:00:00Z";
    struct Frozen;
    impl Clock for Frozen {
        fn now_rfc3339(&self) -> String {
            T0.into()
        }
        fn now_unix_nanos(&self) -> u128 {
            77
        }
    }
    fn store() -> Arc<SqliteHealthStore> {
        Arc::new(SqliteHealthStore::open_in_memory(T0))
    }
    #[test]
    fn quest_activity_content_associations_visibility_and_recovery_are_persisted() {
        let store = store();
        let world = WorldService::new(store.clone(), Frozen);
        let semantics = SemanticsService::new(store.clone(), Frozen);
        let concepts = ConceptService::new(store.clone(), Frozen);
        let player = world.create_player("Ada", None).unwrap();
        let quest = world
            .create_quest(
                player.id.as_str(),
                "main",
                "Write a field guide",
                None,
                None,
                None,
                Some(0),
            )
            .unwrap();
        let stage = semantics
            .create_stage(player.id.as_str(), quest.id.as_str(), "Research", 0)
            .unwrap();
        let branch = semantics
            .create_branch(stage.id.as_str(), "Read primary sources", 0)
            .unwrap();
        let session1 = semantics
            .start_session(
                player.id.as_str(),
                Some(quest.id.as_str()),
                Some(stage.id.as_str()),
                Some(branch.id.as_str()),
                None,
                None,
                Some("2026-09-27T09:00:00Z"),
            )
            .unwrap();
        let session2 = semantics
            .start_session(
                player.id.as_str(),
                Some(quest.id.as_str()),
                Some(stage.id.as_str()),
                None,
                None,
                None,
                Some("2026-09-27T09:30:00Z"),
            )
            .unwrap();
        assert_ne!(session1.id, session2.id);
        assert_eq!(
            semantics
                .list_sessions(
                    player.id.as_str(),
                    Some(quest.id.as_str()),
                    Some(stage.id.as_str())
                )
                .unwrap()
                .len(),
            2
        );
        let finished = semantics
            .finish_session(
                session1.id.as_str(),
                Some("2026-09-27T09:20:00Z"),
                SessionStatus::Completed,
                Some("Read two sources".into()),
                None,
            )
            .unwrap();
        assert_eq!(
            finished.ended_at.as_ref().unwrap().as_str(),
            "2026-09-27T09:20:00+00:00"
        );
        let content = world
            .write_narrative(
                player.id.as_str(),
                "briefing",
                "Quest briefing",
                "Review primary sources before writing.",
            )
            .unwrap();
        semantics
            .attach_content(
                player.id.as_str(),
                content.id.as_str(),
                ContentTargetKind::Session,
                session1.id.as_str(),
                "reading",
            )
            .unwrap();
        assert_eq!(
            semantics
                .content_for(ContentTargetKind::Session, session1.id.as_str())
                .unwrap()
                .len(),
            1
        );
        let concept = concepts
            .create_concept(player.id.as_str(), "subject", "Research", None)
            .unwrap();
        semantics
            .associate(
                concept.id.as_str(),
                AssociatedEntityKind::Session,
                session1.id.as_str(),
                "related_to",
            )
            .unwrap();
        let concept_detail = concepts.detail(concept.id.as_str()).unwrap().unwrap();
        assert_eq!(concept_detail.associations.len(), 1);
        assert!(concept_detail
            .related_search_results
            .iter()
            .any(|h| h.kind == lr_application::SearchEntityKind::QuestSession
                && h.id == session1.id.as_str()));
        let quest_detail = semantics.quest_detail(quest.id.as_str()).unwrap().unwrap();
        assert_eq!(quest_detail.activity.stages.len(), 1);
        assert_eq!(quest_detail.activity.branches.len(), 1);
        assert_eq!(quest_detail.activity.sessions.len(), 2);
        assert_eq!(quest_detail.content_entries.len(), 1);
        assert_eq!(quest_detail.concept_associations.len(), 1);
        assert_eq!(
            semantics
                .associations(
                    concept.id.as_str(),
                    Some(AssociatedEntityKind::Session),
                    Some(session1.id.as_str())
                )
                .unwrap()
                .len(),
            1
        );
        let mut query = lr_application::SearchQuery::default();
        query.kind = Some(SearchEntityKind::QuestSession);
        query.concept_id = Some(concept.id.clone());
        query.sort = SearchSort::Newest;
        assert_eq!(semantics.search(&query).unwrap().len(), 1);
        semantics
            .set_presentation(
                player.id.as_str(),
                "quest_session",
                session1.id.as_str(),
                "dashboard",
                false,
                4,
                true,
                Some(false),
            )
            .unwrap();
        query.context = Some("dashboard".into());
        assert!(semantics.search(&query).unwrap().is_empty());
        query.include_hidden = true;
        assert_eq!(semantics.search(&query).unwrap().len(), 1);
        semantics
            .set_lifecycle(
                RevisionTargetKind::QuestSession,
                session1.id.as_str(),
                player.id.as_str(),
                LifecycleState::Trashed,
                None,
            )
            .unwrap();
        assert!(semantics.search(&query).unwrap().is_empty());
        query.include_trashed = true;
        assert_eq!(semantics.search(&query).unwrap().len(), 1);
        semantics
            .set_lifecycle(
                RevisionTargetKind::QuestSession,
                session1.id.as_str(),
                player.id.as_str(),
                LifecycleState::Active,
                Some("restored"),
            )
            .unwrap();
        assert_eq!(
            semantics
                .lifecycle(RevisionTargetKind::QuestSession, session1.id.as_str())
                .unwrap(),
            LifecycleState::Active
        );
        let mut changed = quest.clone();
        changed.title = "Edited title".into();
        changed.updated_at = Iso8601Timestamp::parse("2026-09-27T10:00:00Z").unwrap();
        store.update_quest(&changed).unwrap();
        let revision = semantics
            .revisions(RevisionTargetKind::Quest, quest.id.as_str())
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        assert!(revision.snapshot_json.contains("Write a field guide"));
        semantics
            .restore_revision(revision.id.as_str(), Some("undo edit"))
            .unwrap();
        assert_eq!(
            store.get_quest(&quest.id).unwrap().unwrap().title,
            "Write a field guide"
        );
        let restores: i64 = store
            .with_conn(|db| {
                db.query_row(
                    "SELECT count(*) FROM entity_revision_restores WHERE revision_id=?1",
                    [revision.id.as_str()],
                    |r| r.get::<_, i64>(0),
                )
                .map_err(op)
            })
            .unwrap();
        assert_eq!(restores, 1);
        let states:i64=store.with_conn(|db|db.query_row("SELECT count(*) FROM entity_lifecycle_history WHERE target_kind='quest_session' AND target_id=?1",[session1.id.as_str()],|r|r.get::<_,i64>(0)).map_err(op)).unwrap();
        assert_eq!(states, 2);
    }
    #[test]
    fn manual_progress_is_protected_and_suggestions_commit_with_acceptance() {
        let store = store();
        let world = WorldService::new(store.clone(), Frozen);
        let semantics = SemanticsService::new(store.clone(), Frozen);
        let concepts = ConceptService::new(store.clone(), Frozen);
        let player = world.create_player("Ada", None).unwrap();
        let concept = concepts
            .create_concept(player.id.as_str(), "subject", "Python", None)
            .unwrap();
        concepts
            .set_progress(concept.id.as_str(), "progress", 10.0, None, None)
            .unwrap();
        let rule = world
            .create_rule(
                "unauthorized overwrite",
                None,
                0,
                EventKind::ConceptProgressChanged,
                RuleCondition::TextCompare {
                    subject: TextSubject::ConceptTrackCode,
                    comparison: TextComparison::Equal,
                    value: "progress".into(),
                },
                vec![RuleAction::SetConceptProgress {
                    concept_id: concept.id.to_string(),
                    track_code: "progress".into(),
                    value: 99.0,
                    level: None,
                }],
            )
            .unwrap();
        assert!(concepts
            .set_progress(concept.id.as_str(), "progress", 20.0, None, None)
            .is_err());
        assert_eq!(
            concepts.progress_tracks(concept.id.as_str()).unwrap()[0].current_value,
            10.0
        );
        let suggestion = semantics
            .suggest_progress(
                player.id.as_str(),
                concept.id.as_str(),
                "progress",
                35.0,
                None,
                Some("review proposal".into()),
                "test",
            )
            .unwrap();
        assert_eq!(
            concepts.progress_tracks(concept.id.as_str()).unwrap()[0].current_value,
            10.0
        );
        assert!(semantics
            .accept_suggestion(player.id.as_str(), suggestion.id.as_str())
            .is_err());
        assert_eq!(
            concepts.progress_tracks(concept.id.as_str()).unwrap()[0].current_value,
            10.0
        );
        assert_eq!(
            semantics.suggestions(concept.id.as_str(), false).unwrap()[0].status,
            SuggestionStatus::Pending
        );
        world.set_rule_enabled(rule.id.as_str(), false).unwrap();
        let accepted = semantics
            .accept_suggestion(player.id.as_str(), suggestion.id.as_str())
            .unwrap();
        assert_eq!(accepted.status, SuggestionStatus::Accepted);
        assert_eq!(
            concepts.progress_tracks(concept.id.as_str()).unwrap()[0].current_value,
            35.0
        );
        assert_eq!(
            concepts
                .progress_history(concept.id.as_str(), Some("progress"))
                .unwrap()
                .len(),
            2
        );
    }
    #[test]
    fn cross_world_stage_session_and_association_writes_fail_closed() {
        let store = store();
        let world = WorldService::new(store.clone(), Frozen);
        let semantics = SemanticsService::new(store.clone(), Frozen);
        let concepts = ConceptService::new(store.clone(), Frozen);
        let ada = world.create_player("Ada", None).unwrap();
        let grace = world.create_player("Grace", None).unwrap();
        let quest = world
            .create_quest(ada.id.as_str(), "main", "Quest", None, None, None, Some(0))
            .unwrap();
        let stage = semantics
            .create_stage(ada.id.as_str(), quest.id.as_str(), "Stage", 0)
            .unwrap();
        assert!(semantics
            .create_stage(grace.id.as_str(), quest.id.as_str(), "Bad stage", 1)
            .is_err());
        assert!(semantics
            .start_session(
                grace.id.as_str(),
                Some(quest.id.as_str()),
                Some(stage.id.as_str()),
                None,
                None,
                None,
                None
            )
            .is_err());
        let c = concepts
            .create_concept(grace.id.as_str(), "subject", "Other world", None)
            .unwrap();
        assert!(semantics
            .associate(
                c.id.as_str(),
                AssociatedEntityKind::Quest,
                quest.id.as_str(),
                "related"
            )
            .is_err());
        let tree = world
            .create_skill_tree(ada.id.as_str(), "programming", "Programming")
            .unwrap();
        let skill = world
            .add_skill(tree.id.as_str(), "core", "Rust", None)
            .unwrap();
        let child = world
            .add_skill(
                tree.id.as_str(),
                "advanced",
                "Ownership",
                Some(skill.id.to_string()),
            )
            .unwrap();
        let python = concepts
            .create_concept(ada.id.as_str(), "subject", "Python", None)
            .unwrap();
        semantics
            .associate(
                python.id.as_str(),
                AssociatedEntityKind::Skill,
                skill.id.as_str(),
                "related_to",
            )
            .unwrap();
        let guide = world
            .write_narrative(
                ada.id.as_str(),
                "guide",
                "Rust guide",
                "Borrowing and ownership",
            )
            .unwrap();
        semantics
            .attach_content(
                ada.id.as_str(),
                guide.id.as_str(),
                ContentTargetKind::Skill,
                skill.id.as_str(),
                "guidance",
            )
            .unwrap();
        world
            .add_comment(
                Some(ada.id.to_string()),
                "skill",
                skill.id.as_str(),
                "Practice ownership daily.",
            )
            .unwrap();
        let detail = semantics.skill_detail(skill.id.as_str()).unwrap().unwrap();
        assert_eq!(detail.tree.unwrap().id, tree.id);
        assert_eq!(detail.children[0].id, child.id);
        assert_eq!(detail.concept_associations.len(), 1);
        assert_eq!(detail.content_entries.len(), 1);
        assert_eq!(detail.comments.len(), 1);
        assert!(detail.related_search_results.iter().any(|h| h.kind
            == lr_application::SearchEntityKind::Skill
            && h.id == skill.id.as_str()));
    }
}
