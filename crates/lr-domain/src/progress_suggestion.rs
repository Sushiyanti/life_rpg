//! Suggestions are persisted proposals, never progress mutations by themselves.
use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionStatus {
    Pending,
    Accepted,
    Rejected,
}
impl SuggestionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }
    pub fn parse(v: &str) -> DomainResult<Self> {
        match v {
            "pending" => Ok(Self::Pending),
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            _ => Err(DomainError::invalid_value(
                "suggestion status",
                "unknown status",
            )),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressSuggestion {
    pub id: EntityId,
    pub player_id: EntityId,
    pub concept_id: EntityId,
    pub track_code: String,
    pub proposed_value: f64,
    pub proposed_level: Option<i32>,
    pub reason: Option<String>,
    pub source: String,
    pub status: SuggestionStatus,
    pub created_at: Iso8601Timestamp,
    pub resolved_at: Option<Iso8601Timestamp>,
    pub metadata_json: String,
}
impl ProgressSuggestion {
    pub fn new(
        id: EntityId,
        player_id: EntityId,
        concept_id: EntityId,
        track_code: impl Into<String>,
        proposed_value: f64,
        proposed_level: Option<i32>,
        reason: Option<String>,
        source: impl Into<String>,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        let track_code = track_code.into();
        let source = source.into();
        if track_code.trim().is_empty()
            || track_code.len() > 160
            || !proposed_value.is_finite()
            || proposed_level.is_some_and(|v| v < 1)
            || source.trim().is_empty()
            || source.len() > 160
        {
            return Err(DomainError::invalid_value(
                "progress suggestion",
                "requires a valid track, finite proposal, source, and optional positive level",
            ));
        }
        if reason.as_ref().is_some_and(|s| s.chars().count() > 512) {
            return Err(DomainError::invalid_value(
                "suggestion reason",
                "must be at most 512 characters",
            ));
        }
        Ok(Self {
            id,
            player_id,
            concept_id,
            track_code,
            proposed_value,
            proposed_level,
            reason,
            source,
            status: SuggestionStatus::Pending,
            created_at: now,
            resolved_at: None,
            metadata_json: "{}".into(),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn suggestion_is_proposal_not_track() {
        let t = Iso8601Timestamp::parse("2026-09-27T00:00:00Z").unwrap();
        let s = ProgressSuggestion::new(
            EntityId::new("s").unwrap(),
            EntityId::new("p").unwrap(),
            EntityId::new("c").unwrap(),
            "mastery",
            72.0,
            None,
            None,
            "player",
            t,
        )
        .unwrap();
        assert_eq!(s.status, SuggestionStatus::Pending);
        assert_eq!(s.proposed_value, 72.0);
    }
}
