//! Bounded, read-only composition of persisted timestamped sources for Timeline.

use lr_application::{
    StorageError, TimelineCategory, TimelineEntityKind, TimelineItem, TimelineQuery,
    TimelineRelationshipContext, TimelineStore, TimelineTimestampKind,
};
use lr_domain::{EntityId, Iso8601Timestamp};
use rusqlite::{params, Connection, Row};

use crate::sqlite_store::SqliteHealthStore;

fn operation(error: impl std::fmt::Display) -> StorageError {
    StorageError::Operation(error.to_string())
}

fn source_queries(query: &TimelineQuery) -> Vec<&'static str> {
    let mut sources = Vec::new();
    let wants = |category| query.category.map_or(true, |selected| selected == category);

    if wants(TimelineCategory::RecordChange) {
        sources.extend([
            r#"SELECT 'record:player:'||p.id||':created',p.id,'record_change','player',p.id,p.created_at,NULL,'created','Player world created · '||p.name,COALESCE(p.description,''),NULL,NULL,NULL,NULL FROM players p WHERE p.id=?1 AND (?3 IS NULL OR p.created_at>=?3) AND (?4 IS NULL OR p.created_at<=?4)"#,
            r#"SELECT 'record:concept:'||c.id||':created',c.player_id,'record_change','concept',c.id,c.created_at,NULL,'created','Concept created · '||c.name,COALESCE(c.description,''),c.id,c.concept_type_code,CASE WHEN c.is_active=1 THEN 'active' ELSE 'archived' END,NULL FROM concepts c WHERE c.player_id=?1 AND (?3 IS NULL OR c.created_at>=?3) AND (?4 IS NULL OR c.created_at<=?4)"#,
            r#"SELECT 'record:skill_tree:'||t.id||':created',t.player_id,'record_change','skill_tree',t.id,t.created_at,NULL,'created','Skill Tree created · '||t.name,COALESCE(t.description,''),NULL,t.tree_type_code,NULL,NULL FROM skill_trees t WHERE t.player_id=?1 AND (?3 IS NULL OR t.created_at>=?3) AND (?4 IS NULL OR t.created_at<=?4)"#,
            r#"SELECT 'record:quest:'||q.id||':created',q.player_id,'record_change','quest',q.id,q.created_at,NULL,'created','Quest created · '||q.title,COALESCE(q.description,''),NULL,q.quest_type_code,q.status,NULL FROM quests q WHERE q.player_id=?1 AND (?3 IS NULL OR q.created_at>=?3) AND (?4 IS NULL OR q.created_at<=?4)"#,
            r#"SELECT 'record:quest_stage:'||s.id||':created',s.player_id,'record_change','quest_stage',s.id,s.created_at,NULL,'created','Quest stage created · '||s.title,COALESCE(s.description,''),NULL,NULL,s.status,NULL FROM quest_stages s WHERE s.player_id=?1 AND (?3 IS NULL OR s.created_at>=?3) AND (?4 IS NULL OR s.created_at<=?4)"#,
            r#"SELECT 'record:quest_branch:'||b.id||':created',b.player_id,'record_change','quest_branch',b.id,b.created_at,NULL,'created','Quest branch created · '||b.title,COALESCE(b.description,''),NULL,NULL,b.status,NULL FROM quest_branches b WHERE b.player_id=?1 AND (?3 IS NULL OR b.created_at>=?3) AND (?4 IS NULL OR b.created_at<=?4)"#,
            r#"SELECT 'record:quest:'||q.id||':started',q.player_id,'record_change','quest',q.id,q.started_at,NULL,'occurred','Quest started · '||q.title,COALESCE(q.description,''),NULL,q.quest_type_code,q.status,NULL FROM quests q WHERE q.player_id=?1 AND q.started_at IS NOT NULL AND (?3 IS NULL OR q.started_at>=?3) AND (?4 IS NULL OR q.started_at<=?4)"#,
            r#"SELECT 'record:quest:'||q.id||':completed',q.player_id,'record_change','quest',q.id,q.completed_at,NULL,'occurred','Quest completed · '||q.title,COALESCE(q.description,''),NULL,q.quest_type_code,q.status,NULL FROM quests q WHERE q.player_id=?1 AND q.completed_at IS NOT NULL AND (?3 IS NULL OR q.completed_at>=?3) AND (?4 IS NULL OR q.completed_at<=?4)"#,
            r#"SELECT 'record:skill:'||s.id||':created',t.player_id,'record_change','skill',s.id,s.created_at,NULL,'created','Skill created · '||s.name,COALESCE(s.description,''),NULL,s.skill_type_code,s.status,NULL FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE t.player_id=?1 AND (?3 IS NULL OR s.created_at>=?3) AND (?4 IS NULL OR s.created_at<=?4)"#,
            r#"SELECT 'record:skill:'||s.id||':started',t.player_id,'record_change','skill',s.id,s.started_at,NULL,'occurred','Skill started · '||s.name,COALESCE(s.description,''),NULL,s.skill_type_code,s.status,NULL FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE t.player_id=?1 AND s.started_at IS NOT NULL AND (?3 IS NULL OR s.started_at>=?3) AND (?4 IS NULL OR s.started_at<=?4)"#,
            r#"SELECT 'record:skill:'||s.id||':completed',t.player_id,'record_change','skill',s.id,s.completed_at,NULL,'occurred','Skill completed · '||s.name,COALESCE(s.description,''),NULL,s.skill_type_code,s.status,NULL FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE t.player_id=?1 AND s.completed_at IS NOT NULL AND (?3 IS NULL OR s.completed_at>=?3) AND (?4 IS NULL OR s.completed_at<=?4)"#,
        ]);
        sources.push(r#"SELECT 'record:skill_history:'||h.id,h.player_id,'record_change','skill',s.id,h.recorded_at,NULL,'recorded',CASE WHEN h.source='rule' THEN 'Skill unlocked by Rule · '||s.name WHEN h.event_kind='availability_changed' AND s.availability='available' THEN 'Skill manually unlocked · '||s.name WHEN h.event_kind='availability_changed' THEN 'Skill manually locked · '||s.name ELSE 'Skill unlock authority changed · '||s.name END,COALESCE(json_extract(h.previous_state_json,'$.availability'),'first')||' / '||COALESCE(json_extract(h.previous_state_json,'$.availabilityControl'),'first')||' → '||json_extract(h.current_state_json,'$.availability')||' / '||json_extract(h.current_state_json,'$.availabilityControl')||' · '||h.source,NULL,s.skill_type_code,h.source,NULL FROM skill_history h JOIN skills s ON s.id=h.skill_id WHERE h.player_id=?1 AND (?3 IS NULL OR h.recorded_at>=?3) AND (?4 IS NULL OR h.recorded_at<=?4)"#);
    }

    if wants(TimelineCategory::Session) {
        sources.push(r#"SELECT 'session:'||s.id,s.player_id,'session','quest_session',s.id,s.started_at,s.ended_at,'occurred','Session · '||COALESCE(q.title,sk.name,c.name,'Recorded activity'),trim(COALESCE(s.result,'')||CASE WHEN s.result IS NOT NULL AND s.notes IS NOT NULL THEN ' · ' ELSE '' END||COALESCE(s.notes,'')),s.concept_id,NULL,s.status,CASE WHEN s.ended_at IS NOT NULL THEN 'occurred' END FROM quest_sessions s LEFT JOIN quests q ON q.id=s.quest_id LEFT JOIN skills sk ON sk.id=s.skill_id LEFT JOIN concepts c ON c.id=s.concept_id WHERE s.player_id=?1 AND (?3 IS NULL OR s.started_at>=?3) AND (?4 IS NULL OR s.started_at<=?4)"#);
    }

    if wants(TimelineCategory::Transaction) {
        sources.push(r#"SELECT 'transaction:'||x.id,x.player_id,'transaction',CASE WHEN x.resource='skill_xp' AND x.source_kind='skill' THEN 'skill' ELSE 'transaction' END,CASE WHEN x.resource='skill_xp' AND x.source_kind='skill' THEN x.source_id ELSE CAST(x.id AS TEXT) END,x.occurred_at,x.captured_at,'occurred',CASE WHEN x.resource='skill_xp' AND x.source_kind='skill' THEN 'Skill XP changed · '||COALESCE(s.name,'Skill') ELSE 'Transaction · '||x.resource END,CASE WHEN x.resource='skill_xp' AND x.source_kind='skill' THEN 'requested '||x.amount||' · applied '||COALESCE(x.applied_amount,0)||' · '||COALESCE(CAST(json_extract(x.metadata_json,'$.previousXp') AS TEXT),'?')||' → '||COALESCE(CAST(json_extract(x.metadata_json,'$.currentXp') AS TEXT),'?')||CASE WHEN x.reason IS NOT NULL THEN ' · '||x.reason ELSE '' END ELSE (CASE WHEN x.amount>0 THEN '+' ELSE '' END)||x.amount||CASE WHEN x.reason IS NOT NULL THEN ' · '||x.reason ELSE '' END END,NULL,x.transaction_type_code,NULL,CASE WHEN x.captured_at IS NOT NULL THEN 'captured' END FROM transactions x LEFT JOIN skills s ON x.resource='skill_xp' AND x.source_kind='skill' AND s.id=x.source_id WHERE x.player_id=?1 AND (?3 IS NULL OR x.occurred_at>=?3) AND (?4 IS NULL OR x.occurred_at<=?4)"#);
    }

    if wants(TimelineCategory::EffectHistory) {
        sources.push(r#"SELECT 'effect_history:'||h.id,h.player_id,'effect_history','effect',h.effect_id,h.recorded_at,NULL,'recorded',CASE h.event_kind WHEN 'created' THEN 'Effect recorded' WHEN 'details_changed' THEN 'Effect details changed' WHEN 'expiry_changed' THEN 'Effect expiry changed' WHEN 'manually_deactivated' THEN 'Effect manually deactivated' WHEN 'rule_deactivated' THEN 'Effect deactivated by Rule' WHEN 'session_linked' THEN 'Effect linked to Session' WHEN 'session_unlinked' THEN 'Effect unlinked from Session' ELSE 'Effect history' END||' · '||e.name,CASE WHEN h.session_id IS NOT NULL THEN 'Session '||h.session_id ELSE '' END,NULL,e.effect_type_code,h.event_kind,NULL FROM effect_history h JOIN effects e ON e.id=h.effect_id WHERE h.player_id=?1 AND (?3 IS NULL OR h.recorded_at>=?3) AND (?4 IS NULL OR h.recorded_at<=?4)"#);
    }

    if wants(TimelineCategory::Content) {
        sources.extend([
            r#"SELECT 'content:'||n.id||':created',n.player_id,'content','narrative_entry',n.id,n.created_at,NULL,'created','Content created · '||n.title,substr(n.content,1,280),NULL,n.kind_code,NULL,NULL FROM narrative_entries n WHERE n.player_id=?1 AND (?3 IS NULL OR n.created_at>=?3) AND (?4 IS NULL OR n.created_at<=?4)"#,
            r#"SELECT 'content:'||n.id||':updated',n.player_id,'content','narrative_entry',n.id,n.updated_at,NULL,'updated','Content last updated · '||n.title,substr(n.content,1,280),NULL,n.kind_code,NULL,NULL FROM narrative_entries n WHERE n.player_id=?1 AND n.updated_at>n.created_at AND (?3 IS NULL OR n.updated_at>=?3) AND (?4 IS NULL OR n.updated_at<=?4)"#,
        ]);
    }

    if wants(TimelineCategory::Comment) {
        sources.push(r#"SELECT 'comment:'||c.id,t.player_id,'comment',t.entity_kind,t.entity_id,c.created_at,NULL,'created','Comment · '||t.title,substr(c.body,1,280),NULL,NULL,NULL,NULL FROM comments c JOIN (
            SELECT 'player' AS entity_kind,p.id AS entity_id,p.id AS player_id,p.name AS title FROM players p WHERE p.id=?1
            UNION ALL SELECT 'quest',q.id,q.player_id,q.title FROM quests q WHERE q.player_id=?1
            UNION ALL SELECT 'skill',s.id,tr.player_id,s.name FROM skills s JOIN skill_trees tr ON tr.id=s.skill_tree_id WHERE tr.player_id=?1
            UNION ALL SELECT 'skill_tree',tr.id,tr.player_id,tr.name FROM skill_trees tr WHERE tr.player_id=?1
            UNION ALL SELECT 'effect',e.id,e.player_id,e.name FROM effects e WHERE e.player_id=?1
            UNION ALL SELECT 'transaction',CAST(x.id AS TEXT),x.player_id,x.resource FROM transactions x WHERE x.player_id=?1
            UNION ALL SELECT 'narrative_entry',n.id,n.player_id,n.title FROM narrative_entries n WHERE n.player_id=?1
        ) t ON c.target_kind=t.entity_kind AND c.target_id=t.entity_id WHERE (?3 IS NULL OR c.created_at>=?3) AND (?4 IS NULL OR c.created_at<=?4)"#);
    }

    if wants(TimelineCategory::ConceptProgress) {
        sources.push(r#"SELECT 'progress:'||h.id,c.player_id,'concept_progress','concept',c.id,h.occurred_at,h.captured_at,'occurred','Concept progress · '||c.name, h.track_code||' · '||COALESCE(CAST(h.previous_value AS TEXT),'first value')||' → '||CAST(h.current_value AS TEXT)||CASE WHEN h.level IS NOT NULL THEN ' · level '||h.level ELSE '' END,h.concept_id,h.track_code,NULL,'captured' FROM concept_progress_history h JOIN concepts c ON c.id=h.concept_id WHERE c.player_id=?1 AND (?3 IS NULL OR h.occurred_at>=?3) AND (?4 IS NULL OR h.occurred_at<=?4)"#);
    }

    if wants(TimelineCategory::Revision) {
        sources.push(r#"SELECT 'revision:'||r.id,r.player_id,'revision',r.target_kind,r.target_id,r.recorded_at,NULL,'recorded',r.target_kind||' revision #'||r.revision_number,COALESCE(r.reason,'Historical record change'),NULL,NULL,CAST(r.revision_number AS TEXT),NULL FROM entity_revisions r WHERE r.player_id=?1 AND (?3 IS NULL OR r.recorded_at>=?3) AND (?4 IS NULL OR r.recorded_at<=?4)"#);
    }

    if wants(TimelineCategory::Snapshot) {
        sources.extend([
            r#"SELECT 'snapshot:player:'||s.id,s.player_id,'snapshot','player',s.player_id,s.created_at,NULL,'captured','Player snapshot captured','State on '||s.snapshot_date,NULL,NULL,NULL,NULL FROM player_state_snapshots s WHERE s.player_id=?1 AND (?3 IS NULL OR s.created_at>=?3) AND (?4 IS NULL OR s.created_at<=?4)"#,
            r#"SELECT 'snapshot:skill:'||s.id,t.player_id,'snapshot','skill',sk.id,s.created_at,NULL,'captured','Skill snapshot captured · '||sk.name,'State on '||s.snapshot_date,NULL,NULL,s.status,NULL FROM skill_state_snapshots s JOIN skills sk ON sk.id=s.skill_id JOIN skill_trees t ON t.id=sk.skill_tree_id WHERE t.player_id=?1 AND (?3 IS NULL OR s.created_at>=?3) AND (?4 IS NULL OR s.created_at<=?4)"#,
            r#"SELECT 'snapshot:concept:'||s.id,c.player_id,'snapshot','concept',c.id,s.captured_at,NULL,'captured','Concept snapshot captured · '||c.name,'State on '||s.snapshot_date,c.id,NULL,NULL,NULL FROM concept_state_snapshots s JOIN concepts c ON c.id=s.concept_id WHERE c.player_id=?1 AND (?3 IS NULL OR s.captured_at>=?3) AND (?4 IS NULL OR s.captured_at<=?4)"#,
        ]);
    }

    if wants(TimelineCategory::Lifecycle) {
        sources.push(r#"SELECT 'lifecycle:'||h.id,h.player_id,'lifecycle',h.target_kind,h.target_id,h.occurred_at,h.captured_at,'occurred','Record lifecycle · '||h.current_state,COALESCE(h.previous_state,'initial')||' → '||h.current_state||CASE WHEN h.reason IS NOT NULL THEN ' · '||h.reason ELSE '' END,NULL,NULL,h.current_state,'captured' FROM entity_lifecycle_history h WHERE h.player_id=?1 AND (?3 IS NULL OR h.occurred_at>=?3) AND (?4 IS NULL OR h.occurred_at<=?4)"#);
    }

    if wants(TimelineCategory::RelationshipHistory) {
        sources.extend([
            r#"SELECT 'relationship:'||ca.id||':created',ca.player_id,'relationship_history',ca.target_kind,ca.target_id,ca.created_at,NULL,'created','Content relationship created · '||ca.role_code,'Content · '||n.title||' → '||ca.target_kind||' '||ca.target_id,CASE WHEN ca.target_kind='concept' THEN ca.target_id END,ca.role_code,'created',NULL FROM content_attachments ca JOIN narrative_entries n ON n.id=ca.content_id WHERE ca.player_id=?1 AND (?3 IS NULL OR ca.created_at>=?3) AND (?4 IS NULL OR ca.created_at<=?4)"#,
            r#"SELECT 'relationship:'||ca.id||':removed',ca.player_id,'relationship_history',ca.target_kind,ca.target_id,ca.removed_at,NULL,'removed','Content relationship removed · '||ca.role_code,'Content · '||n.title||' → '||ca.target_kind||' '||ca.target_id,CASE WHEN ca.target_kind='concept' THEN ca.target_id END,ca.role_code,'removed',NULL FROM content_attachments ca JOIN narrative_entries n ON n.id=ca.content_id WHERE ca.player_id=?1 AND ca.removed_at IS NOT NULL AND (?3 IS NULL OR ca.removed_at>=?3) AND (?4 IS NULL OR ca.removed_at<=?4)"#,
        ]);
    }

    sources
}

fn parse_item(row: &Row<'_>) -> rusqlite::Result<TimelineItem> {
    let category_raw: String = row.get(2)?;
    let category = TimelineCategory::parse(&category_raw).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(2, "category".into(), rusqlite::types::Type::Text)
    })?;
    let kind_raw: String = row.get(3)?;
    let entity_kind = TimelineEntityKind::parse(&kind_raw).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(3, "entity_kind".into(), rusqlite::types::Type::Text)
    })?;
    let timestamp_raw: String = row.get(5)?;
    let timestamp = Iso8601Timestamp::parse(timestamp_raw).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let secondary_raw: Option<String> = row.get(6)?;
    let secondary_timestamp = secondary_raw
        .map(Iso8601Timestamp::parse)
        .transpose()
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
    let timestamp_kind_raw: String = row.get(7)?;
    let timestamp_kind = TimelineTimestampKind::parse(&timestamp_kind_raw).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(7, "timestamp_kind".into(), rusqlite::types::Type::Text)
    })?;
    let secondary_kind_raw: Option<String> = row.get(13)?;
    let secondary_timestamp_kind = secondary_kind_raw
        .map(|raw| {
            TimelineTimestampKind::parse(&raw).ok_or_else(|| {
                rusqlite::Error::InvalidColumnType(
                    13,
                    "secondary_timestamp_kind".into(),
                    rusqlite::types::Type::Text,
                )
            })
        })
        .transpose()?;
    if secondary_timestamp.is_some() != secondary_timestamp_kind.is_some() {
        return Err(rusqlite::Error::InvalidColumnType(
            13,
            "secondary_timestamp_kind".into(),
            rusqlite::types::Type::Text,
        ));
    }
    let player_raw: String = row.get(1)?;
    let player_id = EntityId::new(player_raw).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let concept_raw: Option<String> = row.get(10)?;
    let concept_id = concept_raw
        .map(EntityId::new)
        .transpose()
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                10,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
    let relationship_context = match row.get::<_, Option<String>>(14)? {
        None => None,
        Some(relationship_id) => {
            let created_raw: String = row.get(18)?;
            let created_at = Iso8601Timestamp::parse(created_raw).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    18,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            let removed_raw: Option<String> = row.get(19)?;
            let removed_at = removed_raw
                .map(Iso8601Timestamp::parse)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        19,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            Some(TimelineRelationshipContext {
                relationship_id,
                content_id: row.get(15)?,
                content_title: row.get(16)?,
                role_code: row.get(17)?,
                created_at,
                removed_at,
            })
        }
    };

    Ok(TimelineItem {
        source_id: row.get(0)?,
        player_id,
        category,
        entity_kind,
        entity_id: row.get(4)?,
        timestamp,
        secondary_timestamp,
        timestamp_kind,
        secondary_timestamp_kind,
        title: row.get(8)?,
        summary: row.get(9)?,
        concept_id,
        type_code: row.get(11)?,
        state: row.get(12)?,
        relationship_context,
    })
}

impl TimelineStore for SqliteHealthStore {
    fn query_timeline(&self, query: &TimelineQuery) -> Result<Vec<TimelineItem>, StorageError> {
        query.validate().map_err(operation)?;
        let sources = source_queries(query);
        if sources.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "WITH timeline(source_id,player_id,category,entity_kind,entity_id,timestamp,secondary_timestamp,timestamp_kind,title,summary,concept_id,type_code,state,secondary_timestamp_kind) AS ({}) SELECT t.*,ca.id,n.id,n.title,ca.role_code,ca.created_at,ca.removed_at FROM timeline t LEFT JOIN content_attachments ca ON t.category='relationship_history' AND t.source_id IN ('relationship:'||ca.id||':created','relationship:'||ca.id||':removed') LEFT JOIN narrative_entries n ON n.id=ca.content_id WHERE (?2 IS NULL OR t.category=?2) AND (?5 IS NULL OR t.entity_kind=?5) AND (?6 IS NULL OR t.concept_id=?6 OR (t.entity_kind='concept' AND t.entity_id=?6) OR EXISTS(SELECT 1 FROM concept_entity_links l WHERE l.player_id=t.player_id AND l.concept_id=?6 AND l.entity_kind=t.entity_kind AND l.entity_id=t.entity_id) OR EXISTS(SELECT 1 FROM concept_associations a WHERE a.player_id=t.player_id AND a.concept_id=?6 AND a.entity_kind=t.entity_kind AND a.entity_id=t.entity_id AND a.is_active=1) OR (t.entity_kind='narrative_entry' AND EXISTS(SELECT 1 FROM content_attachments ca WHERE ca.player_id=t.player_id AND ca.content_id=t.entity_id AND ca.target_kind='concept' AND ca.target_id=?6 AND ca.removed_at IS NULL)) OR (t.entity_kind='effect' AND EXISTS(SELECT 1 FROM effects e WHERE e.id=t.entity_id AND e.target_kind='concept' AND e.target_concept_id=?6))) AND (?10 IS NULL OR t.entity_id=?10) ORDER BY CASE WHEN ?7='asc' THEN t.timestamp END ASC, CASE WHEN ?7='desc' THEN t.timestamp END DESC, t.category COLLATE BINARY ASC, t.source_id COLLATE BINARY ASC, t.entity_kind COLLATE BINARY ASC, t.entity_id COLLATE BINARY ASC LIMIT ?8 OFFSET ?9",
            sources.join(" UNION ALL ")
        );
        self.with_conn(|connection| execute_query(connection, &sql, query))
    }
}

fn execute_query(
    connection: &Connection,
    sql: &str,
    query: &TimelineQuery,
) -> Result<Vec<TimelineItem>, StorageError> {
    let mut statement = connection.prepare(sql).map_err(operation)?;
    let mut rows = statement
        .query(params![
            query.player_id.as_str(),
            query.category.map(TimelineCategory::as_str),
            query.from.as_ref().map(Iso8601Timestamp::as_str),
            query.through.as_ref().map(Iso8601Timestamp::as_str),
            query.entity_kind.map(TimelineEntityKind::as_str),
            query.concept_id.as_ref().map(EntityId::as_str),
            query.sort.as_str(),
            query.limit as i64,
            query.offset as i64,
            query.entity_id.as_ref().map(EntityId::as_str),
        ])
        .map_err(operation)?;
    let mut output = Vec::new();
    while let Some(row) = rows.next().map_err(operation)? {
        output.push(parse_item(row).map_err(operation)?);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: &str = "2026-09-27T08:00:00+00:00";
    const T1: &str = "2026-09-28T09:00:00+00:00";
    const T2: &str = "2026-09-28T10:00:00+00:00";
    const T3: &str = "2026-09-28T11:00:00+00:00";
    const T4: &str = "2026-09-28T12:00:00+00:00";
    const T5: &str = "2026-09-28T13:00:00+00:00";

    fn base_query(player: &str) -> TimelineQuery {
        TimelineQuery {
            player_id: EntityId::new(player).unwrap(),
            category: None,
            entity_kind: None,
            entity_id: None,
            concept_id: None,
            from: None,
            through: None,
            sort: lr_application::TimelineSort::Newest,
            limit: 200,
            offset: 0,
        }
    }

    fn fixture() -> SqliteHealthStore {
        let store = SqliteHealthStore::open_in_memory(T0);
        store
            .with_conn_mut(|connection| {
                connection
                    .execute_batch(&format!(
                        r#"
                        INSERT INTO players(id,name,level,current_xp,created_at,updated_at) VALUES
                          ('p1','Ada',1,0,'{T0}','{T0}'),
                          ('p2','Grace',1,0,'{T0}','{T0}');
                        INSERT INTO concepts(id,player_id,concept_type_code,name,created_at,updated_at,transfer_key) VALUES
                          ('c1','p1','subject','Gardening','{T0}','{T0}','concept-ref-v1-garden'),
                          ('c2','p2','subject','Music','{T0}','{T0}','concept-ref-v1-music');
                        INSERT INTO skill_trees(id,player_id,tree_type_code,name,created_at,updated_at) VALUES
                          ('tree1','p1','life','Practice','{T0}','{T0}');
                        INSERT INTO skills(id,skill_tree_id,skill_type_code,name,status,started_at,completed_at,created_at,updated_at) VALUES
                          ('skill1','tree1','core','Observation','completed','{T2}','{T4}','{T1}','{T4}');
                        INSERT INTO quests(id,player_id,quest_type_code,title,description,status,started_at,completed_at,created_at,updated_at) VALUES
                          ('q1','p1','main','Prepare the garden','Prepare soil.','completed','{T2}','{T4}','{T1}','{T4}'),
                          ('q2','p2','main','Practice music',NULL,'open',NULL,NULL,'{T1}','{T1}');
                        INSERT INTO quest_stages(id,player_id,quest_id,title,description,created_at,updated_at) VALUES
                          ('stage1','p1','q1','Prepare soil','Loosen the beds.','{T1}','{T1}');
                        INSERT INTO quest_branches(id,player_id,quest_id,stage_id,title,description,created_at,updated_at) VALUES
                          ('branch1','p1','q1','stage1','Use raised beds','Build raised planters.','{T2}','{T2}');
                        INSERT INTO quest_sessions(id,player_id,quest_id,concept_id,started_at,ended_at,status,result,notes,created_at,updated_at) VALUES
                          ('session1','p1','q1','c1','{T2}','{T3}','completed','Seeds prepared','Started seedlings','{T2}','{T3}'),
                          ('session2','p1','q1','c1','{T1}',NULL,'in_progress',NULL,'Continuing practice','{T1}','{T1}');
                        INSERT INTO transactions(player_id,transaction_type_code,resource,amount,occurred_at,captured_at,reason) VALUES
                          ('p1','xp','experience',15,'{T2}',NULL,'legacy capture unavailable'),
                          ('p1','xp','experience',5,'{T2}',NULL,'tied occurrence'),
                          ('p2','xp','experience',20,'{T3}',NULL,'other world');
                        INSERT INTO effects(id,player_id,effect_type_code,name,started_at,expires_at,target_kind,target_concept_id,created_at,updated_at,deactivated_at,deactivation_source) VALUES
                          ('effect1','p1','buff','Focus','{T1}',NULL,'concept','c1','{T1}','{T4}','{T4}','rule');
                        INSERT INTO effect_history(id,player_id,effect_id,event_kind,recorded_at,current_state_json) VALUES
                          ('eh1','p1','effect1','created','{T2}','{{}}'),
                          ('eh2','p1','effect1','rule_deactivated','{T4}','{{"deactivationSource":"rule"}}');
                        INSERT INTO skill_history(id,player_id,skill_id,event_kind,source,recorded_at,previous_state_json,current_state_json) VALUES
                          ('sh1','p1','skill1','availability_changed','rule','{T3}','{{"availability":"locked","availabilityControl":"rule_controlled"}}','{{"availability":"available","availabilityControl":"rule_controlled"}}');
                        INSERT INTO transactions(player_id,transaction_type_code,resource,amount,applied_amount,occurred_at,captured_at,reason,source_kind,source_id,metadata_json) VALUES
                          ('p1','xp','skill_xp',5,5,'{T3}','{T4}','lesson','skill','skill1','{{"previousXp":0,"currentXp":5,"source":"manual"}}');
                        INSERT INTO narrative_entries(id,player_id,kind_namespace,kind_code,title,content,metadata_json,created_at,updated_at) VALUES
                          ('content1','p1','narrative_entry','guide','Garden guide','A reusable guide.','{{}}','{T1}','{T3}');
                        INSERT INTO comments(author_player_id,target_kind,target_id,body,created_at,updated_at) VALUES
                          ('p1','quest','q1','A comment on the exact Quest.','{T3}','{T3}');
                        INSERT INTO concept_progress_history(id,concept_id,track_code,previous_value,current_value,level,occurred_at,captured_at) VALUES
                          ('progress1','c1','confidence',20,35,2,'{T3}','{T4}');
                        INSERT INTO entity_revisions(id,player_id,target_kind,target_id,revision_number,recorded_at,reason,snapshot_json) VALUES
                          ('rev1','p1','quest','q1',1,'{T4}','Clarified steps','{{}}');
                        INSERT INTO player_state_snapshots(player_id,snapshot_date,level,current_xp,state_json,created_at) VALUES
                          ('p1','2026-09-28',1,15,'{{}}','{T3}');
                        INSERT INTO skill_state_snapshots(skill_id,snapshot_date,level,current_xp,status,invested_minutes,state_json,created_at) VALUES
                          ('skill1','2026-09-28',2,10,'completed',60,'{{}}','{T4}');
                        INSERT INTO concept_state_snapshots(concept_id,snapshot_date,state_json,captured_at) VALUES
                          ('c1','2026-09-28','{{}}','{T5}');
                        INSERT INTO entity_lifecycle_history(id,target_kind,target_id,player_id,previous_state,current_state,occurred_at,captured_at,reason) VALUES
                          ('life1','quest','q1','p1','active','archived','{T4}','{T5}','Put away for now');
                        INSERT INTO concept_entity_links(concept_id,player_id,entity_kind,entity_id,created_at) VALUES
                          ('c1','p1','quest','q1','{T3}');
                        INSERT INTO content_attachments(id,content_id,player_id,target_kind,target_id,role_code,created_at,removed_at,updated_at) VALUES
                          ('rel1','content1','p1','quest','q1','guidance','{T2}',NULL,'{T2}'),
                          ('rel2','content1','p1','concept','c1','reference','{T2}',NULL,'{T2}'),
                          ('rel3','content1','p1','quest','q1','reference','{T2}','{T5}','{T5}');
                        "#
                    ))
                    .map_err(operation)?;
                Ok(())
            })
            .expect("seed valid persisted Timeline sources");
        store
    }

    #[test]
    fn unified_projection_reads_real_sources_with_semantic_timestamps_and_stable_order() {
        let store = fixture();
        let before = store.with_conn(|connection| {
            connection
                .query_row("SELECT COUNT(*) FROM timeline_events", [], |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(operation)
        });
        assert!(
            before.is_err(),
            "there is intentionally no timeline_events table"
        );

        let items = store.query_timeline(&base_query("p1")).unwrap();
        assert!(
            items.len() >= 18,
            "expected the supported persisted source categories: {items:?}"
        );
        assert!(items.iter().all(|item| item.player_id.as_str() == "p1"));
        assert!(items
            .windows(2)
            .all(|pair| pair[0].timestamp >= pair[1].timestamp));
        for category in [
            TimelineCategory::RecordChange,
            TimelineCategory::Session,
            TimelineCategory::Transaction,
            TimelineCategory::EffectHistory,
            TimelineCategory::Content,
            TimelineCategory::Comment,
            TimelineCategory::ConceptProgress,
            TimelineCategory::Revision,
            TimelineCategory::Snapshot,
            TimelineCategory::Lifecycle,
            TimelineCategory::RelationshipHistory,
        ] {
            assert!(
                items.iter().any(|item| item.category == category),
                "missing {category:?}"
            );
        }

        let session = items
            .iter()
            .find(|item| item.source_id == "session:session1")
            .unwrap();
        assert_eq!(session.entity_kind, TimelineEntityKind::QuestSession);
        assert_eq!(session.entity_id, "session1");
        assert_eq!(session.timestamp_kind, TimelineTimestampKind::Occurred);
        assert_eq!(session.timestamp.as_str(), T2);
        assert_eq!(session.secondary_timestamp.as_ref().unwrap().as_str(), T3);
        assert_eq!(
            session.secondary_timestamp_kind,
            Some(TimelineTimestampKind::Occurred)
        );
        assert_eq!(session.summary, "Seeds prepared · Started seedlings");
        assert!(items
            .iter()
            .any(|item| item.source_id == "record:concept:c1:created"));
        assert!(items
            .iter()
            .any(|item| item.source_id == "record:skill_tree:tree1:created"));
        assert!(items
            .iter()
            .any(|item| item.source_id == "record:quest_stage:stage1:created"));
        assert!(items
            .iter()
            .any(|item| item.source_id == "record:quest_branch:branch1:created"));
        let skill_xp = items
            .iter()
            .find(|item| {
                item.category == TimelineCategory::Transaction
                    && item.title.contains("Skill XP changed")
            })
            .unwrap();
        assert_eq!(skill_xp.entity_kind, TimelineEntityKind::Skill);
        assert_eq!(skill_xp.entity_id, "skill1");
        assert!(skill_xp.summary.contains("requested 5 · applied 5 · 0 → 5"));
        let skill_unlock = items
            .iter()
            .find(|item| item.source_id == "record:skill_history:sh1")
            .unwrap();
        assert!(skill_unlock.title.contains("Skill unlocked by Rule"));
        let rule_deactivation = items
            .iter()
            .find(|item| item.source_id == "effect_history:eh2")
            .unwrap();
        assert!(rule_deactivation
            .title
            .contains("Effect deactivated by Rule"));
        assert_eq!(rule_deactivation.state.as_deref(), Some("rule_deactivated"));
        let session_without_end = items
            .iter()
            .find(|item| item.source_id == "session:session2")
            .unwrap();
        assert_eq!(session_without_end.timestamp.as_str(), T1);
        assert!(session_without_end.secondary_timestamp.is_none());
        assert!(session_without_end.secondary_timestamp_kind.is_none());

        let legacy_transaction = items
            .iter()
            .find(|item| {
                item.category == TimelineCategory::Transaction
                    && !item.title.contains("Skill XP changed")
            })
            .unwrap();
        assert_eq!(legacy_transaction.timestamp.as_str(), T2);
        assert!(legacy_transaction.secondary_timestamp.is_none());
        assert!(legacy_transaction.secondary_timestamp_kind.is_none());

        let progress = items
            .iter()
            .find(|item| item.category == TimelineCategory::ConceptProgress)
            .unwrap();
        assert_eq!(progress.timestamp_kind, TimelineTimestampKind::Occurred);
        assert_eq!(progress.secondary_timestamp.as_ref().unwrap().as_str(), T4);
        assert_eq!(
            progress.secondary_timestamp_kind,
            Some(TimelineTimestampKind::Captured)
        );

        let lifecycle = items
            .iter()
            .find(|item| item.category == TimelineCategory::Lifecycle)
            .unwrap();
        assert!(lifecycle.summary.contains("active → archived"));
        assert_eq!(
            lifecycle.secondary_timestamp_kind,
            Some(TimelineTimestampKind::Captured)
        );

        let snapshot = items
            .iter()
            .find(|item| item.category == TimelineCategory::Snapshot)
            .unwrap();
        assert_eq!(snapshot.timestamp_kind, TimelineTimestampKind::Captured);
        assert!(snapshot.title.contains("snapshot"));

        let comment = items
            .iter()
            .find(|item| item.category == TimelineCategory::Comment)
            .unwrap();
        assert_eq!(comment.entity_kind, TimelineEntityKind::Quest);
        assert_eq!(
            comment.entity_id, "q1",
            "Comment navigates to its exact target"
        );

        let related = items
            .iter()
            .filter(|item| item.category == TimelineCategory::RelationshipHistory)
            .collect::<Vec<_>>();
        assert_eq!(
            related.len(),
            4,
            "three explicit attachment creates and one explicit removal"
        );
        assert!(related
            .iter()
            .any(|item| item.timestamp_kind == TimelineTimestampKind::Removed
                && item.state.as_deref() == Some("removed")));
        let active_context = related
            .iter()
            .find(|item| item.source_id == "relationship:rel1:created")
            .unwrap()
            .relationship_context
            .as_ref()
            .unwrap();
        assert_eq!(active_context.content_id, "content1");
        assert_eq!(active_context.content_title, "Garden guide");
        assert_eq!(active_context.role_code, "guidance");
        assert!(active_context.removed_at.is_none());
        let removed_context = related
            .iter()
            .find(|item| item.source_id == "relationship:rel3:removed")
            .unwrap()
            .relationship_context
            .as_ref()
            .unwrap();
        assert_eq!(removed_context.removed_at.as_ref().unwrap().as_str(), T5);

        let effect = items
            .iter()
            .find(|item| item.source_id == "effect_history:eh1")
            .unwrap();
        assert_eq!(effect.type_code.as_deref(), Some("buff"));
        assert_eq!(effect.state.as_deref(), Some("created"));
        assert_eq!(
            items
                .iter()
                .filter(|item| item.category == TimelineCategory::EffectHistory)
                .count(),
            2,
            "only explicitly persisted Effect lifecycle facts appear; expiry creates none"
        );

        let mut tied = base_query("p1");
        tied.category = Some(TimelineCategory::Transaction);
        let tied_rows = store.query_timeline(&tied).unwrap();
        assert_eq!(tied_rows.len(), 3);
        let equal_timestamp_rows: Vec<_> = tied_rows
            .iter()
            .filter(|item| item.timestamp.as_str() == T2)
            .collect();
        assert_eq!(equal_timestamp_rows.len(), 2);
        assert!(
            equal_timestamp_rows[0].source_id < equal_timestamp_rows[1].source_id,
            "source identity deterministically breaks timestamp ties"
        );
    }

    #[test]
    fn filters_scope_dates_categories_entities_and_explicit_concept_relationships() {
        let store = fixture();

        let mut other_world = base_query("p2");
        other_world.category = Some(TimelineCategory::Transaction);
        let rows = store.query_timeline(&other_world).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows.iter().all(|item| item.player_id.as_str() == "p2"));

        let mut category = base_query("p1");
        category.category = Some(TimelineCategory::Content);
        let content = store.query_timeline(&category).unwrap();
        assert_eq!(
            content.len(),
            2,
            "one Content row contributes creation and update, not each attachment"
        );

        let mut range = base_query("p1");
        range.from = Some(Iso8601Timestamp::parse(T3).unwrap());
        range.through = Some(Iso8601Timestamp::parse(T4).unwrap());
        let ranged = store.query_timeline(&range).unwrap();
        assert!(ranged
            .iter()
            .all(|item| item.timestamp.as_str() >= T3 && item.timestamp.as_str() <= T4));
        assert!(ranged
            .iter()
            .any(|item| item.category == TimelineCategory::Comment));

        let mut kind = base_query("p1");
        kind.entity_kind = Some(TimelineEntityKind::QuestSession);
        let sessions = store.query_timeline(&kind).unwrap();
        assert_eq!(sessions.len(), 2);
        assert!(sessions.iter().any(|item| item.entity_id == "session1"));
        assert!(sessions.iter().any(|item| item.entity_id == "session2"));
        kind.entity_id = Some(EntityId::new("session1").unwrap());
        let exact_session = store.query_timeline(&kind).unwrap();
        assert_eq!(exact_session.len(), 1);
        assert_eq!(exact_session[0].source_id, "session:session1");

        let mut linked_concept = base_query("p1");
        linked_concept.category = Some(TimelineCategory::RecordChange);
        linked_concept.concept_id = Some(EntityId::new("c1").unwrap());
        let quest_changes = store.query_timeline(&linked_concept).unwrap();
        assert!(quest_changes.iter().any(|item| item.entity_id == "q1"));
        assert!(quest_changes.iter().all(|item| item.entity_id != "q2"));

        let mut concept_content = base_query("p1");
        concept_content.category = Some(TimelineCategory::Content);
        concept_content.concept_id = Some(EntityId::new("c1").unwrap());
        let attached_content = store.query_timeline(&concept_content).unwrap();
        assert_eq!(
            attached_content.len(),
            2,
            "explicit active Concept attachment makes Content relevant"
        );

        let mut concept_session = base_query("p1");
        concept_session.category = Some(TimelineCategory::Session);
        concept_session.concept_id = Some(EntityId::new("c1").unwrap());
        assert_eq!(store.query_timeline(&concept_session).unwrap().len(), 2);

        let mut ending_today = base_query("p1");
        ending_today.category = Some(TimelineCategory::Session);
        ending_today.from = Some(Iso8601Timestamp::parse(T3).unwrap());
        ending_today.through = Some(Iso8601Timestamp::parse(T3).unwrap());
        assert!(store.query_timeline(&ending_today).unwrap().is_empty());
    }

    #[test]
    fn content_relationship_rows_stay_distinct_and_paging_is_bounded() {
        let store = fixture();
        let mut attachments = base_query("p1");
        attachments.category = Some(TimelineCategory::RelationshipHistory);
        let rows = store.query_timeline(&attachments).unwrap();
        assert_eq!(rows.len(), 4);
        assert!(rows
            .iter()
            .all(|item| item.category != TimelineCategory::Content));

        store.with_conn_mut(|connection| {
            connection.execute(
                "WITH RECURSIVE seq(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM seq WHERE x<250) INSERT INTO transactions(player_id,transaction_type_code,resource,amount,occurred_at) SELECT 'p1','xp','batch',1,?1 FROM seq",
                [T2],
            ).map_err(operation)?;
            Ok(())
        }).unwrap();
        let mut page = base_query("p1");
        page.category = Some(TimelineCategory::Transaction);
        page.limit = 20;
        page.offset = 50;
        page.sort = lr_application::TimelineSort::Oldest;
        let page_rows = store.query_timeline(&page).unwrap();
        assert_eq!(page_rows.len(), 20);
        assert!(page_rows
            .windows(2)
            .all(|pair| pair[0].source_id < pair[1].source_id));

        let plan: Vec<String> = store.with_conn(|connection| {
            let mut statement = connection.prepare("EXPLAIN QUERY PLAN SELECT id FROM effect_history WHERE player_id='p1' AND recorded_at>=?1").map_err(operation)?;
            let rows = statement.query_map([T1], |row| row.get::<_, String>(3)).map_err(operation)?;
            rows.collect::<Result<_, _>>().map_err(operation)
        }).unwrap();
        assert!(
            plan.iter()
                .any(|line| line.contains("idx_timeline_effect_history_player_time")),
            "query plan should use its player/time index: {plan:?}"
        );
    }

    #[test]
    fn query_view_is_read_only_and_does_not_create_timeline_records() {
        let store = fixture();
        let before = store
            .with_conn(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM transactions", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .map_err(operation)
            })
            .unwrap();
        for _ in 0..3 {
            let rows = store.query_timeline(&base_query("p1")).unwrap();
            assert!(!rows.is_empty());
        }
        let after = store
            .with_conn(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM transactions", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .map_err(operation)
            })
            .unwrap();
        assert_eq!(before, after);
    }
}
