//! Serializable Phase 2 DTOs. Domain structs remain serialization-free.

use lr_application::{
    AwardXpOutcome, Rule as AppRule, RuleDefinition, RuleExecutionRecord, WorldOverview,
};
use lr_domain::{
    Comment, Concept, ConceptRelationship, Effect, NarrativeEntry, Player, PlayerStat,
    PlayerStateSnapshot, Quest, Skill, SkillStateSnapshot, SkillTree, StatDefinition, Transaction,
    TypeDefinition,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectTypeDto {
    pub code: String,
    pub label: String,
    pub description: Option<String>,
    pub sort_order: i32,
}
impl From<TypeDefinition> for EffectTypeDto {
    fn from(value: TypeDefinition) -> Self {
        Self {
            code: value.type_ref.code,
            label: value.label,
            description: value.description,
            sort_order: value.sort_order,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeDefinitionDto {
    pub code: String,
    pub namespace: String,
    pub label: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub is_system: bool,
    pub metadata_json: String,
}
impl From<TypeDefinition> for TypeDefinitionDto {
    fn from(value: TypeDefinition) -> Self {
        Self {
            code: value.type_ref.code,
            namespace: value.type_ref.namespace,
            label: value.label,
            description: value.description,
            sort_order: value.sort_order,
            is_active: value.is_active,
            is_system: value.is_system,
            metadata_json: value.metadata_json,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptDto {
    pub id: String,
    pub player_id: String,
    pub transfer_key: String,
    pub type_code: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}
impl From<Concept> for ConceptDto {
    fn from(v: Concept) -> Self {
        Self {
            id: v.id.to_string(),
            player_id: v.player_id.to_string(),
            transfer_key: v.transfer_key,
            type_code: v.concept_type.code,
            name: v.name,
            description: v.description,
            is_active: v.is_active,
            metadata_json: v.metadata_json,
            created_at: v.created_at.to_string(),
            updated_at: v.updated_at.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub priority: i32,
    pub definition: RuleDefinition,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}
impl From<AppRule> for RuleDto {
    fn from(v: AppRule) -> Self {
        Self {
            id: v.id.to_string(),
            name: v.name,
            description: v.description,
            enabled: v.enabled,
            priority: v.priority,
            definition: v.definition,
            metadata_json: v.metadata_json,
            created_at: v.created_at.to_string(),
            updated_at: v.updated_at.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleExecutionDto {
    pub id: String,
    pub chain_id: String,
    pub rule_id: String,
    pub event_kind: String,
    pub event_json: String,
    pub condition_passed: Option<bool>,
    pub actions_json: String,
    pub status: String,
    pub error: Option<String>,
    pub depth: u16,
    pub executed_at: String,
}
impl From<RuleExecutionRecord> for RuleExecutionDto {
    fn from(v: RuleExecutionRecord) -> Self {
        Self {
            id: v.id,
            chain_id: v.chain_id,
            rule_id: v.rule_id,
            event_kind: v.event_kind.as_str().into(),
            event_json: v.event_json,
            condition_passed: v.condition_passed,
            actions_json: v.actions_json,
            status: v.status,
            error: v.error,
            depth: v.depth,
            executed_at: v.executed_at.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub level: i32,
    pub level_name: Option<String>,
    pub progression_label: Option<String>,
    pub current_xp: i64,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}
impl From<Player> for PlayerDto {
    fn from(v: Player) -> Self {
        Self {
            id: v.id.to_string(),
            name: v.name,
            description: v.description,
            level: v.level,
            level_name: v.level_name,
            progression_label: v.progression_label,
            current_xp: v.current_xp,
            is_active: v.is_active,
            metadata_json: v.metadata_json,
            created_at: v.created_at.to_string(),
            updated_at: v.updated_at.to_string(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestDto {
    pub id: String,
    pub player_id: String,
    pub type_code: String,
    pub parent_quest_id: Option<String>,
    pub skill_id: Option<String>,
    pub title: String,
    pub status: String,
    pub difficulty: Option<i32>,
    pub progress: i32,
    pub xp_reward: i64,
    pub due_at: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub description: Option<String>,
}
impl From<Quest> for QuestDto {
    fn from(v: Quest) -> Self {
        Self {
            id: v.id.to_string(),
            player_id: v.player_id.to_string(),
            type_code: v.quest_type.code,
            parent_quest_id: v.parent_quest_id.map(|x| x.to_string()),
            skill_id: v.skill_id.map(|x| x.to_string()),
            title: v.title,
            status: v.status.as_str().into(),
            difficulty: v.difficulty,
            progress: v.progress,
            xp_reward: v.xp_reward,
            due_at: v.due_at.map(|x| x.to_string()),
            started_at: v.started_at.map(|x| x.to_string()),
            completed_at: v.completed_at.map(|x| x.to_string()),
            description: v.description,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillTreeDto {
    pub id: String,
    pub player_id: String,
    pub type_code: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
}
impl From<SkillTree> for SkillTreeDto {
    fn from(v: SkillTree) -> Self {
        Self {
            id: v.id.to_string(),
            player_id: v.player_id.to_string(),
            type_code: v.tree_type.code,
            name: v.name,
            description: v.description,
            is_active: v.is_active,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDto {
    pub id: String,
    pub skill_tree_id: String,
    pub parent_skill_id: Option<String>,
    pub type_code: String,
    pub name: String,
    pub level: i32,
    pub level_name: Option<String>,
    pub progression_label: Option<String>,
    pub current_xp: i64,
    pub invested_minutes: i64,
    pub status: String,
}
impl From<Skill> for SkillDto {
    fn from(v: Skill) -> Self {
        Self {
            id: v.id.to_string(),
            skill_tree_id: v.skill_tree_id.to_string(),
            parent_skill_id: v.parent_skill_id.map(|x| x.to_string()),
            type_code: v.skill_type.code,
            name: v.name,
            level: v.level,
            level_name: v.level_name,
            progression_label: v.progression_label,
            current_xp: v.current_xp,
            invested_minutes: v.invested_minutes,
            status: v.status.as_str().into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectDto {
    pub id: String,
    pub player_id: String,
    pub target_kind: String,
    pub target_concept_id: Option<String>,
    pub type_code: String,
    pub name: String,
    pub description: Option<String>,
    pub started_at: String,
    pub expires_at: Option<String>,
    pub deactivated_at: Option<String>,
    pub intensity: i32,
}
impl From<Effect> for EffectDto {
    fn from(v: Effect) -> Self {
        Self {
            id: v.id.to_string(),
            player_id: v.player_id.to_string(),
            target_kind: match v.target_kind() {
                lr_domain::EffectTargetKind::Player => "player",
                lr_domain::EffectTargetKind::Concept => "concept",
            }
            .into(),
            target_concept_id: v.target_concept_id.map(|id| id.to_string()),
            type_code: v.effect_type.code,
            name: v.name,
            description: v.description,
            started_at: v.started_at.to_string(),
            expires_at: v.expires_at.map(|x| x.to_string()),
            deactivated_at: v.deactivated_at.map(|x| x.to_string()),
            intensity: v.intensity,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDto {
    pub id: Option<i64>,
    pub player_id: String,
    pub type_code: String,
    pub resource: String,
    pub amount: i64,
    pub applied_amount: Option<i64>,
    pub occurred_at: String,
    pub captured_at: Option<String>,
    pub reason: Option<String>,
    pub description: Option<String>,
    pub source_kind: Option<String>,
    pub source_id: Option<String>,
}
impl From<Transaction> for TransactionDto {
    fn from(v: Transaction) -> Self {
        Self {
            id: v.id,
            player_id: v.player_id.to_string(),
            type_code: v.transaction_type.code,
            resource: v.resource,
            amount: v.amount,
            applied_amount: v.applied_amount,
            occurred_at: v.occurred_at.to_string(),
            captured_at: v.captured_at.map(|t| t.to_string()),
            reason: v.reason,
            description: v.description,
            source_kind: v.source_kind,
            source_id: v.source_id,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSnapshotDto {
    pub id: Option<i64>,
    pub player_id: String,
    pub snapshot_date: String,
    pub level: i32,
    pub current_xp: i64,
    pub state_json: String,
    pub created_at: String,
}
impl From<PlayerStateSnapshot> for PlayerSnapshotDto {
    fn from(v: PlayerStateSnapshot) -> Self {
        Self {
            id: v.id,
            player_id: v.player_id.to_string(),
            snapshot_date: v.snapshot_date.to_string(),
            level: v.level,
            current_xp: v.current_xp,
            state_json: v.state_json,
            created_at: v.created_at.to_string(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentDto {
    pub id: Option<i64>,
    pub author_player_id: Option<String>,
    pub target_kind: String,
    pub target_id: String,
    pub body: String,
    pub created_at: String,
}
impl From<Comment> for CommentDto {
    fn from(v: Comment) -> Self {
        Self {
            id: v.id,
            author_player_id: v.author_player_id.map(|x| x.to_string()),
            target_kind: v.target_kind.as_str().into(),
            target_id: v.target_id.to_string(),
            body: v.body,
            created_at: v.created_at.to_string(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeEntryDto {
    pub id: String,
    pub player_id: String,
    pub kind: String,
    pub title: String,
    pub content: String,
    pub author: Option<String>,
    pub source_kind: Option<String>,
    pub source_id: Option<String>,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}
impl From<NarrativeEntry> for NarrativeEntryDto {
    fn from(v: NarrativeEntry) -> Self {
        Self {
            id: v.id.to_string(),
            player_id: v.player_id.to_string(),
            kind: v.kind.code,
            title: v.title,
            content: v.content,
            author: v.author,
            source_kind: v.source_kind,
            source_id: v.source_id,
            is_active: v.is_active,
            metadata_json: v.metadata_json,
            created_at: v.created_at.to_string(),
            updated_at: v.updated_at.to_string(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwardXpOutcomeDto {
    pub player: PlayerDto,
    pub transaction: TransactionDto,
}
impl From<AwardXpOutcome> for AwardXpOutcomeDto {
    fn from(v: AwardXpOutcome) -> Self {
        Self {
            player: v.player.into(),
            transaction: v.transaction.into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldOverviewDto {
    pub player: PlayerDto,
    pub quests: Vec<QuestDto>,
    pub skill_trees: Vec<SkillTreeDto>,
    pub skills: Vec<SkillDto>,
    pub effects: Vec<EffectDto>,
    pub recent_transactions: Vec<TransactionDto>,
    pub narratives: Vec<NarrativeEntryDto>,
}
impl From<WorldOverview> for WorldOverviewDto {
    fn from(v: WorldOverview) -> Self {
        Self {
            player: v.player.into(),
            quests: v.quests.into_iter().map(Into::into).collect(),
            skill_trees: v.skill_trees.into_iter().map(Into::into).collect(),
            skills: v.skills.into_iter().map(Into::into).collect(),
            effects: v.effects.into_iter().map(Into::into).collect(),
            recent_transactions: v.recent_transactions.into_iter().map(Into::into).collect(),
            narratives: v.narratives.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSnapshotDto {
    pub id: Option<i64>,
    pub skill_id: String,
    pub snapshot_date: String,
    pub level: i32,
    pub current_xp: i64,
    pub status: String,
    pub invested_minutes: i64,
    pub state_json: String,
    pub created_at: String,
}
impl From<SkillStateSnapshot> for SkillSnapshotDto {
    fn from(v: SkillStateSnapshot) -> Self {
        Self {
            id: v.id,
            skill_id: v.skill_id.to_string(),
            snapshot_date: v.snapshot_date.to_string(),
            level: v.level,
            current_xp: v.current_xp,
            status: v.status.as_str().into(),
            invested_minutes: v.invested_minutes,
            state_json: v.state_json,
            created_at: v.created_at.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatDefinitionDto {
    pub id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub unit: Option<String>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub is_active: bool,
}
impl From<StatDefinition> for StatDefinitionDto {
    fn from(v: StatDefinition) -> Self {
        Self {
            id: v.id.to_string(),
            code: v.code,
            name: v.name,
            description: v.description,
            unit: v.unit,
            minimum: v.minimum,
            maximum: v.maximum,
            is_active: v.is_active,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStatDto {
    pub player_id: String,
    pub stat_code: String,
    pub current_value: f64,
    pub updated_at: String,
}
impl From<PlayerStat> for PlayerStatDto {
    fn from(v: PlayerStat) -> Self {
        Self {
            player_id: v.player_id.to_string(),
            stat_code: v.stat_code,
            current_value: v.current_value,
            updated_at: v.updated_at.to_string(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptRelationshipDto {
    pub id: String,
    pub player_id: String,
    pub source_concept_id: String,
    pub target_concept_id: String,
    pub relationship_code: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}
impl From<ConceptRelationship> for ConceptRelationshipDto {
    fn from(v: ConceptRelationship) -> Self {
        Self {
            id: v.id.to_string(),
            player_id: v.player_id.to_string(),
            source_concept_id: v.source_concept_id.to_string(),
            target_concept_id: v.target_concept_id.to_string(),
            relationship_code: v.relationship_type.code,
            is_active: v.is_active,
            created_at: v.created_at.to_string(),
            updated_at: v.updated_at.to_string(),
        }
    }
}
