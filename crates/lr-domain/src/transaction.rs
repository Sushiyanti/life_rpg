//! Append-only generic transaction ledger.

use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp, TypeRef};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub id: Option<i64>,
    pub player_id: EntityId,
    pub transaction_type: TypeRef,
    pub resource: String,
    pub amount: i64,
    /// Actual state delta for XP events. `amount` remains the requested event.
    pub applied_amount: Option<i64>,
    pub occurred_at: Iso8601Timestamp,
    /// When this installation recorded the event; legacy rows may not know it.
    pub captured_at: Option<Iso8601Timestamp>,
    pub reason: Option<String>,
    pub description: Option<String>,
    pub source_kind: Option<String>,
    pub source_id: Option<String>,
    pub metadata_json: String,
}
impl Transaction {
    pub fn new(
        player_id: EntityId,
        transaction_type: TypeRef,
        resource: impl Into<String>,
        amount: i64,
        occurred_at: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        let resource = resource.into();
        let resource = resource.trim();
        if transaction_type.namespace != "transaction" {
            return Err(DomainError::Invariant(
                "transaction type must use `transaction` namespace".into(),
            ));
        }
        if resource.is_empty() {
            return Err(DomainError::invalid_value(
                "transaction resource",
                "must not be blank",
            ));
        }
        if amount == 0 {
            return Err(DomainError::invalid_value(
                "transaction amount",
                "must not be zero",
            ));
        }
        Ok(Self {
            id: None,
            player_id,
            transaction_type,
            resource: resource.to_string(),
            amount,
            applied_amount: (resource == "xp").then_some(amount),
            captured_at: Some(occurred_at.clone()),
            occurred_at,
            reason: None,
            description: None,
            source_kind: None,
            source_id: None,
            metadata_json: "{}".into(),
        })
    }

    /// Override the default capture moment when the event's occurrence time is supplied separately.
    pub fn with_capture_time(mut self, captured_at: Iso8601Timestamp) -> DomainResult<Self> {
        if captured_at < self.occurred_at {
            return Err(DomainError::Invariant(
                "transaction occurred_at cannot be after captured_at".into(),
            ));
        }
        self.captured_at = Some(captured_at);
        Ok(self)
    }

    /// Construct the ledger event for a requested XP adjustment, including a
    /// zero-applied penalty when the player's XP is already at the floor.
    pub fn xp_adjustment(
        player_id: EntityId,
        requested: i64,
        applied: i64,
        occurred_at: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        if requested == 0 {
            return Err(DomainError::invalid_value(
                "requested XP adjustment",
                "must not be zero",
            ));
        }
        if (requested > 0 && applied != requested)
            || (requested < 0 && (applied > 0 || applied < requested))
        {
            return Err(DomainError::Invariant(
                "applied XP delta must match an award or be a clamped penalty".into(),
            ));
        }
        let mut tx = Self::new(
            player_id,
            TypeRef::transaction("xp")?,
            "xp",
            requested,
            occurred_at,
        )?;
        tx.applied_amount = Some(applied);
        Ok(tx)
    }

    /// Ledger one Skill XP mutation using a distinct resource while retaining
    /// requested-versus-applied floor semantics.
    pub fn skill_xp_adjustment(
        player_id: EntityId,
        skill_id: &EntityId,
        requested: i64,
        applied: i64,
        occurred_at: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        if requested == 0 {
            return Err(DomainError::invalid_value(
                "requested Skill XP adjustment",
                "must not be zero",
            ));
        }
        if (requested > 0 && applied != requested)
            || (requested < 0 && (applied > 0 || applied < requested))
        {
            return Err(DomainError::Invariant(
                "applied Skill XP delta must match an award or be a clamped penalty".into(),
            ));
        }
        let mut tx = Self::new(
            player_id,
            TypeRef::transaction("xp")?,
            "skill_xp",
            requested,
            occurred_at,
        )?;
        tx.applied_amount = Some(applied);
        tx.source_kind = Some("skill".into());
        tx.source_id = Some(skill_id.to_string());
        Ok(tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transaction_capture_cannot_precede_the_occurred_event() {
        let occurred = Iso8601Timestamp::parse("2026-09-27T10:00:00Z").unwrap();
        let captured = Iso8601Timestamp::parse("2026-09-27T10:01:00Z").unwrap();
        let transaction = Transaction::new(
            EntityId::new("player-1").unwrap(),
            TypeRef::transaction("xp").unwrap(),
            "xp",
            1,
            occurred,
        )
        .unwrap();
        assert!(transaction.clone().with_capture_time(captured).is_ok());
        assert!(transaction
            .with_capture_time(Iso8601Timestamp::parse("2026-09-27T09:59:00Z").unwrap())
            .is_err());
    }
}
