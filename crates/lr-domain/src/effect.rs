//! Generic effects: buffs, debuffs, conditions, and temporary modifiers.

use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp, TypeRef};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    pub id: EntityId,
    pub player_id: EntityId,
    pub effect_type: TypeRef,
    pub name: String,
    pub description: Option<String>,
    pub started_at: Iso8601Timestamp,
    pub expires_at: Option<Iso8601Timestamp>,
    pub intensity: i32,
    pub source_kind: Option<String>,
    pub source_id: Option<String>,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl Effect {
    pub fn new(
        id: EntityId,
        player_id: EntityId,
        effect_type: TypeRef,
        name: impl Into<String>,
        started_at: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        let name = name.into();
        if effect_type.namespace != "effect" {
            return Err(DomainError::Invariant(
                "effect type must use `effect` namespace".into(),
            ));
        }
        if name.trim().is_empty() {
            return Err(DomainError::invalid_value(
                "effect name",
                "must not be blank",
            ));
        }
        Ok(Self {
            id,
            player_id,
            effect_type,
            name: name.trim().into(),
            description: None,
            started_at: started_at.clone(),
            expires_at: None,
            intensity: 1,
            source_kind: None,
            source_id: None,
            metadata_json: "{}".into(),
            created_at: started_at.clone(),
            updated_at: started_at,
        })
    }
    pub fn is_active_at(&self, now: &Iso8601Timestamp) -> bool {
        self.expires_at.as_ref().is_none_or(|expires| expires > now)
    }
}
