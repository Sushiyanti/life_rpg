//! Application ports: storage requirements expressed without SQLite or Tauri.

use crate::error::StorageError;
use crate::rules::{EventKind, Rule, RuleExecutionRecord, RuleOperation};
use lr_domain::{
    Comment, CommentTargetKind, DateValue, Effect, EntityId, Iso8601Timestamp, NarrativeEntry,
    Player, PlayerStat, PlayerStateSnapshot, Quest, Skill, SkillStateSnapshot, SkillTree,
    StatDefinition, Transaction, TypeDefinition,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRecord {
    pub version: u32,
    pub name: String,
    pub applied: bool,
    pub applied_at: Option<String>,
}
impl MigrationRecord {
    pub fn state(&self) -> &'static str {
        if self.applied {
            "applied"
        } else {
            "pending"
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaReport {
    pub current_version: u32,
    pub expected_version: u32,
    pub migrations: Vec<MigrationRecord>,
}
impl SchemaReport {
    pub fn is_current(&self) -> bool {
        self.current_version == self.expected_version
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreDiagnostics {
    pub backend: String,
    pub location_hint: Option<String>,
    pub journal_mode: String,
    pub foreign_keys: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundTripProof {
    pub token: String,
    pub read_back_token: String,
    pub row_id: i64,
    pub probe_rows: u64,
    pub written_at: String,
}
impl RoundTripProof {
    pub fn matches(&self) -> bool {
        self.token == self.read_back_token
    }
}

/// Foundation status-store port.
pub trait HealthStore: Send + Sync {
    fn diagnostics(&self) -> Result<StoreDiagnostics, StorageError>;
    fn schema_report(&self) -> Result<SchemaReport, StorageError>;
    fn verify_round_trip(
        &self,
        token: &str,
        written_at: &str,
    ) -> Result<RoundTripProof, StorageError>;
}

/// Persistent world port. Methods use domain values and application errors only.
/// Compound mutation methods must be implemented atomically by adapters.
pub trait WorldStore: Send + Sync {
    fn create_rule(&self, _: &Rule) -> Result<(), StorageError> {
        Err(StorageError::Operation(
            "rules are not supported by this store".into(),
        ))
    }
    fn get_rule(&self, _: &EntityId) -> Result<Option<Rule>, StorageError> {
        Err(StorageError::Operation(
            "rules are not supported by this store".into(),
        ))
    }
    fn list_rules(&self) -> Result<Vec<Rule>, StorageError> {
        Ok(Vec::new())
    }
    fn list_rules_for_event(&self, _: EventKind) -> Result<Vec<Rule>, StorageError> {
        Ok(Vec::new())
    }
    fn update_rule(&self, _: &Rule) -> Result<(), StorageError> {
        Err(StorageError::Operation(
            "rules are not supported by this store".into(),
        ))
    }
    fn list_rule_executions(&self, _: u32) -> Result<Vec<RuleExecutionRecord>, StorageError> {
        Ok(Vec::new())
    }
    fn record_rule_execution(&self, _: &RuleExecutionRecord) -> Result<(), StorageError> {
        Err(StorageError::Operation(
            "rule history is not supported by this store".into(),
        ))
    }
    fn record_rule_executions(&self, _: &[RuleExecutionRecord]) -> Result<(), StorageError> {
        Err(StorageError::Operation(
            "rule history is not supported by this store".into(),
        ))
    }
    fn apply_rule_chain(
        &self,
        _: &[RuleOperation],
        _: &[RuleExecutionRecord],
    ) -> Result<Vec<RuleOperation>, StorageError> {
        Err(StorageError::Operation(
            "rule execution is not supported by this store".into(),
        ))
    }
    fn list_type_definitions(
        &self,
        namespace: Option<&str>,
    ) -> Result<Vec<TypeDefinition>, StorageError>;
    fn create_type_definition(
        &self,
        definition: &TypeDefinition,
    ) -> Result<TypeDefinition, StorageError>;

    fn create_player(&self, player: &Player) -> Result<(), StorageError>;
    fn get_player(&self, id: &EntityId) -> Result<Option<Player>, StorageError>;
    fn update_player(&self, player: &Player) -> Result<(), StorageError>;
    fn create_stat_definition(&self, _: &StatDefinition) -> Result<(), StorageError> {
        Err(StorageError::Operation(
            "stat definitions are not supported by this store".into(),
        ))
    }
    fn list_stat_definitions(&self) -> Result<Vec<StatDefinition>, StorageError> {
        Err(StorageError::Operation(
            "stat definitions are not supported by this store".into(),
        ))
    }
    fn set_player_stat(&self, _: &PlayerStat) -> Result<(), StorageError> {
        Err(StorageError::Operation(
            "player stats are not supported by this store".into(),
        ))
    }
    fn list_player_stats(&self, _: &EntityId) -> Result<Vec<PlayerStat>, StorageError> {
        Err(StorageError::Operation(
            "player stats are not supported by this store".into(),
        ))
    }

    fn insert_quest(&self, quest: &Quest) -> Result<(), StorageError>;
    fn get_quest(&self, id: &EntityId) -> Result<Option<Quest>, StorageError>;
    fn update_quest(&self, quest: &Quest) -> Result<(), StorageError>;
    fn list_quests(&self, player_id: &EntityId) -> Result<Vec<Quest>, StorageError>;

    fn insert_skill_tree(&self, tree: &SkillTree) -> Result<(), StorageError>;
    fn get_skill_tree(&self, id: &EntityId) -> Result<Option<SkillTree>, StorageError>;
    fn list_skill_trees(&self, player_id: &EntityId) -> Result<Vec<SkillTree>, StorageError>;
    fn insert_skill(&self, skill: &Skill) -> Result<(), StorageError>;
    fn get_skill(&self, id: &EntityId) -> Result<Option<Skill>, StorageError>;
    fn update_skill(&self, skill: &Skill) -> Result<(), StorageError>;
    fn list_skills(&self, tree_id: &EntityId) -> Result<Vec<Skill>, StorageError>;

    fn insert_effect(&self, effect: &Effect) -> Result<(), StorageError>;
    fn list_effects(
        &self,
        player_id: &EntityId,
        active_at: Option<&str>,
    ) -> Result<Vec<Effect>, StorageError>;
    fn deactivate_effect(
        &self,
        _: &EntityId,
        _: &Iso8601Timestamp,
    ) -> Result<Effect, StorageError> {
        Err(StorageError::Operation(
            "effect deactivation is not supported by this store".into(),
        ))
    }

    fn append_transaction(&self, tx: &Transaction) -> Result<Transaction, StorageError>;
    fn list_transactions(
        &self,
        player_id: &EntityId,
        limit: u32,
    ) -> Result<Vec<Transaction>, StorageError>;
    fn transaction_total(&self, player_id: &EntityId, resource: &str) -> Result<i64, StorageError>;

    fn insert_player_snapshot(&self, snapshot: &PlayerStateSnapshot) -> Result<(), StorageError>;
    fn capture_player_snapshot(
        &self,
        _player_id: &EntityId,
        _snapshot_date: &DateValue,
        _created_at: &Iso8601Timestamp,
    ) -> Result<PlayerStateSnapshot, StorageError> {
        Err(StorageError::Operation(
            "player snapshots are not supported by this store".into(),
        ))
    }
    fn list_player_snapshots(
        &self,
        player_id: &EntityId,
    ) -> Result<Vec<PlayerStateSnapshot>, StorageError>;
    fn insert_skill_snapshot(&self, snapshot: &SkillStateSnapshot) -> Result<(), StorageError>;
    fn capture_skill_snapshot(
        &self,
        _skill_id: &EntityId,
        _snapshot_date: &DateValue,
        _created_at: &Iso8601Timestamp,
    ) -> Result<SkillStateSnapshot, StorageError> {
        Err(StorageError::Operation(
            "skill snapshots are not supported by this store".into(),
        ))
    }
    fn list_skill_snapshots(&self, _: &EntityId) -> Result<Vec<SkillStateSnapshot>, StorageError> {
        Err(StorageError::Operation(
            "skill snapshots are not supported by this store".into(),
        ))
    }

    fn add_comment(&self, comment: &Comment) -> Result<Comment, StorageError>;
    fn list_comments(
        &self,
        kind: CommentTargetKind,
        target_id: &EntityId,
    ) -> Result<Vec<Comment>, StorageError>;
    fn insert_narrative_entry(&self, entry: &NarrativeEntry) -> Result<(), StorageError>;
    fn list_narrative_entries(
        &self,
        player_id: &EntityId,
    ) -> Result<Vec<NarrativeEntry>, StorageError>;

    fn award_xp(&self, player: &Player, tx: &Transaction) -> Result<Transaction, StorageError>;
    fn complete_quest(
        &self,
        quest: &Quest,
        player: Option<&Player>,
        reward: Option<&Transaction>,
    ) -> Result<Option<Transaction>, StorageError>;
    fn invest_skill_time(
        &self,
        skill: &Skill,
        tx: &Transaction,
    ) -> Result<Transaction, StorageError>;
}

/// Injected source of time keeps services deterministic under test.
pub trait Clock: Send + Sync {
    fn now_rfc3339(&self) -> String;
    fn now_unix_nanos(&self) -> u128;
}

// The shell shares one SQLite store with health and world services.
impl<T: HealthStore + ?Sized> HealthStore for std::sync::Arc<T> {
    fn diagnostics(&self) -> Result<StoreDiagnostics, StorageError> {
        (**self).diagnostics()
    }
    fn schema_report(&self) -> Result<SchemaReport, StorageError> {
        (**self).schema_report()
    }
    fn verify_round_trip(
        &self,
        token: &str,
        written_at: &str,
    ) -> Result<RoundTripProof, StorageError> {
        (**self).verify_round_trip(token, written_at)
    }
}
impl<T: WorldStore + ?Sized> WorldStore for std::sync::Arc<T> {
    fn create_rule(&self, r: &Rule) -> Result<(), StorageError> {
        (**self).create_rule(r)
    }
    fn get_rule(&self, id: &EntityId) -> Result<Option<Rule>, StorageError> {
        (**self).get_rule(id)
    }
    fn list_rules(&self) -> Result<Vec<Rule>, StorageError> {
        (**self).list_rules()
    }
    fn list_rules_for_event(&self, k: EventKind) -> Result<Vec<Rule>, StorageError> {
        (**self).list_rules_for_event(k)
    }
    fn update_rule(&self, r: &Rule) -> Result<(), StorageError> {
        (**self).update_rule(r)
    }
    fn list_rule_executions(&self, l: u32) -> Result<Vec<RuleExecutionRecord>, StorageError> {
        (**self).list_rule_executions(l)
    }
    fn record_rule_execution(&self, r: &RuleExecutionRecord) -> Result<(), StorageError> {
        (**self).record_rule_execution(r)
    }
    fn record_rule_executions(&self, r: &[RuleExecutionRecord]) -> Result<(), StorageError> {
        (**self).record_rule_executions(r)
    }
    fn apply_rule_chain(
        &self,
        o: &[RuleOperation],
        r: &[RuleExecutionRecord],
    ) -> Result<Vec<RuleOperation>, StorageError> {
        (**self).apply_rule_chain(o, r)
    }
    fn list_type_definitions(&self, n: Option<&str>) -> Result<Vec<TypeDefinition>, StorageError> {
        (**self).list_type_definitions(n)
    }
    fn create_type_definition(&self, d: &TypeDefinition) -> Result<TypeDefinition, StorageError> {
        (**self).create_type_definition(d)
    }
    fn create_player(&self, p: &Player) -> Result<(), StorageError> {
        (**self).create_player(p)
    }
    fn get_player(&self, id: &EntityId) -> Result<Option<Player>, StorageError> {
        (**self).get_player(id)
    }
    fn update_player(&self, p: &Player) -> Result<(), StorageError> {
        (**self).update_player(p)
    }
    fn create_stat_definition(&self, d: &StatDefinition) -> Result<(), StorageError> {
        (**self).create_stat_definition(d)
    }
    fn list_stat_definitions(&self) -> Result<Vec<StatDefinition>, StorageError> {
        (**self).list_stat_definitions()
    }
    fn set_player_stat(&self, s: &PlayerStat) -> Result<(), StorageError> {
        (**self).set_player_stat(s)
    }
    fn list_player_stats(&self, id: &EntityId) -> Result<Vec<PlayerStat>, StorageError> {
        (**self).list_player_stats(id)
    }
    fn insert_quest(&self, q: &Quest) -> Result<(), StorageError> {
        (**self).insert_quest(q)
    }
    fn get_quest(&self, id: &EntityId) -> Result<Option<Quest>, StorageError> {
        (**self).get_quest(id)
    }
    fn update_quest(&self, q: &Quest) -> Result<(), StorageError> {
        (**self).update_quest(q)
    }
    fn list_quests(&self, id: &EntityId) -> Result<Vec<Quest>, StorageError> {
        (**self).list_quests(id)
    }
    fn insert_skill_tree(&self, t: &SkillTree) -> Result<(), StorageError> {
        (**self).insert_skill_tree(t)
    }
    fn get_skill_tree(&self, id: &EntityId) -> Result<Option<SkillTree>, StorageError> {
        (**self).get_skill_tree(id)
    }
    fn list_skill_trees(&self, id: &EntityId) -> Result<Vec<SkillTree>, StorageError> {
        (**self).list_skill_trees(id)
    }
    fn insert_skill(&self, s: &Skill) -> Result<(), StorageError> {
        (**self).insert_skill(s)
    }
    fn get_skill(&self, id: &EntityId) -> Result<Option<Skill>, StorageError> {
        (**self).get_skill(id)
    }
    fn update_skill(&self, s: &Skill) -> Result<(), StorageError> {
        (**self).update_skill(s)
    }
    fn list_skills(&self, id: &EntityId) -> Result<Vec<Skill>, StorageError> {
        (**self).list_skills(id)
    }
    fn insert_effect(&self, e: &Effect) -> Result<(), StorageError> {
        (**self).insert_effect(e)
    }
    fn list_effects(&self, id: &EntityId, a: Option<&str>) -> Result<Vec<Effect>, StorageError> {
        (**self).list_effects(id, a)
    }
    fn deactivate_effect(
        &self,
        id: &EntityId,
        at: &Iso8601Timestamp,
    ) -> Result<Effect, StorageError> {
        (**self).deactivate_effect(id, at)
    }
    fn append_transaction(&self, t: &Transaction) -> Result<Transaction, StorageError> {
        (**self).append_transaction(t)
    }
    fn list_transactions(&self, id: &EntityId, l: u32) -> Result<Vec<Transaction>, StorageError> {
        (**self).list_transactions(id, l)
    }
    fn transaction_total(&self, id: &EntityId, r: &str) -> Result<i64, StorageError> {
        (**self).transaction_total(id, r)
    }
    fn insert_player_snapshot(&self, s: &PlayerStateSnapshot) -> Result<(), StorageError> {
        (**self).insert_player_snapshot(s)
    }
    fn capture_player_snapshot(
        &self,
        id: &EntityId,
        date: &DateValue,
        at: &Iso8601Timestamp,
    ) -> Result<PlayerStateSnapshot, StorageError> {
        (**self).capture_player_snapshot(id, date, at)
    }
    fn list_player_snapshots(
        &self,
        id: &EntityId,
    ) -> Result<Vec<PlayerStateSnapshot>, StorageError> {
        (**self).list_player_snapshots(id)
    }
    fn insert_skill_snapshot(&self, s: &SkillStateSnapshot) -> Result<(), StorageError> {
        (**self).insert_skill_snapshot(s)
    }
    fn capture_skill_snapshot(
        &self,
        id: &EntityId,
        date: &DateValue,
        at: &Iso8601Timestamp,
    ) -> Result<SkillStateSnapshot, StorageError> {
        (**self).capture_skill_snapshot(id, date, at)
    }
    fn list_skill_snapshots(&self, id: &EntityId) -> Result<Vec<SkillStateSnapshot>, StorageError> {
        (**self).list_skill_snapshots(id)
    }
    fn add_comment(&self, c: &Comment) -> Result<Comment, StorageError> {
        (**self).add_comment(c)
    }
    fn list_comments(
        &self,
        k: CommentTargetKind,
        id: &EntityId,
    ) -> Result<Vec<Comment>, StorageError> {
        (**self).list_comments(k, id)
    }
    fn insert_narrative_entry(&self, n: &NarrativeEntry) -> Result<(), StorageError> {
        (**self).insert_narrative_entry(n)
    }
    fn list_narrative_entries(&self, id: &EntityId) -> Result<Vec<NarrativeEntry>, StorageError> {
        (**self).list_narrative_entries(id)
    }
    fn award_xp(&self, p: &Player, t: &Transaction) -> Result<Transaction, StorageError> {
        (**self).award_xp(p, t)
    }
    fn complete_quest(
        &self,
        q: &Quest,
        p: Option<&Player>,
        t: Option<&Transaction>,
    ) -> Result<Option<Transaction>, StorageError> {
        (**self).complete_quest(q, p, t)
    }
    fn invest_skill_time(&self, s: &Skill, t: &Transaction) -> Result<Transaction, StorageError> {
        (**self).invest_skill_time(s, t)
    }
}
