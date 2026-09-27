//! Generic effects: buffs, debuffs, conditions, and temporary modifiers.

use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp, TypeRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectLifecycle {
    Scheduled,
    Active,
    Expired,
    ManuallyDeactivated,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    pub id: EntityId,
    pub player_id: EntityId,
    pub effect_type: TypeRef,
    pub name: String,
    pub description: Option<String>,
    pub started_at: Iso8601Timestamp,
    pub expires_at: Option<Iso8601Timestamp>,
    pub deactivated_at: Option<Iso8601Timestamp>,
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
            deactivated_at: None,
            intensity: 1,
            source_kind: None,
            source_id: None,
            metadata_json: "{}".into(),
            created_at: started_at.clone(),
            updated_at: started_at,
        })
    }
    pub fn is_active_at(&self, now: &Iso8601Timestamp) -> bool {
        self.lifecycle_at(now) == EffectLifecycle::Active
    }
    pub fn lifecycle_at(&self, now: &Iso8601Timestamp) -> EffectLifecycle {
        if self.started_at > *now {
            return EffectLifecycle::Scheduled;
        }
        let deactivated = self.deactivated_at.as_ref().filter(|at| *at <= now);
        let expired = self.expires_at.as_ref().filter(|at| *at <= now);
        match (deactivated, expired) {
            (Some(manual), Some(expiration)) => {
                return if manual < expiration {
                    EffectLifecycle::ManuallyDeactivated
                } else {
                    EffectLifecycle::Expired
                }
            }
            (Some(_), None) => return EffectLifecycle::ManuallyDeactivated,
            (None, Some(_)) => return EffectLifecycle::Expired,
            (None, None) => {}
        }
        EffectLifecycle::Active
    }
    pub fn deactivate(&mut self, now: Iso8601Timestamp) -> DomainResult<()> {
        if self.deactivated_at.is_some() {
            return Err(DomainError::Invariant(
                "effect is already manually deactivated".into(),
            ));
        }
        if now < self.started_at {
            return Err(DomainError::Invariant(
                "effect cannot be deactivated before it starts".into(),
            ));
        }
        self.deactivated_at = Some(now.clone());
        self.updated_at = now;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn time(s: &str) -> Iso8601Timestamp {
        Iso8601Timestamp::parse(s).unwrap()
    }
    fn effect() -> Effect {
        Effect::new(
            EntityId::new("e1").unwrap(),
            EntityId::new("p1").unwrap(),
            TypeRef::effect("buff").unwrap(),
            "Focus",
            time("2026-09-25T00:00:00Z"),
        )
        .unwrap()
    }
    #[test]
    fn lifecycle_distinguishes_scheduled_active_expired_and_manual_off() {
        let mut e = effect();
        e.expires_at = Some(time("2026-09-27T00:00:00Z"));
        assert_eq!(
            e.lifecycle_at(&time("2026-09-24T23:59:00Z")),
            EffectLifecycle::Scheduled
        );
        assert_eq!(
            e.lifecycle_at(&time("2026-09-26T00:00:00Z")),
            EffectLifecycle::Active
        );
        assert_eq!(
            e.lifecycle_at(&time("2026-09-28T00:00:00Z")),
            EffectLifecycle::Expired
        );
        e.deactivate(time("2026-09-26T12:00:00Z")).unwrap();
        assert_eq!(
            e.lifecycle_at(&time("2026-09-26T13:00:00Z")),
            EffectLifecycle::ManuallyDeactivated
        );
        assert_eq!(
            e.lifecycle_at(&time("2026-09-28T00:00:00Z")),
            EffectLifecycle::ManuallyDeactivated
        );
        assert!(e.deactivate(time("2026-09-26T14:00:00Z")).is_err());
    }
}
