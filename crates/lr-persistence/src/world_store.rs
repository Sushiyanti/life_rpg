//! SQLite implementation of the Phase 2 [`lr_application::WorldStore`] port.
//!
//! This is the only module that maps domain values to SQL rows. Compound state
//! changes use one SQLite transaction, so current state and append-only history
//! cannot diverge after a partial failure.

use lr_application::{StorageError, WorldStore};
use lr_domain::{
    Comment, CommentTargetKind, DateValue, Effect, EntityId, Iso8601Timestamp, NarrativeEntry,
    Player, PlayerStateSnapshot, Quest, QuestStatus, Skill, SkillStateSnapshot, SkillStatus,
    SkillTree, Transaction, TypeDefinition, TypeRef,
};
use rusqlite::{params, Row, Transaction as SqlTransaction};

use crate::sqlite_store::SqliteHealthStore;

fn op(error: impl std::fmt::Display) -> StorageError {
    StorageError::Operation(error.to_string())
}
fn id(raw: String) -> Result<EntityId, StorageError> {
    EntityId::new(raw).map_err(|e| op(format!("corrupt entity id: {e}")))
}
fn timestamp(raw: String) -> Result<Iso8601Timestamp, StorageError> {
    Iso8601Timestamp::parse(raw).map_err(|e| op(format!("corrupt timestamp: {e}")))
}
fn date(raw: String) -> Result<DateValue, StorageError> {
    DateValue::parse(raw).map_err(|e| op(format!("corrupt date: {e}")))
}
fn type_ref(namespace: String, code: String) -> Result<TypeRef, StorageError> {
    TypeRef::new(namespace, code).map_err(|e| op(format!("corrupt type reference: {e}")))
}
fn quest_status(raw: String) -> Result<QuestStatus, StorageError> {
    QuestStatus::parse(&raw).map_err(|e| op(format!("corrupt quest status: {e}")))
}
fn skill_status(raw: String) -> Result<SkillStatus, StorageError> {
    SkillStatus::parse(&raw).map_err(|e| op(format!("corrupt skill status: {e}")))
}
fn b(value: i64) -> bool {
    value != 0
}

fn player(row: &Row<'_>) -> Result<Player, StorageError> {
    Ok(Player {
        id: id(row.get(0).map_err(op)?)?,
        name: row.get(1).map_err(op)?,
        description: row.get(2).map_err(op)?,
        level: row.get(3).map_err(op)?,
        current_xp: row.get(4).map_err(op)?,
        is_active: b(row.get(5).map_err(op)?),
        metadata_json: row.get(6).map_err(op)?,
        created_at: timestamp(row.get(7).map_err(op)?)?,
        updated_at: timestamp(row.get(8).map_err(op)?)?,
    })
}
fn quest(row: &Row<'_>) -> Result<Quest, StorageError> {
    Ok(Quest {
        id: id(row.get(0).map_err(op)?)?,
        player_id: id(row.get(1).map_err(op)?)?,
        parent_quest_id: row
            .get::<_, Option<String>>(2)
            .map_err(op)?
            .map(id)
            .transpose()?,
        skill_id: row
            .get::<_, Option<String>>(3)
            .map_err(op)?
            .map(id)
            .transpose()?,
        quest_type: type_ref(row.get(4).map_err(op)?, row.get(5).map_err(op)?)?,
        title: row.get(6).map_err(op)?,
        description: row.get(7).map_err(op)?,
        story: row.get(8).map_err(op)?,
        instructions: row.get(9).map_err(op)?,
        status: quest_status(row.get(10).map_err(op)?)?,
        difficulty: row.get(11).map_err(op)?,
        progress: row.get(12).map_err(op)?,
        xp_reward: row.get(13).map_err(op)?,
        due_at: row
            .get::<_, Option<String>>(14)
            .map_err(op)?
            .map(timestamp)
            .transpose()?,
        started_at: row
            .get::<_, Option<String>>(15)
            .map_err(op)?
            .map(timestamp)
            .transpose()?,
        completed_at: row
            .get::<_, Option<String>>(16)
            .map_err(op)?
            .map(timestamp)
            .transpose()?,
        metadata_json: row.get(17).map_err(op)?,
        created_at: timestamp(row.get(18).map_err(op)?)?,
        updated_at: timestamp(row.get(19).map_err(op)?)?,
    })
}
fn tree(row: &Row<'_>) -> Result<SkillTree, StorageError> {
    Ok(SkillTree {
        id: id(row.get(0).map_err(op)?)?,
        player_id: id(row.get(1).map_err(op)?)?,
        tree_type: type_ref(row.get(2).map_err(op)?, row.get(3).map_err(op)?)?,
        name: row.get(4).map_err(op)?,
        description: row.get(5).map_err(op)?,
        story: row.get(6).map_err(op)?,
        instructions: row.get(7).map_err(op)?,
        is_active: b(row.get(8).map_err(op)?),
        metadata_json: row.get(9).map_err(op)?,
        created_at: timestamp(row.get(10).map_err(op)?)?,
        updated_at: timestamp(row.get(11).map_err(op)?)?,
    })
}
fn skill(row: &Row<'_>) -> Result<Skill, StorageError> {
    Ok(Skill {
        id: id(row.get(0).map_err(op)?)?,
        skill_tree_id: id(row.get(1).map_err(op)?)?,
        parent_skill_id: row
            .get::<_, Option<String>>(2)
            .map_err(op)?
            .map(id)
            .transpose()?,
        skill_type: type_ref(row.get(3).map_err(op)?, row.get(4).map_err(op)?)?,
        name: row.get(5).map_err(op)?,
        description: row.get(6).map_err(op)?,
        story: row.get(7).map_err(op)?,
        instructions: row.get(8).map_err(op)?,
        level: row.get(9).map_err(op)?,
        current_xp: row.get(10).map_err(op)?,
        invested_minutes: row.get(11).map_err(op)?,
        status: skill_status(row.get(12).map_err(op)?)?,
        started_at: row
            .get::<_, Option<String>>(13)
            .map_err(op)?
            .map(timestamp)
            .transpose()?,
        completed_at: row
            .get::<_, Option<String>>(14)
            .map_err(op)?
            .map(timestamp)
            .transpose()?,
        metadata_json: row.get(15).map_err(op)?,
        created_at: timestamp(row.get(16).map_err(op)?)?,
        updated_at: timestamp(row.get(17).map_err(op)?)?,
    })
}
fn effect(row: &Row<'_>) -> Result<Effect, StorageError> {
    Ok(Effect {
        id: id(row.get(0).map_err(op)?)?,
        player_id: id(row.get(1).map_err(op)?)?,
        effect_type: type_ref(row.get(2).map_err(op)?, row.get(3).map_err(op)?)?,
        name: row.get(4).map_err(op)?,
        description: row.get(5).map_err(op)?,
        started_at: timestamp(row.get(6).map_err(op)?)?,
        expires_at: row
            .get::<_, Option<String>>(7)
            .map_err(op)?
            .map(timestamp)
            .transpose()?,
        intensity: row.get(8).map_err(op)?,
        source_kind: row.get(9).map_err(op)?,
        source_id: row.get(10).map_err(op)?,
        metadata_json: row.get(11).map_err(op)?,
        created_at: timestamp(row.get(12).map_err(op)?)?,
        updated_at: timestamp(row.get(13).map_err(op)?)?,
    })
}
fn transaction(row: &Row<'_>) -> Result<Transaction, StorageError> {
    Ok(Transaction {
        id: Some(row.get(0).map_err(op)?),
        player_id: id(row.get(1).map_err(op)?)?,
        transaction_type: type_ref(row.get(2).map_err(op)?, row.get(3).map_err(op)?)?,
        resource: row.get(4).map_err(op)?,
        amount: row.get(5).map_err(op)?,
        occurred_at: timestamp(row.get(6).map_err(op)?)?,
        reason: row.get(7).map_err(op)?,
        description: row.get(8).map_err(op)?,
        source_kind: row.get(9).map_err(op)?,
        source_id: row.get(10).map_err(op)?,
        metadata_json: row.get(11).map_err(op)?,
    })
}
fn comment(row: &Row<'_>) -> Result<Comment, StorageError> {
    Ok(Comment {
        id: Some(row.get(0).map_err(op)?),
        author_player_id: row
            .get::<_, Option<String>>(1)
            .map_err(op)?
            .map(id)
            .transpose()?,
        target_kind: CommentTargetKind::parse(&row.get::<_, String>(2).map_err(op)?)
            .map_err(|e| op(format!("corrupt comment kind: {e}")))?,
        target_id: id(row.get(3).map_err(op)?)?,
        body: row.get(4).map_err(op)?,
        metadata_json: row.get(5).map_err(op)?,
        created_at: timestamp(row.get(6).map_err(op)?)?,
        updated_at: timestamp(row.get(7).map_err(op)?)?,
    })
}
fn narrative(row: &Row<'_>) -> Result<NarrativeEntry, StorageError> {
    Ok(NarrativeEntry {
        id: id(row.get(0).map_err(op)?)?,
        player_id: id(row.get(1).map_err(op)?)?,
        kind: type_ref(row.get(2).map_err(op)?, row.get(3).map_err(op)?)?,
        title: row.get(4).map_err(op)?,
        content: row.get(5).map_err(op)?,
        author: row.get(6).map_err(op)?,
        source_kind: row.get(7).map_err(op)?,
        source_id: row.get(8).map_err(op)?,
        metadata_json: row.get(9).map_err(op)?,
        created_at: timestamp(row.get(10).map_err(op)?)?,
        updated_at: timestamp(row.get(11).map_err(op)?)?,
    })
}

const PLAYER_SQL: &str =
    "id,name,description,level,current_xp,is_active,metadata_json,created_at,updated_at";
const QUEST_SQL: &str = "id,player_id,parent_quest_id,skill_id,quest_type_namespace,quest_type_code,title,description,story,instructions,status,difficulty,progress,xp_reward,due_at,started_at,completed_at,metadata_json,created_at,updated_at";
const TREE_SQL: &str = "id,player_id,tree_type_namespace,tree_type_code,name,description,story,instructions,is_active,metadata_json,created_at,updated_at";
const SKILL_SQL: &str = "id,skill_tree_id,parent_skill_id,skill_type_namespace,skill_type_code,name,description,story,instructions,level,current_xp,invested_minutes,status,started_at,completed_at,metadata_json,created_at,updated_at";
const EFFECT_SQL: &str = "id,player_id,effect_type_namespace,effect_type_code,name,description,started_at,expires_at,intensity,source_kind,source_id,metadata_json,created_at,updated_at";
const TX_SQL: &str = "id,player_id,transaction_type_namespace,transaction_type_code,resource,amount,occurred_at,reason,description,source_kind,source_id,metadata_json";
const COMMENT_SQL: &str =
    "id,author_player_id,target_kind,target_id,body,metadata_json,created_at,updated_at";
const NARRATIVE_SQL: &str = "id,player_id,kind_namespace,kind_code,title,content,author,source_kind,source_id,metadata_json,created_at,updated_at";

fn insert_transaction(
    tx: &SqlTransaction<'_>,
    value: &Transaction,
) -> Result<Transaction, StorageError> {
    tx.execute("INSERT INTO transactions (player_id,transaction_type_namespace,transaction_type_code,resource,amount,occurred_at,reason,description,source_kind,source_id,metadata_json) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![value.player_id.as_str(),value.transaction_type.namespace,value.transaction_type.code,value.resource,value.amount,value.occurred_at.as_str(),value.reason,value.description,value.source_kind,value.source_id,value.metadata_json]).map_err(op)?;
    let mut stored = value.clone();
    stored.id = Some(tx.last_insert_rowid());
    Ok(stored)
}
fn target_exists(
    tx: &SqlTransaction<'_>,
    kind: CommentTargetKind,
    target: &EntityId,
) -> Result<bool, StorageError> {
    let table = match kind {
        CommentTargetKind::Player => "players",
        CommentTargetKind::Quest => "quests",
        CommentTargetKind::Skill => "skills",
        CommentTargetKind::SkillTree => "skill_trees",
        CommentTargetKind::Effect => "effects",
        CommentTargetKind::Transaction => "transactions",
        CommentTargetKind::NarrativeEntry => "narrative_entries",
    };
    let sql = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1)");
    tx.query_row(&sql, [target.as_str()], |row| row.get::<_, i64>(0))
        .map(|v| v != 0)
        .map_err(op)
}

impl WorldStore for SqliteHealthStore {
    fn list_type_definitions(
        &self,
        namespace: Option<&str>,
    ) -> Result<Vec<TypeDefinition>, StorageError> {
        self.with_conn(|conn|{
        let sql="SELECT id,namespace,code,label,description,sort_order,is_active,is_system,metadata_json,created_at,updated_at FROM type_definitions WHERE (?1 IS NULL OR namespace=?1) ORDER BY namespace,sort_order,code";
        let mut stmt=conn.prepare(sql).map_err(op)?;let mut rows=stmt.query([namespace]).map_err(op)?;let mut out=vec![];while let Some(row)=rows.next().map_err(op)?{let numeric:i64=row.get(0).map_err(op)?;out.push(TypeDefinition{id:id(format!("type-{numeric}"))?,type_ref:type_ref(row.get(1).map_err(op)?,row.get(2).map_err(op)?)?,label:row.get(3).map_err(op)?,description:row.get(4).map_err(op)?,sort_order:row.get(5).map_err(op)?,is_active:b(row.get(6).map_err(op)?),is_system:b(row.get(7).map_err(op)?),metadata_json:row.get(8).map_err(op)?,created_at:timestamp(row.get(9).map_err(op)?)?,updated_at:timestamp(row.get(10).map_err(op)?)?});}Ok(out)})
    }
    fn create_type_definition(&self, d: &TypeDefinition) -> Result<TypeDefinition, StorageError> {
        self.with_conn_mut(|conn|{conn.execute("INSERT INTO type_definitions(namespace,code,label,description,sort_order,is_active,is_system,metadata_json,created_at,updated_at)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![d.type_ref.namespace,d.type_ref.code,d.label,d.description,d.sort_order,d.is_active as i64,d.is_system as i64,d.metadata_json,d.created_at.as_str(),d.updated_at.as_str()]).map_err(op)?;let mut stored=d.clone();stored.id=id(format!("type-{}",conn.last_insert_rowid()))?;Ok(stored)})
    }

    fn create_player(&self, p: &Player) -> Result<(), StorageError> {
        self.with_conn(|conn|{conn.execute("INSERT INTO players(id,name,description,level,current_xp,is_active,metadata_json,created_at,updated_at)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![p.id.as_str(),p.name,p.description,p.level,p.current_xp,p.is_active as i64,p.metadata_json,p.created_at.as_str(),p.updated_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn get_player(&self, key: &EntityId) -> Result<Option<Player>, StorageError> {
        self.with_conn(|conn| {
            let sql = format!("SELECT {PLAYER_SQL} FROM players WHERE id=?1");
            let mut stmt = conn.prepare(&sql).map_err(op)?;
            let mut rows = stmt.query([key.as_str()]).map_err(op)?;
            match rows.next().map_err(op)? {
                Some(row) => Ok(Some(player(row)?)),
                None => Ok(None),
            }
        })
    }
    fn update_player(&self, p: &Player) -> Result<(), StorageError> {
        self.with_conn(|conn|{let changed=conn.execute("UPDATE players SET name=?2,description=?3,level=?4,current_xp=?5,is_active=?6,metadata_json=?7,updated_at=?8 WHERE id=?1",params![p.id.as_str(),p.name,p.description,p.level,p.current_xp,p.is_active as i64,p.metadata_json,p.updated_at.as_str()]).map_err(op)?;if changed==0{Err(op("player not found"))}else{Ok(())}})
    }

    fn insert_quest(&self, q: &Quest) -> Result<(), StorageError> {
        self.with_conn(|conn|{conn.execute("INSERT INTO quests(id,player_id,parent_quest_id,skill_id,quest_type_namespace,quest_type_code,title,description,story,instructions,status,difficulty,progress,xp_reward,due_at,started_at,completed_at,metadata_json,created_at,updated_at)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",params![q.id.as_str(),q.player_id.as_str(),q.parent_quest_id.as_ref().map(EntityId::as_str),q.skill_id.as_ref().map(EntityId::as_str),q.quest_type.namespace,q.quest_type.code,q.title,q.description,q.story,q.instructions,q.status.as_str(),q.difficulty,q.progress,q.xp_reward,q.due_at.as_ref().map(Iso8601Timestamp::as_str),q.started_at.as_ref().map(Iso8601Timestamp::as_str),q.completed_at.as_ref().map(Iso8601Timestamp::as_str),q.metadata_json,q.created_at.as_str(),q.updated_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn get_quest(&self, key: &EntityId) -> Result<Option<Quest>, StorageError> {
        self.with_conn(|conn| {
            let sql = format!("SELECT {QUEST_SQL} FROM quests WHERE id=?1");
            let mut stmt = conn.prepare(&sql).map_err(op)?;
            let mut rows = stmt.query([key.as_str()]).map_err(op)?;
            match rows.next().map_err(op)? {
                Some(r) => Ok(Some(quest(r)?)),
                None => Ok(None),
            }
        })
    }
    fn update_quest(&self, q: &Quest) -> Result<(), StorageError> {
        self.with_conn(|conn|{let changed=conn.execute("UPDATE quests SET parent_quest_id=?2,skill_id=?3,title=?4,description=?5,story=?6,instructions=?7,status=?8,difficulty=?9,progress=?10,xp_reward=?11,due_at=?12,started_at=?13,completed_at=?14,metadata_json=?15,updated_at=?16 WHERE id=?1",params![q.id.as_str(),q.parent_quest_id.as_ref().map(EntityId::as_str),q.skill_id.as_ref().map(EntityId::as_str),q.title,q.description,q.story,q.instructions,q.status.as_str(),q.difficulty,q.progress,q.xp_reward,q.due_at.as_ref().map(Iso8601Timestamp::as_str),q.started_at.as_ref().map(Iso8601Timestamp::as_str),q.completed_at.as_ref().map(Iso8601Timestamp::as_str),q.metadata_json,q.updated_at.as_str()]).map_err(op)?;if changed==0{Err(op("quest not found"))}else{Ok(())}})
    }
    fn list_quests(&self, p: &EntityId) -> Result<Vec<Quest>, StorageError> {
        self.with_conn(|conn| {
            let sql = format!(
                "SELECT {QUEST_SQL} FROM quests WHERE player_id=?1 ORDER BY created_at DESC"
            );
            let mut stmt = conn.prepare(&sql).map_err(op)?;
            let mut rows = stmt.query([p.as_str()]).map_err(op)?;
            let mut out = vec![];
            while let Some(r) = rows.next().map_err(op)? {
                out.push(quest(r)?);
            }
            Ok(out)
        })
    }

    fn insert_skill_tree(&self, t: &SkillTree) -> Result<(), StorageError> {
        self.with_conn(|conn|{conn.execute("INSERT INTO skill_trees(id,player_id,tree_type_namespace,tree_type_code,name,description,story,instructions,is_active,metadata_json,created_at,updated_at)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![t.id.as_str(),t.player_id.as_str(),t.tree_type.namespace,t.tree_type.code,t.name,t.description,t.story,t.instructions,t.is_active as i64,t.metadata_json,t.created_at.as_str(),t.updated_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn get_skill_tree(&self, key: &EntityId) -> Result<Option<SkillTree>, StorageError> {
        self.with_conn(|conn| {
            let sql = format!("SELECT {TREE_SQL} FROM skill_trees WHERE id=?1");
            let mut stmt = conn.prepare(&sql).map_err(op)?;
            let mut rows = stmt.query([key.as_str()]).map_err(op)?;
            match rows.next().map_err(op)? {
                Some(row) => Ok(Some(tree(row)?)),
                None => Ok(None),
            }
        })
    }
    fn list_skill_trees(&self, p: &EntityId) -> Result<Vec<SkillTree>, StorageError> {
        self.with_conn(|conn| {
            let sql = format!(
                "SELECT {TREE_SQL} FROM skill_trees WHERE player_id=?1 ORDER BY created_at"
            );
            let mut stmt = conn.prepare(&sql).map_err(op)?;
            let mut rows = stmt.query([p.as_str()]).map_err(op)?;
            let mut out = vec![];
            while let Some(r) = rows.next().map_err(op)? {
                out.push(tree(r)?);
            }
            Ok(out)
        })
    }
    fn insert_skill(&self, s: &Skill) -> Result<(), StorageError> {
        self.with_conn(|conn|{conn.execute("INSERT INTO skills(id,skill_tree_id,parent_skill_id,skill_type_namespace,skill_type_code,name,description,story,instructions,level,current_xp,invested_minutes,status,started_at,completed_at,metadata_json,created_at,updated_at)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",params![s.id.as_str(),s.skill_tree_id.as_str(),s.parent_skill_id.as_ref().map(EntityId::as_str),s.skill_type.namespace,s.skill_type.code,s.name,s.description,s.story,s.instructions,s.level,s.current_xp,s.invested_minutes,s.status.as_str(),s.started_at.as_ref().map(Iso8601Timestamp::as_str),s.completed_at.as_ref().map(Iso8601Timestamp::as_str),s.metadata_json,s.created_at.as_str(),s.updated_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn get_skill(&self, key: &EntityId) -> Result<Option<Skill>, StorageError> {
        self.with_conn(|conn| {
            let sql = format!("SELECT {SKILL_SQL} FROM skills WHERE id=?1");
            let mut stmt = conn.prepare(&sql).map_err(op)?;
            let mut rows = stmt.query([key.as_str()]).map_err(op)?;
            match rows.next().map_err(op)? {
                Some(r) => Ok(Some(skill(r)?)),
                None => Ok(None),
            }
        })
    }
    fn update_skill(&self, s: &Skill) -> Result<(), StorageError> {
        self.with_conn(|conn|{let changed=conn.execute("UPDATE skills SET parent_skill_id=?2,name=?3,description=?4,story=?5,instructions=?6,level=?7,current_xp=?8,invested_minutes=?9,status=?10,started_at=?11,completed_at=?12,metadata_json=?13,updated_at=?14 WHERE id=?1",params![s.id.as_str(),s.parent_skill_id.as_ref().map(EntityId::as_str),s.name,s.description,s.story,s.instructions,s.level,s.current_xp,s.invested_minutes,s.status.as_str(),s.started_at.as_ref().map(Iso8601Timestamp::as_str),s.completed_at.as_ref().map(Iso8601Timestamp::as_str),s.metadata_json,s.updated_at.as_str()]).map_err(op)?;if changed==0{Err(op("skill not found"))}else{Ok(())}})
    }
    fn list_skills(&self, tree_id: &EntityId) -> Result<Vec<Skill>, StorageError> {
        self.with_conn(|conn| {
            let sql = format!(
                "SELECT {SKILL_SQL} FROM skills WHERE skill_tree_id=?1 ORDER BY created_at"
            );
            let mut stmt = conn.prepare(&sql).map_err(op)?;
            let mut rows = stmt.query([tree_id.as_str()]).map_err(op)?;
            let mut out = vec![];
            while let Some(r) = rows.next().map_err(op)? {
                out.push(skill(r)?);
            }
            Ok(out)
        })
    }

    fn insert_effect(&self, e: &Effect) -> Result<(), StorageError> {
        self.with_conn(|conn|{conn.execute("INSERT INTO effects(id,player_id,effect_type_namespace,effect_type_code,name,description,started_at,expires_at,intensity,source_kind,source_id,metadata_json,created_at,updated_at)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",params![e.id.as_str(),e.player_id.as_str(),e.effect_type.namespace,e.effect_type.code,e.name,e.description,e.started_at.as_str(),e.expires_at.as_ref().map(Iso8601Timestamp::as_str),e.intensity,e.source_kind,e.source_id,e.metadata_json,e.created_at.as_str(),e.updated_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn list_effects(
        &self,
        p: &EntityId,
        active_at: Option<&str>,
    ) -> Result<Vec<Effect>, StorageError> {
        self.with_conn(|conn|{let sql=format!("SELECT {EFFECT_SQL} FROM effects WHERE player_id=?1 AND (?2 IS NULL OR expires_at IS NULL OR expires_at>?2) ORDER BY started_at DESC");let mut stmt=conn.prepare(&sql).map_err(op)?;let mut rows=stmt.query(params![p.as_str(),active_at]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(effect(r)?);}Ok(out)})
    }

    fn append_transaction(&self, value: &Transaction) -> Result<Transaction, StorageError> {
        self.with_conn_mut(|conn| {
            let tx = conn.transaction().map_err(op)?;
            let stored = insert_transaction(&tx, value)?;
            tx.commit().map_err(op)?;
            Ok(stored)
        })
    }
    fn list_transactions(
        &self,
        p: &EntityId,
        limit: u32,
    ) -> Result<Vec<Transaction>, StorageError> {
        self.with_conn(|conn|{let sql=format!("SELECT {TX_SQL} FROM transactions WHERE player_id=?1 ORDER BY occurred_at DESC,id DESC LIMIT ?2");let mut stmt=conn.prepare(&sql).map_err(op)?;let mut rows=stmt.query(params![p.as_str(),limit]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(transaction(r)?);}Ok(out)})
    }
    fn transaction_total(&self, p: &EntityId, resource: &str) -> Result<i64, StorageError> {
        self.with_conn(|conn|conn.query_row("SELECT COALESCE(SUM(amount),0) FROM transactions WHERE player_id=?1 AND resource=?2",params![p.as_str(),resource],|r|r.get(0)).map_err(op))
    }

    fn insert_player_snapshot(&self, s: &PlayerStateSnapshot) -> Result<(), StorageError> {
        self.with_conn(|conn|{conn.execute("INSERT INTO player_state_snapshots(player_id,snapshot_date,level,current_xp,state_json,metadata_json,created_at)VALUES(?1,?2,?3,?4,?5,?6,?7)",params![s.player_id.as_str(),s.snapshot_date.as_str(),s.level,s.current_xp,s.state_json,s.metadata_json,s.created_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn list_player_snapshots(
        &self,
        p: &EntityId,
    ) -> Result<Vec<PlayerStateSnapshot>, StorageError> {
        self.with_conn(|conn|{let mut stmt=conn.prepare("SELECT id,player_id,snapshot_date,level,current_xp,state_json,metadata_json,created_at FROM player_state_snapshots WHERE player_id=?1 ORDER BY snapshot_date").map_err(op)?;let mut rows=stmt.query([p.as_str()]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(PlayerStateSnapshot{id:Some(r.get(0).map_err(op)?),player_id:id(r.get(1).map_err(op)?)?,snapshot_date:date(r.get(2).map_err(op)?)?,level:r.get(3).map_err(op)?,current_xp:r.get(4).map_err(op)?,state_json:r.get(5).map_err(op)?,metadata_json:r.get(6).map_err(op)?,created_at:timestamp(r.get(7).map_err(op)?)?});}Ok(out)})
    }
    fn insert_skill_snapshot(&self, s: &SkillStateSnapshot) -> Result<(), StorageError> {
        self.with_conn(|conn|{conn.execute("INSERT INTO skill_state_snapshots(skill_id,snapshot_date,level,current_xp,status,invested_minutes,state_json,metadata_json,created_at)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![s.skill_id.as_str(),s.snapshot_date.as_str(),s.level,s.current_xp,s.status.as_str(),s.invested_minutes,s.state_json,s.metadata_json,s.created_at.as_str()]).map_err(op)?;Ok(())})
    }

    fn add_comment(&self, c: &Comment) -> Result<Comment, StorageError> {
        self.with_conn_mut(|conn|{let tx=conn.transaction().map_err(op)?;if !target_exists(&tx,c.target_kind,&c.target_id)?{return Err(op("comment target does not exist"));}tx.execute("INSERT INTO comments(author_player_id,target_kind,target_id,body,metadata_json,created_at,updated_at)VALUES(?1,?2,?3,?4,?5,?6,?7)",params![c.author_player_id.as_ref().map(EntityId::as_str),c.target_kind.as_str(),c.target_id.as_str(),c.body,c.metadata_json,c.created_at.as_str(),c.updated_at.as_str()]).map_err(op)?;let mut stored=c.clone();stored.id=Some(tx.last_insert_rowid());tx.commit().map_err(op)?;Ok(stored)})
    }
    fn list_comments(
        &self,
        k: CommentTargetKind,
        target: &EntityId,
    ) -> Result<Vec<Comment>, StorageError> {
        self.with_conn(|conn|{let sql=format!("SELECT {COMMENT_SQL} FROM comments WHERE target_kind=?1 AND target_id=?2 ORDER BY created_at");let mut stmt=conn.prepare(&sql).map_err(op)?;let mut rows=stmt.query(params![k.as_str(),target.as_str()]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(comment(r)?);}Ok(out)})
    }
    fn insert_narrative_entry(&self, n: &NarrativeEntry) -> Result<(), StorageError> {
        self.with_conn(|conn|{conn.execute("INSERT INTO narrative_entries(id,player_id,kind_namespace,kind_code,title,content,author,source_kind,source_id,metadata_json,created_at,updated_at)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![n.id.as_str(),n.player_id.as_str(),n.kind.namespace,n.kind.code,n.title,n.content,n.author,n.source_kind,n.source_id,n.metadata_json,n.created_at.as_str(),n.updated_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn list_narrative_entries(&self, p: &EntityId) -> Result<Vec<NarrativeEntry>, StorageError> {
        self.with_conn(|conn|{let sql=format!("SELECT {NARRATIVE_SQL} FROM narrative_entries WHERE player_id=?1 ORDER BY created_at DESC");let mut stmt=conn.prepare(&sql).map_err(op)?;let mut rows=stmt.query([p.as_str()]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(narrative(r)?);}Ok(out)})
    }

    fn award_xp(&self, p: &Player, value: &Transaction) -> Result<Transaction, StorageError> {
        self.with_conn_mut(|conn| {
            let tx = conn.transaction().map_err(op)?;
            let changed = tx
                .execute(
                    "UPDATE players SET level=?2,current_xp=?3,updated_at=?4 WHERE id=?1",
                    params![p.id.as_str(), p.level, p.current_xp, p.updated_at.as_str()],
                )
                .map_err(op)?;
            if changed != 1 {
                return Err(op("player not found"));
            }
            let stored = insert_transaction(&tx, value)?;
            tx.commit().map_err(op)?;
            Ok(stored)
        })
    }
    fn complete_quest(
        &self,
        q: &Quest,
        p: Option<&Player>,
        reward: Option<&Transaction>,
    ) -> Result<Option<Transaction>, StorageError> {
        self.with_conn_mut(|conn|{let tx=conn.transaction().map_err(op)?;let changed=tx.execute("UPDATE quests SET status=?2,progress=?3,started_at=?4,completed_at=?5,updated_at=?6 WHERE id=?1",params![q.id.as_str(),q.status.as_str(),q.progress,q.started_at.as_ref().map(Iso8601Timestamp::as_str),q.completed_at.as_ref().map(Iso8601Timestamp::as_str),q.updated_at.as_str()]).map_err(op)?;if changed!=1{return Err(op("quest not found"));}let created=match(p,reward){(Some(player),Some(reward))=>{if tx.execute("UPDATE players SET level=?2,current_xp=?3,updated_at=?4 WHERE id=?1",params![player.id.as_str(),player.level,player.current_xp,player.updated_at.as_str()]).map_err(op)?!=1{return Err(op("reward player not found"));}Some(insert_transaction(&tx,reward)?)},(None,None)=>None,_=>return Err(op("quest reward player and transaction must be supplied together"))};tx.commit().map_err(op)?;Ok(created)})
    }
    fn invest_skill_time(
        &self,
        s: &Skill,
        value: &Transaction,
    ) -> Result<Transaction, StorageError> {
        self.with_conn_mut(|conn| {
            let tx = conn.transaction().map_err(op)?;
            if tx
                .execute(
                    "UPDATE skills SET invested_minutes=?2,updated_at=?3 WHERE id=?1",
                    params![s.id.as_str(), s.invested_minutes, s.updated_at.as_str()],
                )
                .map_err(op)?
                != 1
            {
                return Err(op("skill not found"));
            }
            let stored = insert_transaction(&tx, value)?;
            tx.commit().map_err(op)?;
            Ok(stored)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lr_application::WorldStore;
    const T0: &str = "2026-09-25T00:00:00+00:00";
    fn now() -> Iso8601Timestamp {
        Iso8601Timestamp::parse(T0).unwrap()
    }
    fn store() -> SqliteHealthStore {
        SqliteHealthStore::open_in_memory(T0)
    }
    fn player_value() -> Player {
        Player::new(EntityId::new("p1").unwrap(), "Ada", now()).unwrap()
    }
    #[test]
    fn persists_the_world_and_keeps_history_atomic() {
        let store = store();
        let mut p = player_value();
        store.create_player(&p).unwrap();
        let tree = SkillTree::new(
            EntityId::new("tree").unwrap(),
            p.id.clone(),
            TypeRef::skill_tree("programming").unwrap(),
            "Programming",
            now(),
        )
        .unwrap();
        store.insert_skill_tree(&tree).unwrap();
        let mut skill = Skill::new(
            EntityId::new("skill").unwrap(),
            tree.id.clone(),
            TypeRef::skill("core").unwrap(),
            "Rust",
            now(),
        )
        .unwrap();
        store.insert_skill(&skill).unwrap();
        let mut q = Quest::new(
            EntityId::new("quest").unwrap(),
            p.id.clone(),
            TypeRef::quest("main").unwrap(),
            "Ship it",
            now(),
        )
        .unwrap();
        q.skill_id = Some(skill.id.clone());
        q.xp_reward = 25;
        store.insert_quest(&q).unwrap();
        p.apply_xp(25, now()).unwrap();
        let mut tx = Transaction::new(
            p.id.clone(),
            TypeRef::transaction("xp").unwrap(),
            "xp",
            25,
            now(),
        )
        .unwrap();
        tx.source_kind = Some("quest".into());
        tx.source_id = Some(q.id.to_string());
        store.award_xp(&p, &tx).unwrap();
        q.complete(now()).unwrap();
        store.complete_quest(&q, None, None).unwrap();
        skill.invest_time(60, now()).unwrap();
        let time = Transaction::new(
            p.id.clone(),
            TypeRef::transaction("time").unwrap(),
            "minutes",
            60,
            now(),
        )
        .unwrap();
        store.invest_skill_time(&skill, &time).unwrap();
        assert_eq!(store.get_player(&p.id).unwrap().unwrap().current_xp, 25);
        assert_eq!(store.transaction_total(&p.id, "xp").unwrap(), 25);
        assert_eq!(
            store.list_quests(&p.id).unwrap()[0].status,
            QuestStatus::Completed
        );
        assert_eq!(store.list_skills(&tree.id).unwrap()[0].invested_minutes, 60);
    }
    #[test]
    fn snapshots_comments_narratives_effects_and_constraints_work() {
        let store = store();
        let p = player_value();
        store.create_player(&p).unwrap();
        let d = DateValue::parse("2026-09-25").unwrap();
        let snap = PlayerStateSnapshot::new(p.id.clone(), d.clone(), 1, 0, now()).unwrap();
        store.insert_player_snapshot(&snap).unwrap();
        assert!(store.insert_player_snapshot(&snap).is_err());
        let comment = Comment::new(
            Some(p.id.clone()),
            CommentTargetKind::Player,
            p.id.clone(),
            "First day",
            now(),
        )
        .unwrap();
        store.add_comment(&comment).unwrap();
        assert_eq!(
            store
                .list_comments(CommentTargetKind::Player, &p.id)
                .unwrap()
                .len(),
            1
        );
        let ghost = Comment::new(
            None,
            CommentTargetKind::Quest,
            EntityId::new("missing").unwrap(),
            "no",
            now(),
        )
        .unwrap();
        assert!(store.add_comment(&ghost).is_err());
        let n = NarrativeEntry::new(
            EntityId::new("n1").unwrap(),
            p.id.clone(),
            TypeRef::new("narrative_entry", "journal").unwrap(),
            "Day 1",
            "Started",
            now(),
        )
        .unwrap();
        store.insert_narrative_entry(&n).unwrap();
        assert_eq!(store.list_narrative_entries(&p.id).unwrap().len(), 1);
        let effect = Effect::new(
            EntityId::new("e1").unwrap(),
            p.id.clone(),
            TypeRef::effect("buff").unwrap(),
            "Focused",
            now(),
        )
        .unwrap();
        store.insert_effect(&effect).unwrap();
        assert_eq!(store.list_effects(&p.id, Some(T0)).unwrap().len(), 1);
    }
}
