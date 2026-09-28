//! Generic effects: buffs, debuffs, conditions, and temporary modifiers.

use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp, TypeRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectLifecycle {
    Scheduled,
    Active,
    Expired,
    ManuallyDeactivated,
    RuleDeactivated,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectDeactivationSource {
    Manual,
    Rule,
}
impl EffectDeactivationSource {
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
                "Effect deactivation source",
                "unknown value",
            )),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectTargetKind {
    Player,
    Concept,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    pub id: EntityId,
    pub player_id: EntityId,
    /// When present, the Effect targets this Concept rather than the Player.
    pub target_concept_id: Option<EntityId>,
    pub effect_type: TypeRef,
    pub name: String,
    pub description: Option<String>,
    pub started_at: Iso8601Timestamp,
    pub expires_at: Option<Iso8601Timestamp>,
    pub deactivated_at: Option<Iso8601Timestamp>,
    pub deactivation_source: Option<EffectDeactivationSource>,
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
            target_concept_id: None,
            effect_type,
            name: name.trim().into(),
            description: None,
            started_at: started_at.clone(),
            expires_at: None,
            deactivated_at: None,
            deactivation_source: None,
            intensity: 1,
            source_kind: None,
            source_id: None,
            metadata_json: "{}".into(),
            created_at: started_at.clone(),
            updated_at: started_at,
        })
    }
    pub fn target_kind(&self) -> EffectTargetKind {
        if self.target_concept_id.is_some() {
            EffectTargetKind::Concept
        } else {
            EffectTargetKind::Player
        }
    }
    pub fn target_concept(&mut self, concept_id: EntityId) {
        self.target_concept_id = Some(concept_id);
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
            (Some(deactivation), Some(expiration)) => {
                return if deactivation < expiration {
                    self.deactivation_lifecycle()
                } else {
                    EffectLifecycle::Expired
                }
            }
            (Some(_), None) => return self.deactivation_lifecycle(),
            (None, Some(_)) => return EffectLifecycle::Expired,
            (None, None) => {}
        }
        EffectLifecycle::Active
    }
    pub fn deactivate(&mut self, now: Iso8601Timestamp) -> DomainResult<()> {
        self.deactivate_as(now, EffectDeactivationSource::Manual)
    }
    pub fn deactivate_by_rule(&mut self, now: Iso8601Timestamp) -> DomainResult<()> {
        self.deactivate_as(now, EffectDeactivationSource::Rule)
    }
    fn deactivation_lifecycle(&self) -> EffectLifecycle {
        match self.deactivation_source {
            Some(EffectDeactivationSource::Rule) => EffectLifecycle::RuleDeactivated,
            Some(EffectDeactivationSource::Manual) | None => EffectLifecycle::ManuallyDeactivated,
        }
    }
    fn deactivate_as(
        &mut self,
        now: Iso8601Timestamp,
        source: EffectDeactivationSource,
    ) -> DomainResult<()> {
        if self.deactivated_at.is_some() {
            return Err(DomainError::Invariant(
                "effect is already deactivated".into(),
            ));
        }
        if now < self.started_at {
            return Err(DomainError::Invariant(
                "effect cannot be deactivated before it starts".into(),
            ));
        }
        if self
            .expires_at
            .as_ref()
            .is_some_and(|expires_at| expires_at <= &now)
        {
            return Err(DomainError::Invariant(
                "expired Effect cannot be manually deactivated".into(),
            ));
        }
        self.deactivated_at = Some(now.clone());
        self.deactivation_source = Some(source);
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

    #[test]
    fn expired_effect_cannot_be_manually_deactivated_at_or_after_expiry() {
        let mut e = effect();
        e.expires_at = Some(time("2026-09-27T00:00:00Z"));

        for now in ["2026-09-27T00:00:00Z", "2026-09-28T00:00:00Z"] {
            assert_eq!(e.lifecycle_at(&time(now)), EffectLifecycle::Expired);
            assert!(e.deactivate(time(now)).is_err());
            assert_eq!(e.deactivated_at, None);
        }
    }

    #[test]
    fn rule_deactivation_is_distinct_and_cannot_target_scheduled_or_expired_effects() {
        let now = time("2026-09-26T00:00:00Z");
        let mut rule_effect = effect();
        rule_effect.deactivate_by_rule(now.clone()).unwrap();
        assert_eq!(
            rule_effect.deactivation_source,
            Some(EffectDeactivationSource::Rule)
        );
        assert_eq!(
            rule_effect.lifecycle_at(&now),
            EffectLifecycle::RuleDeactivated
        );

        let mut manual_effect = effect();
        manual_effect.deactivate(now.clone()).unwrap();
        assert_eq!(
            manual_effect.deactivation_source,
            Some(EffectDeactivationSource::Manual)
        );
        assert_eq!(
            manual_effect.lifecycle_at(&now),
            EffectLifecycle::ManuallyDeactivated
        );

        let mut scheduled = effect();
        scheduled.started_at = time("2026-09-27T00:00:00Z");
        assert_eq!(scheduled.lifecycle_at(&now), EffectLifecycle::Scheduled);
        assert!(scheduled.deactivate_by_rule(now.clone()).is_err());
        assert!(scheduled.deactivate(now.clone()).is_err());
        assert_eq!(scheduled.deactivated_at, None);

        let mut expired = effect();
        expired.expires_at = Some(time("2026-09-26T00:00:00Z"));
        assert_eq!(expired.lifecycle_at(&now), EffectLifecycle::Expired);
        assert!(expired.deactivate_by_rule(now.clone()).is_err());
        assert!(expired.deactivate(now).is_err());
        assert_eq!(
            expired.deactivated_at, None,
            "expiry is derived and never synthesized as deactivation"
        );
    }
}
