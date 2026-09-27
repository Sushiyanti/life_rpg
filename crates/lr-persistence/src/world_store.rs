//! SQLite implementation of the Phase 2 [`lr_application::WorldStore`] port.
//!
//! This is the only module that maps domain values to SQL rows. Compound state
//! changes use one SQLite transaction, so current state and append-only history
//! cannot diverge after a partial failure.

use lr_application::{
    EventKind, Rule, RuleDefinition, RuleExecutionRecord, RuleOperation, StorageError, WorldStore,
};
use lr_domain::{
    Comment, CommentTargetKind, DateValue, Effect, EntityId, Iso8601Timestamp, NarrativeEntry,
    Player, PlayerStat, PlayerStateSnapshot, Quest, QuestStatus, Skill, SkillStateSnapshot,
    SkillStatus, SkillTree, StatDefinition, Transaction, TypeDefinition, TypeRef,
};
use rusqlite::{params, OptionalExtension, Row, Transaction as SqlTransaction};

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
        level_name: row.get(9).map_err(op)?,
        progression_label: row.get(10).map_err(op)?,
        current_xp: row.get(4).map_err(op)?,
        is_active: b(row.get(5).map_err(op)?),
        metadata_json: row.get(6).map_err(op)?,
        created_at: timestamp(row.get(7).map_err(op)?)?,
        updated_at: timestamp(row.get(8).map_err(op)?)?,
    })
}
fn stat_definition(row: &Row<'_>) -> Result<StatDefinition, StorageError> {
    Ok(StatDefinition {
        id: id(row.get(0).map_err(op)?)?,
        code: row.get(1).map_err(op)?,
        name: row.get(2).map_err(op)?,
        description: row.get(3).map_err(op)?,
        unit: row.get(4).map_err(op)?,
        minimum: row.get(5).map_err(op)?,
        maximum: row.get(6).map_err(op)?,
        is_active: b(row.get(7).map_err(op)?),
        metadata_json: row.get(8).map_err(op)?,
        created_at: timestamp(row.get(9).map_err(op)?)?,
        updated_at: timestamp(row.get(10).map_err(op)?)?,
    })
}
fn player_stat(row: &Row<'_>) -> Result<PlayerStat, StorageError> {
    Ok(PlayerStat {
        player_id: id(row.get(0).map_err(op)?)?,
        stat_code: row.get(1).map_err(op)?,
        current_value: row.get(2).map_err(op)?,
        metadata_json: row.get(3).map_err(op)?,
        updated_at: timestamp(row.get(4).map_err(op)?)?,
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
        level_name: row.get(18).map_err(op)?,
        progression_label: row.get(19).map_err(op)?,
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
    let target_kind: String = row.get(15).map_err(op)?;
    let target_concept_id: Option<String> = row.get(16).map_err(op)?;
    if (target_kind == "player" && target_concept_id.is_some())
        || (target_kind == "concept" && target_concept_id.is_none())
        || !matches!(target_kind.as_str(), "player" | "concept")
    {
        return Err(op("corrupt Effect target reference"));
    }
    Ok(Effect {
        id: id(row.get(0).map_err(op)?)?,
        player_id: id(row.get(1).map_err(op)?)?,
        effect_type: type_ref(row.get(2).map_err(op)?, row.get(3).map_err(op)?)?,
        target_concept_id: target_concept_id.map(id).transpose()?,
        name: row.get(4).map_err(op)?,
        description: row.get(5).map_err(op)?,
        started_at: timestamp(row.get(6).map_err(op)?)?,
        expires_at: row
            .get::<_, Option<String>>(7)
            .map_err(op)?
            .map(timestamp)
            .transpose()?,
        deactivated_at: row
            .get::<_, Option<String>>(8)
            .map_err(op)?
            .map(timestamp)
            .transpose()?,
        intensity: row.get(9).map_err(op)?,
        source_kind: row.get(10).map_err(op)?,
        source_id: row.get(11).map_err(op)?,
        metadata_json: row.get(12).map_err(op)?,
        created_at: timestamp(row.get(13).map_err(op)?)?,
        updated_at: timestamp(row.get(14).map_err(op)?)?,
    })
}
fn transaction(row: &Row<'_>) -> Result<Transaction, StorageError> {
    Ok(Transaction {
        id: Some(row.get(0).map_err(op)?),
        player_id: id(row.get(1).map_err(op)?)?,
        transaction_type: type_ref(row.get(2).map_err(op)?, row.get(3).map_err(op)?)?,
        resource: row.get(4).map_err(op)?,
        amount: row.get(5).map_err(op)?,
        applied_amount: row.get(6).map_err(op)?,
        occurred_at: timestamp(row.get(7).map_err(op)?)?,
        captured_at: row
            .get::<_, Option<String>>(13)
            .map_err(op)?
            .map(timestamp)
            .transpose()?,
        reason: row.get(8).map_err(op)?,
        description: row.get(9).map_err(op)?,
        source_kind: row.get(10).map_err(op)?,
        source_id: row.get(11).map_err(op)?,
        metadata_json: row.get(12).map_err(op)?,
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
        is_active: b(row.get(12).map_err(op)?),
    })
}

const PLAYER_SQL: &str =
    "id,name,description,level,current_xp,is_active,metadata_json,created_at,updated_at,level_name,progression_label";
const QUEST_SQL: &str = "id,player_id,parent_quest_id,skill_id,quest_type_namespace,quest_type_code,title,description,story,instructions,status,difficulty,progress,xp_reward,due_at,started_at,completed_at,metadata_json,created_at,updated_at";
const TREE_SQL: &str = "id,player_id,tree_type_namespace,tree_type_code,name,description,story,instructions,is_active,metadata_json,created_at,updated_at";
const SKILL_SQL: &str = "id,skill_tree_id,parent_skill_id,skill_type_namespace,skill_type_code,name,description,story,instructions,level,current_xp,invested_minutes,status,started_at,completed_at,metadata_json,created_at,updated_at,level_name,progression_label";
const EFFECT_SQL: &str = "id,player_id,effect_type_namespace,effect_type_code,name,description,started_at,expires_at,deactivated_at,intensity,source_kind,source_id,metadata_json,created_at,updated_at,target_kind,target_concept_id";
const TX_SQL: &str = "id,player_id,transaction_type_namespace,transaction_type_code,resource,amount,applied_amount,occurred_at,reason,description,source_kind,source_id,metadata_json,captured_at";
const COMMENT_SQL: &str =
    "id,author_player_id,target_kind,target_id,body,metadata_json,created_at,updated_at";
const NARRATIVE_SQL: &str = "id,player_id,kind_namespace,kind_code,title,content,author,source_kind,source_id,metadata_json,created_at,updated_at,is_active";

fn insert_transaction(
    tx: &SqlTransaction<'_>,
    value: &Transaction,
) -> Result<Transaction, StorageError> {
    tx.execute("INSERT INTO transactions (player_id,transaction_type_namespace,transaction_type_code,resource,amount,applied_amount,occurred_at,reason,description,source_kind,source_id,metadata_json,captured_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![value.player_id.as_str(),value.transaction_type.namespace,value.transaction_type.code,value.resource,value.amount,value.applied_amount,value.occurred_at.as_str(),value.reason,value.description,value.source_kind,value.source_id,value.metadata_json,value.captured_at.as_ref().map(Iso8601Timestamp::as_str)]).map_err(op)?;
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

fn event_kind(raw: String) -> Result<EventKind, StorageError> {
    match raw.as_str() {
        "quest_completed" => Ok(EventKind::QuestCompleted),
        "player_xp_changed" => Ok(EventKind::PlayerXpChanged),
        "stat_changed" => Ok(EventKind::StatChanged),
        "concept_progress_changed" => Ok(EventKind::ConceptProgressChanged),
        _ => Err(op(format!("unknown stored rule event kind `{raw}`"))),
    }
}
fn rule(row: &Row<'_>) -> Result<Rule, StorageError> {
    let definition_json: String = row.get(7).map_err(op)?;
    let definition: RuleDefinition = serde_json::from_str(&definition_json).map_err(op)?;
    definition.validate().map_err(op)?;
    let trigger = event_kind(row.get(5).map_err(op)?)?;
    if trigger != definition.trigger {
        return Err(op("stored rule trigger disagrees with its definition"));
    }
    Ok(Rule {
        id: id(row.get(0).map_err(op)?)?,
        name: row.get(1).map_err(op)?,
        description: row.get(2).map_err(op)?,
        enabled: b(row.get(3).map_err(op)?),
        priority: row.get(4).map_err(op)?,
        definition,
        metadata_json: row.get(8).map_err(op)?,
        created_at: timestamp(row.get(9).map_err(op)?)?,
        updated_at: timestamp(row.get(10).map_err(op)?)?,
    })
}
fn rule_execution(row: &Row<'_>) -> Result<RuleExecutionRecord, StorageError> {
    Ok(RuleExecutionRecord {
        id: row.get(0).map_err(op)?,
        chain_id: row.get(1).map_err(op)?,
        rule_id: row.get(2).map_err(op)?,
        event_kind: event_kind(row.get(3).map_err(op)?)?,
        event_json: row.get(4).map_err(op)?,
        condition_passed: row.get::<_, Option<i64>>(5).map_err(op)?.map(|v| v != 0),
        actions_json: row.get(6).map_err(op)?,
        status: row.get(7).map_err(op)?,
        error: row.get(8).map_err(op)?,
        depth: row.get(9).map_err(op)?,
        executed_at: timestamp(row.get(10).map_err(op)?)?,
    })
}
fn insert_rule_execution(
    tx: &SqlTransaction<'_>,
    record: &RuleExecutionRecord,
) -> Result<(), StorageError> {
    tx.execute("INSERT INTO rule_execution_history(id,chain_id,rule_id,event_kind,event_json,condition_passed,actions_json,status,error,depth,executed_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![record.id,record.chain_id,record.rule_id,record.event_kind.as_str(),record.event_json,record.condition_passed.map(i64::from),record.actions_json,record.status,record.error,record.depth,record.executed_at.as_str()]).map_err(op)?;
    Ok(())
}
const RULE_SQL: &str = "id,name,description,is_enabled,priority,trigger_kind,schema_version,definition_json,metadata_json,created_at,updated_at";
const RULE_EXECUTION_SQL: &str = "id,chain_id,rule_id,event_kind,event_json,condition_passed,actions_json,status,error,depth,executed_at";

impl WorldStore for SqliteHealthStore {
    fn get_concept_for_rule(
        &self,
        id: &EntityId,
    ) -> Result<Option<lr_domain::Concept>, StorageError> {
        lr_application::ConceptStore::get_concept(self, id)
    }
    fn list_progress_definitions_for_rule(
        &self,
    ) -> Result<Vec<lr_domain::ProgressTrackDefinition>, StorageError> {
        lr_application::ConceptStore::list_progress_track_definitions(self)
    }
    fn list_progress_for_rule(
        &self,
        id: &EntityId,
    ) -> Result<Vec<lr_domain::ConceptProgressTrack>, StorageError> {
        lr_application::ConceptStore::list_concept_progress(self, id)
    }
    fn create_rule(&self, value: &Rule) -> Result<(), StorageError> {
        value.definition.validate().map_err(op)?;
        let json = serde_json::to_string(&value.definition).map_err(op)?;
        self.with_conn(|c|{c.execute("INSERT INTO rules(id,name,description,is_enabled,priority,trigger_kind,schema_version,definition_json,metadata_json,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![value.id.as_str(),value.name,value.description,value.enabled as i64,value.priority,value.definition.trigger.as_str(),value.definition.schema_version,json,value.metadata_json,value.created_at.as_str(),value.updated_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn get_rule(&self, key: &EntityId) -> Result<Option<Rule>, StorageError> {
        self.with_conn(|c| {
            let sql = format!("SELECT {RULE_SQL} FROM rules WHERE id=?1");
            let mut stmt = c.prepare(&sql).map_err(op)?;
            let mut rows = stmt.query([key.as_str()]).map_err(op)?;
            match rows.next().map_err(op)? {
                Some(r) => Ok(Some(rule(r)?)),
                None => Ok(None),
            }
        })
    }
    fn list_rules(&self) -> Result<Vec<Rule>, StorageError> {
        self.with_conn(|c| {
            let sql = format!("SELECT {RULE_SQL} FROM rules ORDER BY priority DESC,id ASC");
            let mut stmt = c.prepare(&sql).map_err(op)?;
            let mut rows = stmt.query([]).map_err(op)?;
            let mut out = vec![];
            while let Some(r) = rows.next().map_err(op)? {
                out.push(rule(r)?);
            }
            Ok(out)
        })
    }
    fn list_rules_for_event(&self, kind: EventKind) -> Result<Vec<Rule>, StorageError> {
        self.with_conn(|c|{let sql=format!("SELECT {RULE_SQL} FROM rules WHERE is_enabled=1 AND trigger_kind=?1 ORDER BY priority DESC,id ASC");let mut stmt=c.prepare(&sql).map_err(op)?;let mut rows=stmt.query([kind.as_str()]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(rule(r)?);}Ok(out)})
    }
    fn update_rule(&self, value: &Rule) -> Result<(), StorageError> {
        value.definition.validate().map_err(op)?;
        let json = serde_json::to_string(&value.definition).map_err(op)?;
        self.with_conn(|c|{let changed=c.execute("UPDATE rules SET name=?2,description=?3,is_enabled=?4,priority=?5,trigger_kind=?6,schema_version=?7,definition_json=?8,metadata_json=?9,updated_at=?10 WHERE id=?1",params![value.id.as_str(),value.name,value.description,value.enabled as i64,value.priority,value.definition.trigger.as_str(),value.definition.schema_version,json,value.metadata_json,value.updated_at.as_str()]).map_err(op)?;if changed==0{Err(op("rule not found"))}else{Ok(())}})
    }
    fn list_rule_executions(&self, limit: u32) -> Result<Vec<RuleExecutionRecord>, StorageError> {
        self.with_conn(|c|{let sql=format!("SELECT {RULE_EXECUTION_SQL} FROM rule_execution_history ORDER BY executed_at DESC,id DESC LIMIT ?1");let mut stmt=c.prepare(&sql).map_err(op)?;let mut rows=stmt.query([limit]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(rule_execution(r)?);}Ok(out)})
    }
    fn record_rule_executions(&self, records: &[RuleExecutionRecord]) -> Result<(), StorageError> {
        self.with_conn_mut(|conn| {
            let tx = conn.transaction().map_err(op)?;
            for record in records {
                insert_rule_execution(&tx, record)?;
            }
            tx.commit().map_err(op)?;
            Ok(())
        })
    }
    fn apply_rule_chain(
        &self,
        operations: &[RuleOperation],
        records: &[RuleExecutionRecord],
    ) -> Result<Vec<RuleOperation>, StorageError> {
        self.with_conn_mut(|conn|{
            let tx=conn.transaction().map_err(op)?;
            let mut stored_operations=operations.to_vec();
            for operation in &mut stored_operations {
                match operation {
                    RuleOperation::CompleteQuest{quest,expected_status,reward}=>{
                        let (owner,status,reward_amount):(String,String,i64)=tx.query_row("SELECT player_id,status,xp_reward FROM quests WHERE id=?1",[quest.id.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(op)?;
                        if owner!=quest.player_id.as_str()||quest.status!=QuestStatus::Completed||quest.progress!=100||quest.xp_reward!=reward_amount||status!=expected_status.as_str(){return Err(op("rule Quest completion no longer matches persisted state"));}
                        match reward {
                            Some((player,transaction))=>{
                                let current:i64=tx.query_row("SELECT current_xp FROM players WHERE id=?1",[player.id.as_str()],|r|r.get(0)).map_err(op)?;
                                let applied=transaction.applied_amount.ok_or_else(||op("rule quest reward is missing applied XP"))?;
                                if player.id!=quest.player_id||transaction.player_id!=player.id||transaction.resource!="xp"||transaction.amount!=quest.xp_reward||current.checked_add(applied)!=Some(player.current_xp){return Err(op("rule Quest reward does not match owner XP transition"));}
                                let stored=insert_transaction(&tx,transaction)?;transaction.id=stored.id;
                                if tx.execute("UPDATE players SET level=?2,current_xp=?3,updated_at=?4 WHERE id=?1",params![player.id.as_str(),player.level,player.current_xp,player.updated_at.as_str()]).map_err(op)?!=1{return Err(op("rule reward Player disappeared"));}
                            }
                            None if quest.xp_reward==0=>{},
                            None=>return Err(op("rewarded rule Quest completion must include its XP transaction")),
                        }
                        if tx.execute("UPDATE quests SET status='completed',progress=100,completed_at=?2,updated_at=?2 WHERE id=?1",params![quest.id.as_str(),quest.completed_at.as_ref().map(Iso8601Timestamp::as_str)]).map_err(op)?!=1{return Err(op("rule Quest disappeared during completion"));}
                    }
                    RuleOperation::PlayerXp{player,previous_xp,transaction}=>{
                        let current:i64=tx.query_row("SELECT current_xp FROM players WHERE id=?1",[player.id.as_str()],|r|r.get(0)).map_err(op)?;
                        let applied=transaction.applied_amount.ok_or_else(||op("rule XP action is missing its applied amount"))?;
                        if current!=*previous_xp||transaction.player_id!=player.id||transaction.resource!="xp"||current.checked_add(applied)!=Some(player.current_xp)||player.current_xp<0||transaction.amount==0||(transaction.amount>0&&applied!=transaction.amount)||(transaction.amount<0&&(applied>0||applied<transaction.amount)){return Err(op("rule XP action no longer matches persisted Player state"));}
                        let stored=insert_transaction(&tx,transaction)?;transaction.id=stored.id;
                        if tx.execute("UPDATE players SET level=?2,current_xp=?3,updated_at=?4 WHERE id=?1",params![player.id.as_str(),player.level,player.current_xp,player.updated_at.as_str()]).map_err(op)?!=1{return Err(op("rule Player disappeared during XP action"));}
                    }
                    RuleOperation::PlayerStat{stat,expected_previous}=>{
                        let current:Option<f64>=tx.query_row("SELECT current_value FROM player_stats WHERE player_id=?1 AND stat_code=?2",params![stat.player_id.as_str(),stat.stat_code],|r|r.get(0)).optional().map_err(op)?;
                        if current!=*expected_previous{return Err(op("rule stat action observed a stale value"));}
                        tx.execute("INSERT INTO player_stats(player_id,stat_code,current_value,metadata_json,updated_at) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(player_id,stat_code) DO UPDATE SET current_value=excluded.current_value,metadata_json=excluded.metadata_json,updated_at=excluded.updated_at",params![stat.player_id.as_str(),stat.stat_code,stat.current_value,stat.metadata_json,stat.updated_at.as_str()]).map_err(op)?;
                    }
                    RuleOperation::ConceptProgress{player_id,source,track,expected_previous,history}=>{
                        if history.concept_id!=track.concept_id||history.track_code!=track.track_code||history.previous_value!=*expected_previous||history.current_value!=track.current_value||history.level!=track.level{return Err(op("Concept progress operation and history disagree"));}
                        let owner:String=tx.query_row("SELECT player_id FROM concepts WHERE id=?1",[track.concept_id.as_str()],|r|r.get(0)).map_err(op)?;
                        let current:Option<(f64,String)>=tx.query_row("SELECT current_value,control FROM concept_progress_tracks WHERE concept_id=?1 AND track_code=?2",params![track.concept_id.as_str(),track.track_code],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(op)?;
                        if current.as_ref().map(|v|v.0)!=*expected_previous{return Err(op("Concept progress action observed a stale value"));}
                        match (source,current.as_ref()) { (lr_application::rules::ProgressMutationSource::Rule,Some((_,control))) if control!="rule_controlled"=>return Err(op("Rule action cannot overwrite a manual Concept track")), (lr_application::rules::ProgressMutationSource::Rule,None)=>return Err(op("Rule action requires an explicitly rule-controlled Concept track")), _=>{} }
                        if current.is_some(){tx.execute("UPDATE concept_progress_tracks SET current_value=?3,level=?4,is_active=?5,metadata_json=?6,updated_at=?7,level_name=?8,progression_label=?9 WHERE concept_id=?1 AND track_code=?2",params![track.concept_id.as_str(),track.track_code,track.current_value,track.level,track.is_active as i64,track.metadata_json,track.updated_at.as_str(),track.level_name,track.progression_label]).map_err(op)?;}else{tx.execute("INSERT INTO concept_progress_tracks(id,concept_id,track_code,current_value,level,is_active,metadata_json,created_at,updated_at,level_name,progression_label,control) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![track.id.as_str(),track.concept_id.as_str(),track.track_code,track.current_value,track.level,track.is_active as i64,track.metadata_json,track.created_at.as_str(),track.updated_at.as_str(),track.level_name,track.progression_label,track.control.as_str()]).map_err(op)?;}
                        if owner!=player_id.as_str(){return Err(op("Concept progress rule target belongs to another Player"));}
                        tx.execute("INSERT INTO concept_progress_history(id,concept_id,track_code,previous_value,current_value,level,occurred_at,captured_at,metadata_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![history.id.as_str(),history.concept_id.as_str(),history.track_code,history.previous_value,history.current_value,history.level,history.occurred_at.as_str(),history.captured_at.as_str(),history.metadata_json]).map_err(op)?;
                    }
                    RuleOperation::ResolveProgressSuggestion{suggestion_id,player_id,accepted_at}=>{
                        let (concept_id,track_code,value,created_at,status):(String,String,f64,String,String)=tx.query_row("SELECT concept_id,track_code,proposed_value,created_at,status FROM progress_suggestions WHERE id=?1 AND player_id=?2",params![suggestion_id.as_str(),player_id.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(op)?;
                        if status!="pending"{return Err(op("progress suggestion is not pending"));}
                        let recorded:i64=tx.query_row("SELECT count(*) FROM concept_progress_history WHERE concept_id=?1 AND track_code=?2 AND current_value=?3 AND occurred_at>=?4",params![concept_id,track_code,value,created_at],|r|r.get(0)).map_err(op)?;
                        if recorded==0{return Err(op("accepted suggestion has no matching progress history"));}
                        if tx.execute("UPDATE progress_suggestions SET status='accepted',resolved_at=?2 WHERE id=?1 AND player_id=?3 AND status='pending'",params![suggestion_id.as_str(),accepted_at.as_str(),player_id.as_str()]).map_err(op)?!=1{return Err(op("progress suggestion changed during atomic acceptance"));}
                    }
                }
            }
            for record in records{insert_rule_execution(&tx,record)?;}
            tx.commit().map_err(op)?;Ok(stored_operations)
        })
    }
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
        self.with_conn(|conn|{conn.execute("INSERT INTO players(id,name,description,level,current_xp,is_active,metadata_json,created_at,updated_at,level_name,progression_label)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![p.id.as_str(),p.name,p.description,p.level,p.current_xp,p.is_active as i64,p.metadata_json,p.created_at.as_str(),p.updated_at.as_str(),p.level_name,p.progression_label]).map_err(op)?;Ok(())})
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
        self.with_conn(|conn|{let changed=conn.execute("UPDATE players SET name=?2,description=?3,level=?4,current_xp=?5,is_active=?6,metadata_json=?7,updated_at=?8,level_name=?9,progression_label=?10 WHERE id=?1",params![p.id.as_str(),p.name,p.description,p.level,p.current_xp,p.is_active as i64,p.metadata_json,p.updated_at.as_str(),p.level_name,p.progression_label]).map_err(op)?;if changed==0{Err(op("player not found"))}else{Ok(())}})
    }

    fn create_stat_definition(&self, d: &StatDefinition) -> Result<(), StorageError> {
        self.with_conn(|conn| { conn.execute("INSERT INTO player_stat_definitions(id,code,name,description,unit,minimum,maximum,is_active,metadata_json,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)", params![d.id.as_str(),d.code,d.name,d.description,d.unit,d.minimum,d.maximum,d.is_active as i64,d.metadata_json,d.created_at.as_str(),d.updated_at.as_str()]).map_err(op)?; Ok(()) })
    }
    fn list_stat_definitions(&self) -> Result<Vec<StatDefinition>, StorageError> {
        self.with_conn(|conn| { let mut stmt=conn.prepare("SELECT id,code,name,description,unit,minimum,maximum,is_active,metadata_json,created_at,updated_at FROM player_stat_definitions ORDER BY code").map_err(op)?; let mut rows=stmt.query([]).map_err(op)?; let mut out=vec![]; while let Some(row)=rows.next().map_err(op)? { out.push(stat_definition(row)?); } Ok(out) })
    }
    fn set_player_stat(&self, s: &PlayerStat) -> Result<(), StorageError> {
        self.with_conn(|conn| { conn.execute("INSERT INTO player_stats(player_id,stat_code,current_value,metadata_json,updated_at) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(player_id,stat_code) DO UPDATE SET current_value=excluded.current_value,metadata_json=excluded.metadata_json,updated_at=excluded.updated_at", params![s.player_id.as_str(),s.stat_code,s.current_value,s.metadata_json,s.updated_at.as_str()]).map_err(op)?; Ok(()) })
    }
    fn list_player_stats(&self, player_id: &EntityId) -> Result<Vec<PlayerStat>, StorageError> {
        self.with_conn(|conn| { let mut stmt=conn.prepare("SELECT player_id,stat_code,current_value,metadata_json,updated_at FROM player_stats WHERE player_id=?1 ORDER BY stat_code").map_err(op)?; let mut rows=stmt.query([player_id.as_str()]).map_err(op)?; let mut out=vec![]; while let Some(row)=rows.next().map_err(op)? { out.push(player_stat(row)?); } Ok(out) })
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
        self.with_conn(|conn|{conn.execute("INSERT INTO skills(id,skill_tree_id,parent_skill_id,skill_type_namespace,skill_type_code,name,description,story,instructions,level,current_xp,invested_minutes,status,started_at,completed_at,metadata_json,created_at,updated_at,level_name,progression_label)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",params![s.id.as_str(),s.skill_tree_id.as_str(),s.parent_skill_id.as_ref().map(EntityId::as_str),s.skill_type.namespace,s.skill_type.code,s.name,s.description,s.story,s.instructions,s.level,s.current_xp,s.invested_minutes,s.status.as_str(),s.started_at.as_ref().map(Iso8601Timestamp::as_str),s.completed_at.as_ref().map(Iso8601Timestamp::as_str),s.metadata_json,s.created_at.as_str(),s.updated_at.as_str(),s.level_name,s.progression_label]).map_err(op)?;Ok(())})
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
        self.with_conn(|conn|{let changed=conn.execute("UPDATE skills SET parent_skill_id=?2,name=?3,description=?4,story=?5,instructions=?6,level=?7,current_xp=?8,invested_minutes=?9,status=?10,started_at=?11,completed_at=?12,metadata_json=?13,updated_at=?14,level_name=?15,progression_label=?16 WHERE id=?1",params![s.id.as_str(),s.parent_skill_id.as_ref().map(EntityId::as_str),s.name,s.description,s.story,s.instructions,s.level,s.current_xp,s.invested_minutes,s.status.as_str(),s.started_at.as_ref().map(Iso8601Timestamp::as_str),s.completed_at.as_ref().map(Iso8601Timestamp::as_str),s.metadata_json,s.updated_at.as_str(),s.level_name,s.progression_label]).map_err(op)?;if changed==0{Err(op("skill not found"))}else{Ok(())}})
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
        self.with_conn(|conn|{let target_kind=if e.target_concept_id.is_some(){"concept"}else{"player"};conn.execute("INSERT INTO effects(id,player_id,effect_type_namespace,effect_type_code,name,description,started_at,expires_at,deactivated_at,intensity,source_kind,source_id,metadata_json,created_at,updated_at,target_kind,target_concept_id)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",params![e.id.as_str(),e.player_id.as_str(),e.effect_type.namespace,e.effect_type.code,e.name,e.description,e.started_at.as_str(),e.expires_at.as_ref().map(Iso8601Timestamp::as_str),e.deactivated_at.as_ref().map(Iso8601Timestamp::as_str),e.intensity,e.source_kind,e.source_id,e.metadata_json,e.created_at.as_str(),e.updated_at.as_str(),target_kind,e.target_concept_id.as_ref().map(EntityId::as_str)]).map_err(op)?;Ok(())})
    }
    fn list_effects(
        &self,
        p: &EntityId,
        active_at: Option<&str>,
    ) -> Result<Vec<Effect>, StorageError> {
        self.with_conn(|conn|{let sql=format!("SELECT {EFFECT_SQL} FROM effects WHERE player_id=?1 AND (?2 IS NULL OR (started_at<=?2 AND (expires_at IS NULL OR expires_at>?2) AND (deactivated_at IS NULL OR deactivated_at>?2))) ORDER BY started_at DESC");let mut stmt=conn.prepare(&sql).map_err(op)?;let mut rows=stmt.query(params![p.as_str(),active_at]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(effect(r)?);}Ok(out)})
    }
    fn deactivate_effect(
        &self,
        effect_id: &EntityId,
        at: &Iso8601Timestamp,
    ) -> Result<Effect, StorageError> {
        self.with_conn_mut(|conn| { let tx=conn.transaction().map_err(op)?; let sql=format!("SELECT {EFFECT_SQL} FROM effects WHERE id=?1"); let current={let mut stmt=tx.prepare(&sql).map_err(op)?;let mut rows=stmt.query([effect_id.as_str()]).map_err(op)?;match rows.next().map_err(op)?{Some(r)=>effect(r)?,None=>return Err(op("effect not found"))}}; let mut changed=current; changed.deactivate(at.clone()).map_err(op)?; let n=tx.execute("UPDATE effects SET deactivated_at=?2,updated_at=?2 WHERE id=?1 AND deactivated_at IS NULL",params![effect_id.as_str(),at.as_str()]).map_err(op)?; if n!=1{return Err(op("effect is already deactivated"));} tx.commit().map_err(op)?; Ok(changed) })
    }

    fn append_transaction(&self, value: &Transaction) -> Result<Transaction, StorageError> {
        if value.resource == "xp" {
            return Err(op(
                "XP history must be appended through an atomic XP operation",
            ));
        }
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
        self.with_conn(|conn|conn.query_row("SELECT COALESCE(SUM(CASE WHEN resource='xp' THEN COALESCE(applied_amount,amount) ELSE amount END),0) FROM transactions WHERE player_id=?1 AND resource=?2",params![p.as_str(),resource],|r|r.get(0)).map_err(op))
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
    fn capture_player_snapshot(
        &self,
        player_id: &EntityId,
        snapshot_date: &DateValue,
        created_at: &Iso8601Timestamp,
    ) -> Result<PlayerStateSnapshot, StorageError> {
        self.with_conn_mut(|conn| {
            let tx = conn.transaction().map_err(op)?;
            let mut stmt=tx.prepare("SELECT level,current_xp FROM players WHERE id=?1").map_err(op)?;
            let p=stmt.query_row([player_id.as_str()], |r| Ok((r.get::<_,i32>(0)?,r.get::<_,i64>(1)?))).map_err(op)?;
            drop(stmt);
            let stats={let mut stmt=tx.prepare("SELECT d.code,d.name,d.description,d.unit,d.minimum,d.maximum,d.metadata_json,s.current_value FROM player_stats s JOIN player_stat_definitions d ON d.code=s.stat_code WHERE s.player_id=?1 AND d.is_active=1 ORDER BY d.code").map_err(op)?;let rows=stmt.query_map([player_id.as_str()],|r| Ok(serde_json::json!({"code":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"description":r.get::<_,Option<String>>(2)?,"unit":r.get::<_,Option<String>>(3)?,"minimum":r.get::<_,Option<f64>>(4)?,"maximum":r.get::<_,Option<f64>>(5)?,"metadataJson":r.get::<_,String>(6)?,"value":r.get::<_,f64>(7)?}))).map_err(op)?;let values=rows.collect::<Result<Vec<_>,_>>().map_err(op)?;values};
            let effects={let mut stmt=tx.prepare("SELECT id,effect_type_code,name,description,started_at,expires_at,intensity,source_kind,source_id,metadata_json FROM effects WHERE player_id=?1 AND started_at<=?2 AND (expires_at IS NULL OR expires_at>?2) AND (deactivated_at IS NULL OR deactivated_at>?2) ORDER BY id").map_err(op)?;let rows=stmt.query_map(params![player_id.as_str(),created_at.as_str()],|r| Ok(serde_json::json!({"id":r.get::<_,String>(0)?,"typeCode":r.get::<_,String>(1)?,"name":r.get::<_,String>(2)?,"description":r.get::<_,Option<String>>(3)?,"startedAt":r.get::<_,String>(4)?,"expiresAt":r.get::<_,Option<String>>(5)?,"intensity":r.get::<_,i32>(6)?,"sourceKind":r.get::<_,Option<String>>(7)?,"sourceId":r.get::<_,Option<String>>(8)?,"metadataJson":r.get::<_,String>(9)?}))).map_err(op)?;let values=rows.collect::<Result<Vec<_>,_>>().map_err(op)?;values};
            let state=serde_json::json!({"schemaVersion":1,"stats":stats,"activeEffects":effects});
            let state_json=serde_json::to_string(&state).map_err(op)?;
            let mut value=PlayerStateSnapshot::new(player_id.clone(),snapshot_date.clone(),p.0,p.1,created_at.clone()).map_err(op)?.with_state_json(state_json);
            tx.execute("INSERT INTO player_state_snapshots(player_id,snapshot_date,level,current_xp,state_json,metadata_json,created_at) VALUES (?1,?2,?3,?4,?5,'{}',?6)",params![player_id.as_str(),snapshot_date.as_str(),value.level,value.current_xp,value.state_json,created_at.as_str()]).map_err(op)?;
            value.id=Some(tx.last_insert_rowid());
            tx.commit().map_err(op)?;
            Ok(value)
        })
    }
    fn insert_skill_snapshot(&self, s: &SkillStateSnapshot) -> Result<(), StorageError> {
        self.with_conn(|conn|{conn.execute("INSERT INTO skill_state_snapshots(skill_id,snapshot_date,level,current_xp,status,invested_minutes,state_json,metadata_json,created_at)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![s.skill_id.as_str(),s.snapshot_date.as_str(),s.level,s.current_xp,s.status.as_str(),s.invested_minutes,s.state_json,s.metadata_json,s.created_at.as_str()]).map_err(op)?;Ok(())})
    }
    fn capture_skill_snapshot(
        &self,
        skill_id: &EntityId,
        snapshot_date: &DateValue,
        created_at: &Iso8601Timestamp,
    ) -> Result<SkillStateSnapshot, StorageError> {
        self.with_conn_mut(|conn| {
            let tx=conn.transaction().map_err(op)?;
            let row=tx.query_row("SELECT skill_tree_id,parent_skill_id,level,current_xp,invested_minutes,status FROM skills WHERE id=?1",[skill_id.as_str()],|r| Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?,r.get::<_,i32>(2)?,r.get::<_,i64>(3)?,r.get::<_,i64>(4)?,r.get::<_,String>(5)?))).map_err(op)?;
            let state=serde_json::json!({"schemaVersion":1,"skillTreeId":row.0,"parentSkillId":row.1});
            let state_json=serde_json::to_string(&state).map_err(op)?;
            let mut value=SkillStateSnapshot::new(skill_id.clone(),snapshot_date.clone(),row.2,row.3,skill_status(row.5)?,row.4,created_at.clone()).map_err(op)?;
            value.state_json=state_json;
            tx.execute("INSERT INTO skill_state_snapshots(skill_id,snapshot_date,level,current_xp,status,invested_minutes,state_json,metadata_json,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,'{}',?8)",params![skill_id.as_str(),snapshot_date.as_str(),value.level,value.current_xp,value.status.as_str(),value.invested_minutes,value.state_json,created_at.as_str()]).map_err(op)?;
            value.id=Some(tx.last_insert_rowid());
            tx.commit().map_err(op)?;
            Ok(value)
        })
    }
    fn list_skill_snapshots(
        &self,
        skill_id: &EntityId,
    ) -> Result<Vec<SkillStateSnapshot>, StorageError> {
        self.with_conn(|conn| { let mut stmt=conn.prepare("SELECT id,skill_id,snapshot_date,level,current_xp,status,invested_minutes,state_json,metadata_json,created_at FROM skill_state_snapshots WHERE skill_id=?1 ORDER BY snapshot_date").map_err(op)?; let mut rows=stmt.query([skill_id.as_str()]).map_err(op)?; let mut out=vec![]; while let Some(r)=rows.next().map_err(op)? { out.push(SkillStateSnapshot{id:Some(r.get(0).map_err(op)?),skill_id:id(r.get(1).map_err(op)?)?,snapshot_date:date(r.get(2).map_err(op)?)?,level:r.get(3).map_err(op)?,current_xp:r.get(4).map_err(op)?,status:skill_status(r.get(5).map_err(op)?)?,invested_minutes:r.get(6).map_err(op)?,state_json:r.get(7).map_err(op)?,metadata_json:r.get(8).map_err(op)?,created_at:timestamp(r.get(9).map_err(op)?)?}); } Ok(out) })
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
        self.with_conn(|conn|{conn.execute("INSERT INTO narrative_entries(id,player_id,kind_namespace,kind_code,title,content,author,source_kind,source_id,metadata_json,created_at,updated_at,is_active)VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![n.id.as_str(),n.player_id.as_str(),n.kind.namespace,n.kind.code,n.title,n.content,n.author,n.source_kind,n.source_id,n.metadata_json,n.created_at.as_str(),n.updated_at.as_str(),n.is_active as i64]).map_err(op)?;Ok(())})
    }
    fn get_narrative_entry(&self, id: &EntityId) -> Result<Option<NarrativeEntry>, StorageError> {
        self.with_conn(|conn| {
            let sql = format!("SELECT {NARRATIVE_SQL} FROM narrative_entries WHERE id=?1");
            let mut stmt = conn.prepare(&sql).map_err(op)?;
            let mut rows = stmt.query([id.as_str()]).map_err(op)?;
            rows.next().map_err(op)?.map(narrative).transpose()
        })
    }
    fn update_narrative_entry(&self, n: &NarrativeEntry) -> Result<(), StorageError> {
        self.with_conn(|conn| { let changed=conn.execute("UPDATE narrative_entries SET kind_namespace=?3,kind_code=?4,title=?5,content=?6,author=?7,source_kind=?8,source_id=?9,is_active=?10,metadata_json=?11,updated_at=?12 WHERE id=?1 AND player_id=?2",params![n.id.as_str(),n.player_id.as_str(),n.kind.namespace,n.kind.code,n.title,n.content,n.author,n.source_kind,n.source_id,n.is_active as i64,n.metadata_json,n.updated_at.as_str()]).map_err(op)?; if changed==1 {Ok(())} else {Err(op("content record not found in Player world"))} })
    }
    fn list_narrative_entries(&self, p: &EntityId) -> Result<Vec<NarrativeEntry>, StorageError> {
        self.with_conn(|conn|{let sql=format!("SELECT {NARRATIVE_SQL} FROM narrative_entries WHERE player_id=?1 ORDER BY created_at DESC");let mut stmt=conn.prepare(&sql).map_err(op)?;let mut rows=stmt.query([p.as_str()]).map_err(op)?;let mut out=vec![];while let Some(r)=rows.next().map_err(op)?{out.push(narrative(r)?);}Ok(out)})
    }

    fn award_xp(&self, p: &Player, value: &Transaction) -> Result<Transaction, StorageError> {
        self.with_conn_mut(|conn| {
            let tx = conn.transaction().map_err(op)?;
            let current: i64 = tx
                .query_row(
                    "SELECT current_xp FROM players WHERE id=?1",
                    [p.id.as_str()],
                    |r| r.get(0),
                )
                .map_err(op)?;
            let applied = value
                .applied_amount
                .ok_or_else(|| op("XP event must record its applied amount"))?;
            if value.player_id != p.id
                || value.resource != "xp"
                || current.checked_add(applied) != Some(p.current_xp)
                || p.current_xp < 0
            {
                return Err(op(
                    "XP transaction does not match the Player state transition",
                ));
            }
            if (value.amount > 0 && applied != value.amount)
                || (value.amount < 0 && (applied > 0 || applied < value.amount))
                || value.amount == 0
            {
                return Err(op("requested and applied XP amounts are inconsistent"));
            }
            let stored = insert_transaction(&tx, value)?;
            let changed = tx
                .execute(
                    "UPDATE players SET level=?2,current_xp=?3,updated_at=?4 WHERE id=?1",
                    params![p.id.as_str(), p.level, p.current_xp, p.updated_at.as_str()],
                )
                .map_err(op)?;
            if changed != 1 {
                return Err(op("player not found"));
            }
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
        self.with_conn_mut(|conn| {
            let tx=conn.transaction().map_err(op)?;
            let owner:String=tx.query_row("SELECT player_id FROM quests WHERE id=?1",[q.id.as_str()],|r|r.get(0)).map_err(op)?;
            if owner!=q.player_id.as_str(){return Err(op("quest owner cannot change"));}
            let created=match(p,reward){
                (Some(player),Some(reward))=>{
                    let current:i64=tx.query_row("SELECT current_xp FROM players WHERE id=?1",[player.id.as_str()],|r|r.get(0)).map_err(op)?;
                    let applied=reward.applied_amount.ok_or_else(||op("XP reward must record its applied amount"))?;
                    if player.id!=q.player_id||reward.player_id!=player.id||reward.resource!="xp"||reward.amount!=q.xp_reward||current.checked_add(applied)!=Some(player.current_xp)||player.current_xp<0 {
                        return Err(op("quest reward does not match its owner's state transition"));
                    }
                    let stored=insert_transaction(&tx,reward)?;
                    if tx.execute("UPDATE players SET level=?2,current_xp=?3,updated_at=?4 WHERE id=?1",params![player.id.as_str(),player.level,player.current_xp,player.updated_at.as_str()]).map_err(op)?!=1{return Err(op("reward player not found"));}
                    Some(stored)
                },
                (None, None) if q.xp_reward == 0 => None,
                (None, None) => return Err(op("quest with an XP reward must provide its Player and ledger event")),
                _=>return Err(op("reward player and transaction must be supplied together")),
            };
            let changed=tx.execute("UPDATE quests SET status=?2,progress=?3,started_at=?4,completed_at=?5,updated_at=?6 WHERE id=?1",params![q.id.as_str(),q.status.as_str(),q.progress,q.started_at.as_ref().map(Iso8601Timestamp::as_str),q.completed_at.as_ref().map(Iso8601Timestamp::as_str),q.updated_at.as_str()]).map_err(op)?;
            if changed!=1{return Err(op("quest not found"));}
            tx.commit().map_err(op)?;
            Ok(created)
        })
    }
    fn invest_skill_time(
        &self,
        s: &Skill,
        value: &Transaction,
    ) -> Result<Transaction, StorageError> {
        self.with_conn_mut(|conn| {
            let tx = conn.transaction().map_err(op)?;
            let current:i64=tx.query_row("SELECT invested_minutes FROM skills WHERE id=?1",[s.id.as_str()],|r|r.get(0)).map_err(op)?;
            let owner:String=tx.query_row("SELECT t.player_id FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE s.id=?1",[s.id.as_str()],|r|r.get(0)).map_err(op)?;
            if value.player_id.as_str()!=owner || value.resource!="minutes" || value.amount!=s.invested_minutes-current || value.amount<=0 {
                return Err(op("time transaction does not match the Skill state transition"));
            }
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
    use lr_application::{
        Clock, Comparison, EventKind, NumericSubject, RuleAction, RuleCondition, WorldService,
        WorldStore,
    };
    use lr_domain::EffectLifecycle;
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
    struct FrozenClock;
    impl Clock for FrozenClock {
        fn now_rfc3339(&self) -> String {
            T0.into()
        }
        fn now_unix_nanos(&self) -> u128 {
            123
        }
    }

    #[test]
    fn quest_description_survives_creation_and_reload() {
        let store = std::sync::Arc::new(store());
        let service = WorldService::new(store.clone(), FrozenClock);
        let player = service.create_player("Ada", None).unwrap();
        let description = "Prepare the soil before planting the seedlings.";

        let created = service
            .create_quest(
                player.id.as_str(),
                "main",
                "Prepare the garden",
                Some(description.into()),
                None,
                None,
                None,
                Some(0),
            )
            .unwrap();
        let reloaded = store.get_quest(&created.id).unwrap().unwrap();

        assert_eq!(reloaded.description.as_deref(), Some(description));
    }

    #[test]
    fn declarative_rules_chain_atomically_in_priority_order_and_persist_audit() {
        let store = std::sync::Arc::new(store());
        let service = WorldService::new(store.clone(), FrozenClock);
        let player = service.create_player("Ada", None).unwrap();
        let eq = |value| RuleCondition::NumberCompare {
            subject: NumericSubject::CurrentXp,
            comparison: Comparison::Equal,
            value,
        };
        service
            .create_rule(
                "first",
                None,
                20,
                EventKind::PlayerXpChanged,
                eq(10.0),
                vec![RuleAction::AwardXp {
                    amount: 10,
                    reason: Some("first_rule".into()),
                }],
            )
            .unwrap();
        service
            .create_rule(
                "second",
                None,
                10,
                EventKind::PlayerXpChanged,
                eq(20.0),
                vec![RuleAction::AwardXp {
                    amount: 5,
                    reason: Some("second_rule".into()),
                }],
            )
            .unwrap();

        let result = service
            .award_xp(player.id.as_str(), 10, Some("root".into()), None)
            .unwrap();
        assert_eq!(result.player.current_xp, 25);
        assert_eq!(
            result.transaction.amount, 10,
            "the API returns the source ledger entry"
        );
        assert_eq!(store.transaction_total(&player.id, "xp").unwrap(), 25);
        let ledger = store.list_transactions(&player.id, 20).unwrap();
        assert_eq!(ledger.len(), 3);
        assert_eq!(
            ledger
                .iter()
                .map(|t| t.applied_amount.unwrap())
                .sum::<i64>(),
            25
        );
        let history = service.list_rule_executions(100).unwrap();
        assert_eq!(
            history.len(),
            6,
            "both active rules are audited for each of three event states"
        );
        assert!(history
            .iter()
            .all(|r| r.status == "succeeded" || r.status == "condition_failed"));
        assert!(history.iter().all(|r| r.chain_id == history[0].chain_id));
        assert_eq!(
            service
                .list_rules()
                .unwrap()
                .iter()
                .map(|r| r.priority)
                .collect::<Vec<_>>(),
            vec![20, 10]
        );
        let disabled = service
            .set_rule_enabled(&service.list_rules().unwrap()[0].id.to_string(), false)
            .unwrap();
        assert!(!disabled.enabled);
        let before = service.list_rule_executions(100).unwrap().len();
        service.award_xp(player.id.as_str(), 1, None, None).unwrap();
        let after = service.list_rule_executions(100).unwrap();
        assert_eq!(after.len(), before + 1);
        assert!(after
            .iter()
            .filter(|record| record.chain_id != history[0].chain_id)
            .all(|record| record.rule_id != disabled.id.to_string()));
    }

    #[test]
    fn stat_actions_emit_follow_on_events_and_commit_with_the_root_stat_write() {
        let store = std::sync::Arc::new(store());
        let service = WorldService::new(store.clone(), FrozenClock);
        let player = service.create_player("Ada", None).unwrap();
        service
            .define_stat("focus", "Focus", None, None, Some(0.0), Some(10.0))
            .unwrap();
        let stat_eq = |value| RuleCondition::NumberCompare {
            subject: NumericSubject::StatValue,
            comparison: Comparison::Equal,
            value,
        };
        service
            .create_rule(
                "increase focus",
                None,
                10,
                EventKind::StatChanged,
                stat_eq(5.0),
                vec![
                    RuleAction::ModifyPlayerStat {
                        stat_code: "focus".into(),
                        delta: 2.0,
                    },
                    RuleAction::AwardXp {
                        amount: 2,
                        reason: Some("second action".into()),
                    },
                ],
            )
            .unwrap();
        service
            .create_rule(
                "reward focus",
                None,
                0,
                EventKind::StatChanged,
                stat_eq(7.0),
                vec![RuleAction::AwardXp {
                    amount: 3,
                    reason: None,
                }],
            )
            .unwrap();
        let stat = service
            .set_player_stat(player.id.as_str(), "focus", 5.0)
            .unwrap();
        assert_eq!(stat.current_value, 7.0);
        assert_eq!(
            service
                .get_player(player.id.as_str())
                .unwrap()
                .unwrap()
                .current_xp,
            5
        );
        assert_eq!(
            store.list_player_stats(&player.id).unwrap()[0].current_value,
            7.0
        );
        assert_eq!(store.transaction_total(&player.id, "xp").unwrap(), 5);
        assert_eq!(service.list_rule_executions(100).unwrap().len(), 4);
    }

    #[test]
    fn recursive_rules_are_stopped_without_partial_player_or_ledger_writes() {
        let store = std::sync::Arc::new(store());
        let service = WorldService::new(store.clone(), FrozenClock);
        let player = service.create_player("Ada", None).unwrap();
        service
            .create_rule(
                "unbounded XP recursion",
                None,
                0,
                EventKind::PlayerXpChanged,
                RuleCondition::Always,
                vec![RuleAction::AwardXp {
                    amount: 1,
                    reason: None,
                }],
            )
            .unwrap();
        let error = service
            .award_xp(player.id.as_str(), 1, None, None)
            .unwrap_err();
        assert!(
            matches!(
                error,
                lr_application::AppError::RuleExecution(
                    lr_application::RuleExecutionError::MaxDepth(_)
                )
            ),
            "unexpected rule guard: {error:?}"
        );
        assert_eq!(
            service
                .get_player(player.id.as_str())
                .unwrap()
                .unwrap()
                .current_xp,
            0
        );
        assert_eq!(store.transaction_total(&player.id, "xp").unwrap(), 0);
        let history = service.list_rule_executions(100).unwrap();
        assert!(history.iter().any(|r| r.status == "guard_aborted"));
        assert!(history
            .iter()
            .filter(|r| r.status == "guard_aborted")
            .all(|r| r.error.is_some()));
    }

    #[test]
    fn completed_quest_reward_and_matching_rule_xp_share_one_audited_chain() {
        let store = std::sync::Arc::new(store());
        let service = WorldService::new(store.clone(), FrozenClock);
        let player = service.create_player("Ada", None).unwrap();
        let quest = service
            .create_quest(
                player.id.as_str(),
                "main",
                "Ship",
                None,
                None,
                None,
                None,
                Some(5),
            )
            .unwrap();
        service
            .create_rule(
                "main quest bonus",
                None,
                0,
                EventKind::QuestCompleted,
                RuleCondition::TextCompare {
                    subject: lr_application::TextSubject::QuestType,
                    comparison: lr_application::TextComparison::Equal,
                    value: "main".into(),
                },
                vec![RuleAction::AwardXp {
                    amount: 100,
                    reason: None,
                }],
            )
            .unwrap();
        service
            .create_rule(
                "large XP follow-up",
                None,
                0,
                EventKind::PlayerXpChanged,
                RuleCondition::NumberCompare {
                    subject: NumericSubject::AppliedAmount,
                    comparison: Comparison::GreaterOrEqual,
                    value: 100.0,
                },
                vec![RuleAction::AwardXp {
                    amount: 20,
                    reason: None,
                }],
            )
            .unwrap();
        let completed = service.complete_quest(quest.id.as_str()).unwrap();
        assert_eq!(completed.status, QuestStatus::Completed);
        assert_eq!(
            service
                .get_player(player.id.as_str())
                .unwrap()
                .unwrap()
                .current_xp,
            125
        );
        assert_eq!(store.transaction_total(&player.id, "xp").unwrap(), 125);
        let history = service.list_rule_executions(100).unwrap();
        assert_eq!(
            history.len(),
            4,
            "Quest completion triggers XP; that XP event triggers another rule and a final nonmatching evaluation"
        );
        assert_eq!(
            history
                .iter()
                .filter(|record| record.status == "succeeded")
                .count(),
            2
        );
        assert_eq!(
            history
                .iter()
                .filter(|record| record.status == "condition_failed")
                .count(),
            2
        );
    }

    #[test]
    fn invalid_rule_action_rolls_back_root_state_and_records_failure() {
        let store = std::sync::Arc::new(store());
        let service = WorldService::new(store.clone(), FrozenClock);
        let player = service.create_player("Ada", None).unwrap();
        service
            .define_stat("focus", "Focus", None, None, Some(0.0), Some(10.0))
            .unwrap();
        service
            .create_rule(
                "out of bounds",
                None,
                0,
                EventKind::StatChanged,
                RuleCondition::Always,
                vec![
                    RuleAction::AwardXp {
                        amount: 7,
                        reason: None,
                    },
                    RuleAction::SetPlayerStat {
                        stat_code: "focus".into(),
                        value: 20.0,
                    },
                ],
            )
            .unwrap();
        let error = service
            .set_player_stat(player.id.as_str(), "focus", 5.0)
            .unwrap_err();
        assert!(
            matches!(
                error,
                lr_application::AppError::RuleExecution(
                    lr_application::RuleExecutionError::ActionFailed { .. }
                )
            ),
            "unexpected rule error: {error:?}"
        );
        assert!(
            store.list_player_stats(&player.id).unwrap().is_empty(),
            "root and derived state writes roll back together"
        );
        assert_eq!(
            service
                .get_player(player.id.as_str())
                .unwrap()
                .unwrap()
                .current_xp,
            0
        );
        assert_eq!(
            store.transaction_total(&player.id, "xp").unwrap(),
            0,
            "earlier planned action also rolls back"
        );
        let history = service.list_rule_executions(100).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].status, "failed");
        assert!(history[0].error.is_some());
    }

    #[test]
    fn declarative_xp_penalty_respects_floor_and_preserves_requested_amount() {
        let store = std::sync::Arc::new(store());
        let service = WorldService::new(store.clone(), FrozenClock);
        let player = service.create_player("Ada", None).unwrap();
        service
            .create_rule(
                "penalize low XP",
                None,
                0,
                EventKind::PlayerXpChanged,
                RuleCondition::NumberCompare {
                    subject: NumericSubject::CurrentXp,
                    comparison: Comparison::Greater,
                    value: 0.0,
                },
                vec![RuleAction::AwardXp {
                    amount: -10,
                    reason: Some("penalty".into()),
                }],
            )
            .unwrap();
        service.award_xp(player.id.as_str(), 5, None, None).unwrap();
        assert_eq!(
            service
                .get_player(player.id.as_str())
                .unwrap()
                .unwrap()
                .current_xp,
            0
        );
        let ledger = store.list_transactions(&player.id, 10).unwrap();
        let penalty = ledger.iter().find(|tx| tx.amount == -10).unwrap();
        assert_eq!(penalty.applied_amount, Some(-5));
        assert_eq!(store.transaction_total(&player.id, "xp").unwrap(), 0);
    }

    #[test]
    fn action_and_evaluation_budgets_abort_before_any_world_write() {
        let store = std::sync::Arc::new(store());
        let service = WorldService::new(store.clone(), FrozenClock);
        let player = service.create_player("Ada", None).unwrap();
        let actions = vec![
            RuleAction::AwardXp {
                amount: 1,
                reason: None
            };
            lr_application::MAX_ACTIONS_PER_RULE
        ];
        for n in 0..3 {
            service
                .create_rule(
                    &format!("action budget {n}"),
                    None,
                    n,
                    EventKind::PlayerXpChanged,
                    RuleCondition::Always,
                    actions.clone(),
                )
                .unwrap();
        }
        let error = service
            .award_xp(player.id.as_str(), 1, None, None)
            .unwrap_err();
        assert!(matches!(
            error,
            lr_application::AppError::RuleExecution(
                lr_application::RuleExecutionError::MaxActions(_)
            )
        ));
        assert_eq!(
            service
                .get_player(player.id.as_str())
                .unwrap()
                .unwrap()
                .current_xp,
            0
        );
        assert_eq!(store.transaction_total(&player.id, "xp").unwrap(), 0);

        let store = std::sync::Arc::new(SqliteHealthStore::open_in_memory(T0));
        let service = WorldService::new(store.clone(), FrozenClock);
        let player = service.create_player("Grace", None).unwrap();
        for n in 0..=lr_application::MAX_RULE_EVALUATIONS_PER_CHAIN {
            service
                .create_rule(
                    &format!("evaluation budget {n}"),
                    None,
                    n as i32,
                    EventKind::PlayerXpChanged,
                    RuleCondition::NumberCompare {
                        subject: NumericSubject::CurrentXp,
                        comparison: Comparison::Less,
                        value: 0.0,
                    },
                    vec![RuleAction::AwardXp {
                        amount: 1,
                        reason: None,
                    }],
                )
                .unwrap();
        }
        let error = service
            .award_xp(player.id.as_str(), 1, None, None)
            .unwrap_err();
        assert!(matches!(
            error,
            lr_application::AppError::RuleExecution(
                lr_application::RuleExecutionError::MaxEvaluations(_)
            )
        ));
        assert_eq!(
            service
                .get_player(player.id.as_str())
                .unwrap()
                .unwrap()
                .current_xp,
            0
        );
        assert_eq!(store.transaction_total(&player.id, "xp").unwrap(), 0);
        assert_eq!(
            service.list_rule_executions(100).unwrap().len(),
            lr_application::MAX_RULE_EVALUATIONS_PER_CHAIN + 1
        );
    }

    #[test]
    fn rules_and_execution_history_survive_database_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules.sqlite3");
        let player_id;
        {
            let store = std::sync::Arc::new(SqliteHealthStore::open_file(&path, T0));
            let service = WorldService::new(store, FrozenClock);
            let player = service.create_player("Ada", None).unwrap();
            player_id = player.id.to_string();
            service
                .create_rule(
                    "small bonus",
                    None,
                    0,
                    EventKind::PlayerXpChanged,
                    RuleCondition::NumberCompare {
                        subject: NumericSubject::CurrentXp,
                        comparison: Comparison::Equal,
                        value: 1.0,
                    },
                    vec![RuleAction::AwardXp {
                        amount: 2,
                        reason: None,
                    }],
                )
                .unwrap();
            service.award_xp(&player_id, 1, None, None).unwrap();
        }
        let reopened = std::sync::Arc::new(SqliteHealthStore::open_file(&path, T0));
        let service = WorldService::new(reopened, FrozenClock);
        assert_eq!(
            service.get_player(&player_id).unwrap().unwrap().current_xp,
            3
        );
        assert_eq!(service.list_rules().unwrap().len(), 1);
        assert_eq!(service.list_rule_executions(100).unwrap().len(), 2);
    }

    #[test]
    fn repeated_rule_event_fingerprint_is_detected_and_rolled_back() {
        let store = std::sync::Arc::new(store());
        let service = WorldService::new(store.clone(), FrozenClock);
        let player = service.create_player("Ada", None).unwrap();
        service
            .define_stat("focus", "Focus", None, None, Some(0.0), Some(10.0))
            .unwrap();
        service
            .create_rule(
                "repeat same stat value",
                None,
                0,
                EventKind::StatChanged,
                RuleCondition::Always,
                vec![RuleAction::SetPlayerStat {
                    stat_code: "focus".into(),
                    value: 5.0,
                }],
            )
            .unwrap();
        let error = service
            .set_player_stat(player.id.as_str(), "focus", 5.0)
            .unwrap_err();
        assert!(
            matches!(
                error,
                lr_application::AppError::RuleExecution(
                    lr_application::RuleExecutionError::LoopDetected(_, _)
                )
            ),
            "unexpected rule guard: {error:?}"
        );
        assert!(store.list_player_stats(&player.id).unwrap().is_empty());
        assert!(service
            .list_rule_executions(100)
            .unwrap()
            .iter()
            .any(|r| r.status == "guard_aborted"));
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

    #[test]
    fn snapshots_capture_stats_effects_and_skill_state_atomically() {
        let store = store();
        let p = player_value();
        store.create_player(&p).unwrap();
        let definition = StatDefinition::new(
            EntityId::new("stat-focus").unwrap(),
            "focus",
            "Focus",
            None,
            Some("points".into()),
            Some(0.0),
            Some(10.0),
            now(),
        )
        .unwrap();
        store.create_stat_definition(&definition).unwrap();
        let stat = PlayerStat::new(p.id.clone(), &definition, 7.5, now()).unwrap();
        store.set_player_stat(&stat).unwrap();
        store
            .set_player_stat(&PlayerStat::new(p.id.clone(), &definition, 8.0, now()).unwrap())
            .unwrap();
        assert_eq!(
            store.list_player_stats(&p.id).unwrap()[0].current_value,
            8.0
        );
        let mut invalid_stat = stat.clone();
        invalid_stat.current_value = 11.0;
        assert!(store.set_player_stat(&invalid_stat).is_err());

        let mut active = Effect::new(
            EntityId::new("active-effect").unwrap(),
            p.id.clone(),
            TypeRef::effect("buff").unwrap(),
            "Focused",
            now(),
        )
        .unwrap();
        active.intensity = 2;
        store.insert_effect(&active).unwrap();
        let snapshot = store
            .capture_player_snapshot(&p.id, &DateValue::parse("2026-09-25").unwrap(), &now())
            .unwrap();
        let state: serde_json::Value = serde_json::from_str(&snapshot.state_json).unwrap();
        assert_eq!(state["schemaVersion"], 1);
        assert_eq!(state["stats"][0]["value"], 8.0);
        assert_eq!(state["activeEffects"][0]["id"], "active-effect");
        assert_eq!(
            store.list_player_snapshots(&p.id).unwrap(),
            vec![snapshot.clone()]
        );
        let snapshot_id = snapshot.id.unwrap();
        assert!(
            store
                .with_conn(|conn| conn
                    .execute(
                        "UPDATE player_state_snapshots SET state_json='{}' WHERE id=?1",
                        [snapshot_id]
                    )
                    .map(|_| ())
                    .map_err(op))
                .is_err(),
            "historical state cannot be edited in place"
        );
        assert!(
            store
                .with_conn(|conn| conn
                    .execute(
                        "DELETE FROM player_state_snapshots WHERE id=?1",
                        [snapshot_id]
                    )
                    .map(|_| ())
                    .map_err(op))
                .is_err(),
            "historical state cannot be deleted in place"
        );
        assert!(
            store
                .capture_player_snapshot(&p.id, &DateValue::parse("2026-09-25").unwrap(), &now())
                .is_err(),
            "one snapshot per player/date"
        );

        let tree = SkillTree::new(
            EntityId::new("t1").unwrap(),
            p.id.clone(),
            TypeRef::skill_tree("programming").unwrap(),
            "Programming",
            now(),
        )
        .unwrap();
        store.insert_skill_tree(&tree).unwrap();
        let mut skill = Skill::new(
            EntityId::new("s1").unwrap(),
            tree.id.clone(),
            TypeRef::skill("core").unwrap(),
            "Rust",
            now(),
        )
        .unwrap();
        skill.invest_time(30, now()).unwrap();
        store.insert_skill(&skill).unwrap();
        let skill_snapshot = store
            .capture_skill_snapshot(&skill.id, &DateValue::parse("2026-09-25").unwrap(), &now())
            .unwrap();
        let skill_state: serde_json::Value =
            serde_json::from_str(&skill_snapshot.state_json).unwrap();
        assert_eq!(skill_state["skillTreeId"], "t1");
        assert_eq!(
            store.list_skill_snapshots(&skill.id).unwrap(),
            vec![skill_snapshot]
        );
    }

    #[test]
    fn effect_lifecycle_and_xp_applied_delta_are_persisted() {
        let store = store();
        let mut p = player_value();
        store.create_player(&p).unwrap();
        let mut effect = Effect::new(
            EntityId::new("e1").unwrap(),
            p.id.clone(),
            TypeRef::effect("buff").unwrap(),
            "Focus",
            now(),
        )
        .unwrap();
        effect.expires_at = Some(Iso8601Timestamp::parse("2026-09-26T00:00:00Z").unwrap());
        store.insert_effect(&effect).unwrap();
        assert_eq!(store.list_effects(&p.id, Some(T0)).unwrap().len(), 1);
        let deactivated = store
            .deactivate_effect(
                &effect.id,
                &Iso8601Timestamp::parse("2026-09-25T12:00:00Z").unwrap(),
            )
            .unwrap();
        assert_eq!(
            deactivated.lifecycle_at(&Iso8601Timestamp::parse("2026-09-25T13:00:00Z").unwrap()),
            EffectLifecycle::ManuallyDeactivated
        );
        assert!(store
            .list_effects(&p.id, Some("2026-09-25T13:00:00+00:00"))
            .unwrap()
            .is_empty());

        p.apply_xp(5, now()).unwrap();
        let gain = Transaction::xp_adjustment(p.id.clone(), 5, 5, now()).unwrap();
        store.award_xp(&p, &gain).unwrap();
        p.apply_xp(-20, now()).unwrap();
        let penalty = Transaction::xp_adjustment(p.id.clone(), -20, -5, now()).unwrap();
        store.award_xp(&p, &penalty).unwrap();
        assert_eq!(p.current_xp, 0);
        assert_eq!(store.transaction_total(&p.id, "xp").unwrap(), 0);
        let events = store.list_transactions(&p.id, 10).unwrap();
        assert_eq!(events[0].amount, -20);
        assert_eq!(events[0].applied_amount, Some(-5));
        let event_id = events[0].id.unwrap();
        assert!(
            store
                .with_conn(|conn| conn
                    .execute(
                        "UPDATE transactions SET reason='rewritten' WHERE id=?1",
                        [event_id]
                    )
                    .map(|_| ())
                    .map_err(op))
                .is_err(),
            "ledger events are append-only"
        );
        assert!(
            store
                .with_conn(|conn| conn
                    .execute("DELETE FROM transactions WHERE id=?1", [event_id])
                    .map(|_| ())
                    .map_err(op))
                .is_err(),
            "ledger events cannot be deleted"
        );
        let direct = store.with_conn(|conn| {
            conn.execute("UPDATE players SET current_xp=-1 WHERE id='p1'", [])
                .map(|_| ())
                .map_err(op)
        });
        assert!(
            direct.is_err(),
            "SQLite protects cached XP even when bypassing the domain"
        );
    }

    #[test]
    fn atomic_world_operations_reject_mismatched_history_without_partial_writes() {
        let store = store();
        let p = player_value();
        store.create_player(&p).unwrap();
        let mut next = p.clone();
        next.apply_xp(10, now()).unwrap();
        let mismatched = Transaction::xp_adjustment(p.id.clone(), 9, 9, now()).unwrap();
        assert!(store.award_xp(&next, &mismatched).is_err());
        assert_eq!(store.get_player(&p.id).unwrap().unwrap().current_xp, 0);
        assert_eq!(store.list_transactions(&p.id, 10).unwrap().len(), 0);

        let mut q = Quest::new(
            EntityId::new("q1").unwrap(),
            p.id.clone(),
            TypeRef::quest("main").unwrap(),
            "Reward",
            now(),
        )
        .unwrap();
        q.xp_reward = 20;
        store.insert_quest(&q).unwrap();
        q.complete(now()).unwrap();
        let mut awarded = p.clone();
        awarded.apply_xp(20, now()).unwrap();
        let reward = Transaction::xp_adjustment(p.id.clone(), 19, 19, now()).unwrap();
        assert!(store
            .complete_quest(&q, Some(&awarded), Some(&reward))
            .is_err());
        assert_eq!(store.get_player(&p.id).unwrap().unwrap().current_xp, 0);
        assert_eq!(
            store.get_quest(&q.id).unwrap().unwrap().status,
            QuestStatus::Open
        );
        assert_eq!(store.list_transactions(&p.id, 10).unwrap().len(), 0);

        let tree = SkillTree::new(
            EntityId::new("t1").unwrap(),
            p.id.clone(),
            TypeRef::skill_tree("programming").unwrap(),
            "Programming",
            now(),
        )
        .unwrap();
        store.insert_skill_tree(&tree).unwrap();
        let mut skill = Skill::new(
            EntityId::new("s1").unwrap(),
            tree.id.clone(),
            TypeRef::skill("core").unwrap(),
            "Rust",
            now(),
        )
        .unwrap();
        store.insert_skill(&skill).unwrap();
        skill.invest_time(60, now()).unwrap();
        let time = Transaction::new(
            p.id.clone(),
            TypeRef::transaction("time").unwrap(),
            "minutes",
            30,
            now(),
        )
        .unwrap();
        assert!(store.invest_skill_time(&skill, &time).is_err());
        assert_eq!(
            store
                .get_skill(&skill.id)
                .unwrap()
                .unwrap()
                .invested_minutes,
            0
        );
        assert_eq!(store.list_transactions(&p.id, 10).unwrap().len(), 0);
    }

    #[test]
    fn sqlite_rejects_cross_owner_parent_links_and_cycles() {
        let store = store();
        let p1 = player_value();
        let p2 = Player::new(EntityId::new("p2").unwrap(), "Lin", now()).unwrap();
        store.create_player(&p1).unwrap();
        store.create_player(&p2).unwrap();
        let t1 = SkillTree::new(
            EntityId::new("t1").unwrap(),
            p1.id.clone(),
            TypeRef::skill_tree("programming").unwrap(),
            "One",
            now(),
        )
        .unwrap();
        let t2 = SkillTree::new(
            EntityId::new("t2").unwrap(),
            p2.id.clone(),
            TypeRef::skill_tree("programming").unwrap(),
            "Two",
            now(),
        )
        .unwrap();
        store.insert_skill_tree(&t1).unwrap();
        store.insert_skill_tree(&t2).unwrap();
        let parent = Skill::new(
            EntityId::new("s1").unwrap(),
            t1.id.clone(),
            TypeRef::skill("core").unwrap(),
            "Parent",
            now(),
        )
        .unwrap();
        store.insert_skill(&parent).unwrap();
        let mut child = Skill::new(
            EntityId::new("s2").unwrap(),
            t2.id.clone(),
            TypeRef::skill("core").unwrap(),
            "Child",
            now(),
        )
        .unwrap();
        child.set_parent(Some(parent.id.clone())).unwrap();
        assert!(
            store.insert_skill(&child).is_err(),
            "cross-tree parent must be rejected"
        );
        let foreign_skill = Skill::new(
            EntityId::new("s-foreign").unwrap(),
            t2.id.clone(),
            TypeRef::skill("core").unwrap(),
            "Foreign skill",
            now(),
        )
        .unwrap();
        store.insert_skill(&foreign_skill).unwrap();
        let mut local_child = Skill::new(
            EntityId::new("s-local").unwrap(),
            t1.id.clone(),
            TypeRef::skill("core").unwrap(),
            "Local child",
            now(),
        )
        .unwrap();
        local_child.set_parent(Some(parent.id.clone())).unwrap();
        store.insert_skill(&local_child).unwrap();
        let mut cycle = parent.clone();
        cycle.set_parent(Some(local_child.id.clone())).unwrap();
        assert!(
            store.update_skill(&cycle).is_err(),
            "indirect skill cycles are rejected"
        );

        let mut q1 = Quest::new(
            EntityId::new("q1").unwrap(),
            p1.id.clone(),
            TypeRef::quest("main").unwrap(),
            "One",
            now(),
        )
        .unwrap();
        let q2 = Quest::new(
            EntityId::new("q2").unwrap(),
            p2.id.clone(),
            TypeRef::quest("main").unwrap(),
            "Two",
            now(),
        )
        .unwrap();
        store.insert_quest(&q1).unwrap();
        store.insert_quest(&q2).unwrap();
        let mut cross = q1.clone();
        cross.id = EntityId::new("q3").unwrap();
        cross.player_id = p2.id.clone();
        cross.set_parent(Some(q1.id.clone())).unwrap();
        assert!(
            store.insert_quest(&cross).is_err(),
            "cross-player parent must be rejected"
        );
        let mut other_world_skill = Quest::new(
            EntityId::new("q-skill").unwrap(),
            p1.id.clone(),
            TypeRef::quest("main").unwrap(),
            "Wrong skill",
            now(),
        )
        .unwrap();
        other_world_skill.skill_id = Some(foreign_skill.id.clone());
        assert!(
            store.insert_quest(&other_world_skill).is_err(),
            "quest cannot refer to a skill owned by another player's world"
        );
        let mut local_child_quest = Quest::new(
            EntityId::new("q-local").unwrap(),
            p1.id.clone(),
            TypeRef::quest("main").unwrap(),
            "Child",
            now(),
        )
        .unwrap();
        local_child_quest.set_parent(Some(q1.id.clone())).unwrap();
        store.insert_quest(&local_child_quest).unwrap();
        let mut quest_cycle = q1.clone();
        quest_cycle
            .set_parent(Some(local_child_quest.id.clone()))
            .unwrap();
        assert!(
            store.update_quest(&quest_cycle).is_err(),
            "indirect quest cycles are rejected"
        );
        q1.set_parent(Some(q2.id.clone())).unwrap();
        assert!(
            store.update_quest(&q1).is_err(),
            "cross-player parent update must be rejected"
        );
    }
}
