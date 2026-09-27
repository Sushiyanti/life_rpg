//! Declarative, versioned rule vocabulary. These types contain data only; they
//! never contain source code or executable callbacks.
use lr_domain::{
    DomainError, DomainResult, EntityId, Iso8601Timestamp, Player, PlayerStat, Quest, QuestStatus,
    Transaction,
};
use serde::{Deserialize, Serialize};

pub const RULE_SCHEMA_VERSION: u16 = 1;
pub const MAX_CONDITION_DEPTH: usize = 8;
pub const MAX_CONDITION_NODES: usize = 128;
pub const MAX_CONDITIONS_PER_GROUP: usize = 16;
pub const MAX_ACTIONS_PER_RULE: usize = 16;
pub const MAX_RULE_CHAIN_DEPTH: usize = 8;
pub const MAX_ACTIONS_PER_CHAIN: usize = 32;
pub const MAX_RULE_EVALUATIONS_PER_CHAIN: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    QuestCompleted,
    PlayerXpChanged,
    StatChanged,
}
impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::QuestCompleted => "quest_completed",
            Self::PlayerXpChanged => "player_xp_changed",
            Self::StatChanged => "stat_changed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuleEvent {
    QuestCompleted {
        player_id: String,
        quest_id: String,
        quest_type: String,
        progress: i32,
        xp_reward: i64,
    },
    PlayerXpChanged {
        player_id: String,
        previous_xp: i64,
        current_xp: i64,
        requested_amount: i64,
        applied_amount: i64,
        player_level: i32,
    },
    StatChanged {
        player_id: String,
        stat_code: String,
        previous_value: Option<f64>,
        current_value: f64,
    },
}
impl RuleEvent {
    pub fn kind(&self) -> EventKind {
        match self {
            Self::QuestCompleted { .. } => EventKind::QuestCompleted,
            Self::PlayerXpChanged { .. } => EventKind::PlayerXpChanged,
            Self::StatChanged { .. } => EventKind::StatChanged,
        }
    }
    pub fn player_id(&self) -> &str {
        match self {
            Self::QuestCompleted { player_id, .. }
            | Self::PlayerXpChanged { player_id, .. }
            | Self::StatChanged { player_id, .. } => player_id,
        }
    }
    pub(crate) fn loop_key(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|_| format!("{}:{}", self.kind().as_str(), self.player_id()))
    }
    fn number(&self, subject: NumericSubject) -> Option<f64> {
        match (self, subject) {
            (Self::QuestCompleted { progress, .. }, NumericSubject::QuestProgress) => {
                Some(*progress as f64)
            }
            (Self::QuestCompleted { xp_reward, .. }, NumericSubject::QuestXpReward) => {
                Some(*xp_reward as f64)
            }
            (Self::PlayerXpChanged { previous_xp, .. }, NumericSubject::PreviousXp) => {
                Some(*previous_xp as f64)
            }
            (Self::PlayerXpChanged { current_xp, .. }, NumericSubject::CurrentXp) => {
                Some(*current_xp as f64)
            }
            (
                Self::PlayerXpChanged {
                    requested_amount, ..
                },
                NumericSubject::RequestedAmount,
            ) => Some(*requested_amount as f64),
            (Self::PlayerXpChanged { applied_amount, .. }, NumericSubject::AppliedAmount) => {
                Some(*applied_amount as f64)
            }
            (Self::PlayerXpChanged { player_level, .. }, NumericSubject::PlayerLevel) => {
                Some(*player_level as f64)
            }
            (Self::StatChanged { current_value, .. }, NumericSubject::StatValue) => {
                Some(*current_value)
            }
            _ => None,
        }
    }
    fn text<'a>(&'a self, subject: TextSubject) -> Option<&'a str> {
        match (self, subject) {
            (Self::QuestCompleted { quest_type, .. }, TextSubject::QuestType) => Some(quest_type),
            (Self::StatChanged { stat_code, .. }, TextSubject::StatCode) => Some(stat_code),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    Equal,
    NotEqual,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
}
impl Comparison {
    fn test(self, left: f64, right: f64) -> bool {
        match self {
            Self::Equal => left == right,
            Self::NotEqual => left != right,
            Self::Greater => left > right,
            Self::GreaterOrEqual => left >= right,
            Self::Less => left < right,
            Self::LessOrEqual => left <= right,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumericSubject {
    PreviousXp,
    CurrentXp,
    RequestedAmount,
    AppliedAmount,
    PlayerLevel,
    QuestProgress,
    QuestXpReward,
    StatValue,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextSubject {
    QuestType,
    StatCode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuleCondition {
    Always,
    EventKindIs {
        kind: EventKind,
    },
    NumberCompare {
        subject: NumericSubject,
        comparison: Comparison,
        value: f64,
    },
    TextCompare {
        subject: TextSubject,
        comparison: TextComparison,
        value: String,
    },
    All {
        conditions: Vec<RuleCondition>,
    },
    Any {
        conditions: Vec<RuleCondition>,
    },
    Not {
        condition: Box<RuleCondition>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextComparison {
    Equal,
    NotEqual,
}
impl RuleCondition {
    pub fn evaluate(&self, event: &RuleEvent) -> bool {
        self.evaluate_known(event).unwrap_or(false)
    }
    fn evaluate_known(&self, event: &RuleEvent) -> Option<bool> {
        match self {
            Self::Always => Some(true),
            Self::EventKindIs { kind } => Some(event.kind() == *kind),
            Self::NumberCompare {
                subject,
                comparison,
                value,
            } => event.number(*subject).map(|v| comparison.test(v, *value)),
            Self::TextCompare {
                subject,
                comparison,
                value,
            } => event.text(*subject).map(|v| match comparison {
                TextComparison::Equal => v == value,
                TextComparison::NotEqual => v != value,
            }),
            Self::All { conditions } => {
                let values: Vec<_> = conditions.iter().map(|c| c.evaluate_known(event)).collect();
                if values.iter().any(|v| *v == Some(false)) {
                    Some(false)
                } else if values.iter().all(Option::is_some) {
                    Some(true)
                } else {
                    None
                }
            }
            Self::Any { conditions } => {
                let values: Vec<_> = conditions.iter().map(|c| c.evaluate_known(event)).collect();
                if values.iter().any(|v| *v == Some(true)) {
                    Some(true)
                } else if values.iter().all(Option::is_some) {
                    Some(false)
                } else {
                    None
                }
            }
            Self::Not { condition } => condition.evaluate_known(event).map(|value| !value),
        }
    }
    fn validate(&self, depth: usize, trigger: EventKind, nodes: &mut usize) -> DomainResult<()> {
        *nodes += 1;
        if *nodes > MAX_CONDITION_NODES {
            return Err(DomainError::invalid_value(
                "rule condition",
                "maximum total node count exceeded",
            ));
        }
        if depth > MAX_CONDITION_DEPTH {
            return Err(DomainError::invalid_value(
                "rule condition",
                "maximum nesting depth exceeded",
            ));
        }
        match self {
            Self::NumberCompare { value, .. } if !value.is_finite() => Err(
                DomainError::invalid_value("rule condition number", "must be finite"),
            ),
            Self::NumberCompare { subject, .. }
                if !matches!(
                    (trigger, subject),
                    (
                        EventKind::QuestCompleted,
                        NumericSubject::QuestProgress | NumericSubject::QuestXpReward
                    ) | (
                        EventKind::PlayerXpChanged,
                        NumericSubject::PreviousXp
                            | NumericSubject::CurrentXp
                            | NumericSubject::RequestedAmount
                            | NumericSubject::AppliedAmount
                            | NumericSubject::PlayerLevel
                    ) | (EventKind::StatChanged, NumericSubject::StatValue)
                ) =>
            {
                Err(DomainError::invalid_value(
                    "rule condition subject",
                    format!("{subject:?} is not available for {:?}", trigger),
                ))
            }
            Self::TextCompare { value, .. } if value.trim().is_empty() || value.len() > 512 => {
                Err(DomainError::invalid_value(
                    "rule condition text",
                    "must contain at most 512 nonblank bytes",
                ))
            }
            Self::TextCompare { subject, .. }
                if !matches!(
                    (trigger, subject),
                    (EventKind::QuestCompleted, TextSubject::QuestType)
                        | (EventKind::StatChanged, TextSubject::StatCode)
                ) =>
            {
                Err(DomainError::invalid_value(
                    "rule condition subject",
                    format!("{subject:?} is not available for {:?}", trigger),
                ))
            }
            Self::All { conditions } | Self::Any { conditions } => {
                if conditions.is_empty() || conditions.len() > MAX_CONDITIONS_PER_GROUP {
                    return Err(DomainError::invalid_value(
                        "rule condition",
                        format!("ALL/ANY requires 1 to {MAX_CONDITIONS_PER_GROUP} children"),
                    ));
                }
                for c in conditions {
                    c.validate(depth + 1, trigger, nodes)?;
                }
                Ok(())
            }
            Self::Not { condition } => condition.validate(depth + 1, trigger, nodes),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RuleAction {
    AwardXp { amount: i64, reason: Option<String> },
    CompleteQuest { quest_id: String },
    SetPlayerStat { stat_code: String, value: f64 },
    ModifyPlayerStat { stat_code: String, delta: f64 },
}
impl RuleAction {
    fn validate(&self) -> DomainResult<()> {
        match self {
            Self::AwardXp { amount, .. } if *amount == 0 => Err(DomainError::invalid_value(
                "rule XP action",
                "amount must not be zero",
            )),
            Self::AwardXp {
                reason: Some(reason),
                ..
            } if reason.len() > 512 => Err(DomainError::invalid_value(
                "rule XP reason",
                "must be at most 512 bytes",
            )),
            Self::CompleteQuest { quest_id }
                if quest_id.trim().is_empty() || quest_id.len() > 160 =>
            {
                Err(DomainError::invalid_value(
                    "rule Quest action",
                    "quest id must contain 1 to 160 bytes",
                ))
            }
            Self::SetPlayerStat { stat_code, value }
                if stat_code.trim().is_empty() || stat_code.len() > 160 || !value.is_finite() =>
            {
                Err(DomainError::invalid_value(
                    "rule stat action",
                    "requires a stat code and finite value",
                ))
            }
            Self::ModifyPlayerStat { stat_code, delta }
                if stat_code.trim().is_empty() || stat_code.len() > 160 || !delta.is_finite() =>
            {
                Err(DomainError::invalid_value(
                    "rule stat action",
                    "requires a stat code and finite delta",
                ))
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuleDefinition {
    pub schema_version: u16,
    pub trigger: EventKind,
    pub condition: RuleCondition,
    pub actions: Vec<RuleAction>,
}
impl RuleDefinition {
    pub fn validate(&self) -> DomainResult<()> {
        if self.schema_version != RULE_SCHEMA_VERSION {
            return Err(DomainError::invalid_value(
                "rule schema version",
                format!("unsupported version {}", self.schema_version),
            ));
        }
        if self.actions.is_empty() || self.actions.len() > MAX_ACTIONS_PER_RULE {
            return Err(DomainError::invalid_value(
                "rule actions",
                format!("requires 1 to {MAX_ACTIONS_PER_RULE} actions"),
            ));
        }
        let mut condition_nodes = 0;
        self.condition
            .validate(0, self.trigger, &mut condition_nodes)?;
        for action in &self.actions {
            action.validate()?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub id: EntityId,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub priority: i32,
    pub definition: RuleDefinition,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl Rule {
    pub fn new(
        id: EntityId,
        name: impl Into<String>,
        description: Option<String>,
        priority: i32,
        trigger: EventKind,
        condition: RuleCondition,
        actions: Vec<RuleAction>,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        let name = name.into();
        let name = name.trim();
        if name.is_empty() {
            return Err(DomainError::invalid_value("rule name", "must not be blank"));
        }
        if name.len() > 160 {
            return Err(DomainError::invalid_value(
                "rule name",
                "must be at most 160 bytes",
            ));
        }
        if description.as_ref().is_some_and(|text| text.len() > 2000) {
            return Err(DomainError::invalid_value(
                "rule description",
                "must be at most 2000 bytes",
            ));
        }
        let definition = RuleDefinition {
            schema_version: RULE_SCHEMA_VERSION,
            trigger,
            condition,
            actions,
        };
        definition.validate()?;
        Ok(Self {
            id,
            name: name.to_owned(),
            description: description.filter(|s| !s.trim().is_empty()),
            enabled: true,
            priority,
            definition,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuleExecutionRecord {
    pub id: String,
    pub chain_id: String,
    pub rule_id: String,
    pub event_kind: EventKind,
    pub event_json: String,
    pub condition_passed: Option<bool>,
    pub actions_json: String,
    pub status: String,
    pub error: Option<String>,
    pub depth: u16,
    pub executed_at: Iso8601Timestamp,
}
#[derive(Debug, Clone, PartialEq)]
pub enum RuleOperation {
    CompleteQuest {
        quest: Quest,
        expected_status: QuestStatus,
        reward: Option<(Player, Transaction)>,
    },
    PlayerXp {
        player: Player,
        previous_xp: i64,
        transaction: Transaction,
    },
    PlayerStat {
        stat: PlayerStat,
        expected_previous: Option<f64>,
    },
}
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum RuleExecutionError {
    #[error("rule chain exceeded maximum depth ({0})")]
    MaxDepth(usize),
    #[error("rule chain exceeded maximum action count ({0})")]
    MaxActions(usize),
    #[error("rule chain exceeded maximum evaluation count ({0})")]
    MaxEvaluations(usize),
    #[error("rule/event loop detected at rule `{0}` for `{1}`")]
    LoopDetected(String, String),
    #[error("rule `{rule_id}` action {action_index} failed: {message}")]
    ActionFailed {
        rule_id: String,
        action_index: usize,
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    fn quest_event() -> RuleEvent {
        RuleEvent::QuestCompleted {
            player_id: "p1".into(),
            quest_id: "q1".into(),
            quest_type: "main".into(),
            progress: 100,
            xp_reward: 0,
        }
    }
    #[test]
    fn typed_numeric_text_and_logical_conditions_are_deterministic() {
        let event = quest_event();
        let numeric = RuleCondition::NumberCompare {
            subject: NumericSubject::QuestProgress,
            comparison: Comparison::GreaterOrEqual,
            value: 100.0,
        };
        let text = RuleCondition::TextCompare {
            subject: TextSubject::QuestType,
            comparison: TextComparison::Equal,
            value: "main".into(),
        };
        assert!(RuleCondition::All {
            conditions: vec![numeric.clone(), text.clone()]
        }
        .evaluate(&event));
        assert!(RuleCondition::Any {
            conditions: vec![
                RuleCondition::EventKindIs {
                    kind: EventKind::StatChanged
                },
                numeric.clone()
            ]
        }
        .evaluate(&event));
        assert!(!RuleCondition::Not {
            condition: Box::new(text)
        }
        .evaluate(&event));
        assert!(!RuleCondition::NumberCompare {
            subject: NumericSubject::StatValue,
            comparison: Comparison::Less,
            value: 20.0
        }
        .evaluate(&event));
        assert!(RuleCondition::EventKindIs {
            kind: EventKind::QuestCompleted
        }
        .evaluate(&event));
        assert!(!RuleCondition::EventKindIs {
            kind: EventKind::PlayerXpChanged
        }
        .evaluate(&event));
        let missing = RuleCondition::NumberCompare {
            subject: NumericSubject::StatValue,
            comparison: Comparison::Less,
            value: 20.0,
        };
        assert!(!missing.evaluate(&event));
        assert!(
            !RuleCondition::Not {
                condition: Box::new(missing.clone())
            }
            .evaluate(&event),
            "NOT over a missing event subject must fail closed"
        );
        assert!(!RuleCondition::Any {
            conditions: vec![
                missing.clone(),
                RuleCondition::NumberCompare {
                    subject: NumericSubject::PlayerLevel,
                    comparison: Comparison::Greater,
                    value: 99.0
                }
            ]
        }
        .evaluate(&event));
        assert!(
            RuleCondition::All {
                conditions: vec![
                    missing,
                    RuleCondition::TextCompare {
                        subject: TextSubject::QuestType,
                        comparison: TextComparison::Equal,
                        value: "other".into()
                    }
                ]
            }
            .evaluate(&event)
                == false
        );
    }
    #[test]
    fn definitions_reject_bad_versions_unsafe_shapes_and_invalid_references() {
        let invalid = RuleDefinition {
            schema_version: 99,
            trigger: EventKind::QuestCompleted,
            condition: RuleCondition::Always,
            actions: vec![RuleAction::AwardXp {
                amount: 10,
                reason: None,
            }],
        };
        assert!(invalid.validate().is_err());
        let empty = RuleDefinition {
            schema_version: RULE_SCHEMA_VERSION,
            trigger: EventKind::QuestCompleted,
            condition: RuleCondition::All { conditions: vec![] },
            actions: vec![RuleAction::AwardXp {
                amount: 10,
                reason: None,
            }],
        };
        assert!(empty.validate().is_err());
        let zero = RuleDefinition {
            schema_version: RULE_SCHEMA_VERSION,
            trigger: EventKind::QuestCompleted,
            condition: RuleCondition::Always,
            actions: vec![RuleAction::AwardXp {
                amount: 0,
                reason: None,
            }],
        };
        assert!(zero.validate().is_err());
        let wire = serde_json::json!({"schemaVersion":1,"trigger":"quest_completed","condition":{"op":"run_code","source":"danger"},"actions":[{"kind":"award_xp","amount":10}]});
        assert!(serde_json::from_value::<RuleDefinition>(wire).is_err());
        let nan = RuleDefinition {
            schema_version: RULE_SCHEMA_VERSION,
            trigger: EventKind::QuestCompleted,
            condition: RuleCondition::NumberCompare {
                subject: NumericSubject::QuestProgress,
                comparison: Comparison::Greater,
                value: f64::NAN,
            },
            actions: vec![RuleAction::AwardXp {
                amount: 10,
                reason: None,
            }],
        };
        assert!(nan.validate().is_err());
        let invalid_reference = RuleDefinition {
            schema_version: RULE_SCHEMA_VERSION,
            trigger: EventKind::QuestCompleted,
            condition: RuleCondition::NumberCompare {
                subject: NumericSubject::CurrentXp,
                comparison: Comparison::Greater,
                value: 0.0,
            },
            actions: vec![RuleAction::AwardXp {
                amount: 1,
                reason: None,
            }],
        };
        assert!(
            invalid_reference.validate().is_err(),
            "a Quest trigger cannot compare Player XP because that value is absent"
        );
        let too_wide = RuleDefinition {
            schema_version: RULE_SCHEMA_VERSION,
            trigger: EventKind::QuestCompleted,
            condition: RuleCondition::Any {
                conditions: vec![RuleCondition::Always; MAX_CONDITIONS_PER_GROUP + 1],
            },
            actions: vec![RuleAction::AwardXp {
                amount: 1,
                reason: None,
            }],
        };
        assert!(too_wide.validate().is_err());
        let mut too_deep = RuleCondition::Always;
        for _ in 0..=MAX_CONDITION_DEPTH {
            too_deep = RuleCondition::Not {
                condition: Box::new(too_deep),
            };
        }
        let too_deep = RuleDefinition {
            schema_version: RULE_SCHEMA_VERSION,
            trigger: EventKind::QuestCompleted,
            condition: too_deep,
            actions: vec![RuleAction::AwardXp {
                amount: 1,
                reason: None,
            }],
        };
        assert!(too_deep.validate().is_err());
        let too_long_reason = RuleDefinition {
            schema_version: RULE_SCHEMA_VERSION,
            trigger: EventKind::QuestCompleted,
            condition: RuleCondition::Always,
            actions: vec![RuleAction::AwardXp {
                amount: 1,
                reason: Some("x".repeat(513)),
            }],
        };
        assert!(too_long_reason.validate().is_err());
    }
}
