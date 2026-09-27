use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectHistoryKind {
    Created,
    DetailsChanged,
    ExpiryChanged,
    ManuallyDeactivated,
    SessionLinked,
    SessionUnlinked,
}
impl EffectHistoryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::DetailsChanged => "details_changed",
            Self::ExpiryChanged => "expiry_changed",
            Self::ManuallyDeactivated => "manually_deactivated",
            Self::SessionLinked => "session_linked",
            Self::SessionUnlinked => "session_unlinked",
        }
    }
    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "created" => Ok(Self::Created),
            "details_changed" => Ok(Self::DetailsChanged),
            "expiry_changed" => Ok(Self::ExpiryChanged),
            "manually_deactivated" => Ok(Self::ManuallyDeactivated),
            "session_linked" => Ok(Self::SessionLinked),
            "session_unlinked" => Ok(Self::SessionUnlinked),
            _ => Err(DomainError::invalid_value(
                "Effect history kind",
                "unsupported event",
            )),
        }
    }
}

/// An append-only fact about an Effect. Snapshots are explicit before/after
/// images; expiry itself remains a recorded timestamp, not a synthetic event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectHistoryEntry {
    pub id: EntityId,
    pub player_id: EntityId,
    pub effect_id: EntityId,
    pub session_id: Option<EntityId>,
    pub kind: EffectHistoryKind,
    pub recorded_at: Iso8601Timestamp,
    pub previous_state_json: Option<String>,
    pub current_state_json: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEffectRole {
    Relevant,
    Applied,
    Removed,
    Observed,
}
impl SessionEffectRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Relevant => "relevant",
            Self::Applied => "applied",
            Self::Removed => "removed",
            Self::Observed => "observed",
        }
    }
    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "relevant" => Ok(Self::Relevant),
            "applied" => Ok(Self::Applied),
            "removed" => Ok(Self::Removed),
            "observed" => Ok(Self::Observed),
            _ => Err(DomainError::invalid_value(
                "Session Effect role",
                "unsupported role",
            )),
        }
    }
}

/// A typed, explicitly recorded Session-to-Effect context link. Removing the
/// link only sets `removed_at`; it never changes either referenced record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEffect {
    pub id: EntityId,
    pub player_id: EntityId,
    pub session_id: EntityId,
    pub effect_id: EntityId,
    pub role: SessionEffectRole,
    pub added_at: Iso8601Timestamp,
    pub removed_at: Option<Iso8601Timestamp>,
}
