use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillHistoryKind {
    AvailabilityChanged,
    ControlChanged,
}
impl SkillHistoryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AvailabilityChanged => "availability_changed",
            Self::ControlChanged => "control_changed",
        }
    }
    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "availability_changed" => Ok(Self::AvailabilityChanged),
            "control_changed" => Ok(Self::ControlChanged),
            _ => Err(DomainError::invalid_value(
                "Skill history kind",
                "unknown value",
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillHistorySource {
    Manual,
    Rule,
}
impl SkillHistorySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Rule => "rule",
        }
    }
    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "manual" => Ok(Self::Manual),
            "rule" => Ok(Self::Rule),
            _ => Err(DomainError::invalid_value(
                "Skill history source",
                "unknown value",
            )),
        }
    }
}

/// A bounded, append-only fact about Skill availability or its explicit Rule
/// authority policy. XP changes remain in the Transaction ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillHistoryEntry {
    pub id: EntityId,
    pub player_id: EntityId,
    pub skill_id: EntityId,
    pub kind: SkillHistoryKind,
    pub source: SkillHistorySource,
    pub recorded_at: Iso8601Timestamp,
    pub previous_state_json: Option<String>,
    pub current_state_json: String,
}
