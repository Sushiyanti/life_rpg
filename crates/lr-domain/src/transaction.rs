//! Append-only generic transaction ledger.

use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp, TypeRef};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub id: Option<i64>,
    pub player_id: EntityId,
    pub transaction_type: TypeRef,
    pub resource: String,
    pub amount: i64,
    pub occurred_at: Iso8601Timestamp,
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
            occurred_at,
            reason: None,
            description: None,
            source_kind: None,
            source_id: None,
            metadata_json: "{}".into(),
        })
    }
}
