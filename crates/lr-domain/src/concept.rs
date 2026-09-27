//! Meaningful world subjects, typed relationships, and non-universal progression tracks.
use crate::{DateValue, DomainError, DomainResult, EntityId, Iso8601Timestamp, TypeRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConceptEntityKind {
    Quest,
    Skill,
    SkillTree,
    Effect,
    Transaction,
    Comment,
    NarrativeEntry,
}
impl ConceptEntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Quest => "quest",
            Self::Skill => "skill",
            Self::SkillTree => "skill_tree",
            Self::Effect => "effect",
            Self::Transaction => "transaction",
            Self::Comment => "comment",
            Self::NarrativeEntry => "narrative_entry",
        }
    }
    pub fn parse(v: &str) -> DomainResult<Self> {
        match v {
            "quest" => Ok(Self::Quest),
            "skill" => Ok(Self::Skill),
            "skill_tree" => Ok(Self::SkillTree),
            "effect" => Ok(Self::Effect),
            "transaction" => Ok(Self::Transaction),
            "comment" => Ok(Self::Comment),
            "narrative_entry" => Ok(Self::NarrativeEntry),
            _ => Err(DomainError::invalid_value(
                "Concept entity kind",
                "unsupported related entity kind",
            )),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptEntityLink {
    pub concept_id: EntityId,
    pub player_id: EntityId,
    pub entity_kind: ConceptEntityKind,
    pub entity_id: String,
    pub created_at: Iso8601Timestamp,
}

fn non_blank(field: &'static str, value: impl Into<String>) -> DomainResult<String> {
    let value = value.into();
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 512 {
        return Err(DomainError::invalid_value(
            field,
            "must contain 1 to 512 characters",
        ));
    }
    Ok(value.to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Concept {
    pub id: EntityId,
    pub player_id: EntityId,
    pub concept_type: TypeRef,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl Concept {
    pub fn new(
        id: EntityId,
        player_id: EntityId,
        concept_type: TypeRef,
        name: impl Into<String>,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        if concept_type.namespace != "concept" {
            return Err(DomainError::Invariant(
                "concept type must use `concept` namespace".into(),
            ));
        }
        Ok(Self {
            id,
            player_id,
            concept_type,
            name: non_blank("concept name", name)?,
            description: None,
            is_active: true,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptRelationship {
    pub id: EntityId,
    pub player_id: EntityId,
    pub source_concept_id: EntityId,
    pub target_concept_id: EntityId,
    pub relationship_type: TypeRef,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl ConceptRelationship {
    pub fn new(
        id: EntityId,
        player_id: EntityId,
        source: EntityId,
        target: EntityId,
        relationship_type: TypeRef,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        if source == target {
            return Err(DomainError::Invariant(
                "a Concept cannot relate to itself".into(),
            ));
        }
        if relationship_type.namespace != "concept_relationship" {
            return Err(DomainError::Invariant(
                "relationship type must use `concept_relationship` namespace".into(),
            ));
        }
        Ok(Self {
            id,
            player_id,
            source_concept_id: source,
            target_concept_id: target,
            relationship_type,
            is_active: true,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
}

/// Whether Rules may write the current progress value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressControl {
    Manual,
    RuleControlled,
}
impl ProgressControl {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::RuleControlled => "rule_controlled",
        }
    }
    pub fn parse(v: &str) -> DomainResult<Self> {
        match v {
            "manual" => Ok(Self::Manual),
            "rule_controlled" => Ok(Self::RuleControlled),
            _ => Err(DomainError::invalid_value(
                "progress control",
                "unknown control mode",
            )),
        }
    }
}

/// Progress semantics are explicit: these measures are not interchangeable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressSemantics {
    Numeric,
    Percentage,
    Experience,
    Level,
    Mastery,
}
impl ProgressSemantics {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Numeric => "numeric",
            Self::Percentage => "percentage",
            Self::Experience => "experience",
            Self::Level => "level",
            Self::Mastery => "mastery",
        }
    }
    pub fn parse(v: &str) -> DomainResult<Self> {
        match v {
            "numeric" => Ok(Self::Numeric),
            "percentage" => Ok(Self::Percentage),
            "experience" => Ok(Self::Experience),
            "level" => Ok(Self::Level),
            "mastery" => Ok(Self::Mastery),
            _ => Err(DomainError::invalid_value(
                "progress semantics",
                "unknown semantics",
            )),
        }
    }
    fn validate(self, value: f64, min: Option<f64>, max: Option<f64>) -> DomainResult<()> {
        if !value.is_finite() {
            return Err(DomainError::invalid_value(
                "progress value",
                "must be finite",
            ));
        }
        if min.is_some_and(|v| value < v) || max.is_some_and(|v| value > v) {
            return Err(DomainError::invalid_value(
                "progress value",
                "outside track bounds",
            ));
        }
        match self {
            Self::Percentage if !(0.0..=100.0).contains(&value) => Err(DomainError::invalid_value(
                "percentage progress",
                "must be between 0 and 100",
            )),
            Self::Experience if value < 0.0 || value.fract() != 0.0 => {
                Err(DomainError::invalid_value(
                    "experience progress",
                    "must be a nonnegative whole number",
                ))
            }
            Self::Level if value < 1.0 || value.fract() != 0.0 => Err(DomainError::invalid_value(
                "level progress",
                "must be a positive whole number",
            )),
            _ => Ok(()),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressTrackDefinition {
    pub id: EntityId,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub semantics: ProgressSemantics,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl ProgressTrackDefinition {
    pub fn new(
        id: EntityId,
        code: impl Into<String>,
        name: impl Into<String>,
        semantics: ProgressSemantics,
        minimum: Option<f64>,
        maximum: Option<f64>,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        let code = code.into();
        if code.is_empty()
            || code.len() > 160
            || !code
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        {
            return Err(DomainError::invalid_value(
                "progress track code",
                "use 1-160 lowercase ASCII letters, digits, or underscores",
            ));
        }
        let name = non_blank("progress track name", name)?;
        if minimum.is_some_and(|v| !v.is_finite())
            || maximum.is_some_and(|v| !v.is_finite())
            || matches!((minimum, maximum), (Some(a), Some(b)) if a > b)
        {
            return Err(DomainError::invalid_value(
                "progress track bounds",
                "must be finite and ordered",
            ));
        }
        if semantics == ProgressSemantics::Percentage
            && (minimum.is_some_and(|x| x < 0.0) || maximum.is_some_and(|x| x > 100.0))
        {
            return Err(DomainError::invalid_value(
                "percentage track bounds",
                "must fit within 0..=100",
            ));
        }
        Ok(Self {
            id,
            code,
            name,
            description: None,
            semantics,
            minimum,
            maximum,
            is_active: true,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
    pub fn validate_value(&self, value: f64) -> DomainResult<()> {
        self.semantics.validate(value, self.minimum, self.maximum)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct ConceptProgressTrack {
    pub id: EntityId,
    pub concept_id: EntityId,
    pub track_code: String,
    pub current_value: f64,
    pub level: Option<i32>,
    pub level_name: Option<String>,
    pub progression_label: Option<String>,
    pub control: ProgressControl,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl ConceptProgressTrack {
    pub fn new(
        id: EntityId,
        concept_id: EntityId,
        definition: &ProgressTrackDefinition,
        value: f64,
        level: Option<i32>,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        if !definition.is_active {
            return Err(DomainError::Invariant(
                "inactive progress definition cannot be used".into(),
            ));
        }
        definition.validate_value(value)?;
        if level.is_some_and(|l| l < 1) {
            return Err(DomainError::invalid_value(
                "progress level",
                "must be positive",
            ));
        }
        Ok(Self {
            id,
            concept_id,
            track_code: definition.code.clone(),
            current_value: value,
            level,
            level_name: None,
            progression_label: None,
            control: ProgressControl::Manual,
            is_active: true,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
    pub fn change(
        &mut self,
        definition: &ProgressTrackDefinition,
        value: f64,
        level: Option<i32>,
        now: Iso8601Timestamp,
    ) -> DomainResult<()> {
        if definition.code != self.track_code {
            return Err(DomainError::Invariant(
                "progress definition does not match the track".into(),
            ));
        }
        if !definition.is_active {
            return Err(DomainError::Invariant(
                "inactive progress definition cannot be used".into(),
            ));
        }
        definition.validate_value(value)?;
        if level.is_some_and(|l| l < 1) {
            return Err(DomainError::invalid_value(
                "progress level",
                "must be positive",
            ));
        }
        self.current_value = value;
        self.level = level;
        self.updated_at = now;
        Ok(())
    }
    pub fn set_control(&mut self, control: ProgressControl, now: Iso8601Timestamp) {
        self.control = control;
        self.updated_at = now;
    }
    pub fn set_labels(
        &mut self,
        level_name: Option<String>,
        progression_label: Option<String>,
        now: Iso8601Timestamp,
    ) -> DomainResult<()> {
        self.level_name = level_name.map(|v| non_blank("level name", v)).transpose()?;
        self.progression_label = progression_label
            .map(|v| non_blank("progression label", v))
            .transpose()?;
        self.updated_at = now;
        Ok(())
    }
}

/// Immutable event for one progress change. occurred_at is when it happened;
/// captured_at is when this installation persisted the observation.
#[derive(Debug, Clone, PartialEq)]
pub struct ConceptProgressEntry {
    pub id: EntityId,
    pub concept_id: EntityId,
    pub track_code: String,
    pub previous_value: Option<f64>,
    pub current_value: f64,
    pub level: Option<i32>,
    pub occurred_at: Iso8601Timestamp,
    pub captured_at: Iso8601Timestamp,
    pub metadata_json: String,
}
impl ConceptProgressEntry {
    pub fn new(
        id: EntityId,
        track: &ConceptProgressTrack,
        previous_value: Option<f64>,
        occurred_at: Iso8601Timestamp,
        captured_at: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        if !track.current_value.is_finite() || previous_value.is_some_and(|v| !v.is_finite()) {
            return Err(DomainError::invalid_value(
                "progress history",
                "values must be finite",
            ));
        }
        if occurred_at > captured_at {
            return Err(DomainError::Invariant(
                "occurred_at cannot be after captured_at".into(),
            ));
        }
        Ok(Self {
            id,
            concept_id: track.concept_id.clone(),
            track_code: track.track_code.clone(),
            previous_value,
            current_value: track.current_value,
            level: track.level,
            occurred_at,
            captured_at,
            metadata_json: "{}".into(),
        })
    }
}

/// Immutable once-per-concept/calendar-date state observation (gaps are valid).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptStateSnapshot {
    pub id: Option<i64>,
    pub concept_id: EntityId,
    pub snapshot_date: DateValue,
    pub state_json: String,
    pub metadata_json: String,
    pub captured_at: Iso8601Timestamp,
}
impl ConceptStateSnapshot {
    pub fn new(
        concept_id: EntityId,
        snapshot_date: DateValue,
        state_json: impl Into<String>,
        captured_at: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        let state_json = state_json.into();
        if serde_json::from_str::<serde_json::Value>(&state_json).is_err() {
            return Err(DomainError::invalid_value(
                "concept snapshot state",
                "must be valid JSON",
            ));
        }
        Ok(Self {
            id: None,
            concept_id,
            snapshot_date,
            state_json,
            metadata_json: "{}".into(),
            captured_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn at(s: &str) -> Iso8601Timestamp {
        Iso8601Timestamp::parse(s).unwrap()
    }
    #[test]
    fn concepts_are_typed_active_and_timestamped() {
        let c = Concept::new(
            EntityId::new("c1").unwrap(),
            EntityId::new("p1").unwrap(),
            TypeRef::new("concept", "knowledge").unwrap(),
            "Python",
            at("2026-09-27T09:00:00Z"),
        )
        .unwrap();
        assert_eq!(c.name, "Python");
        assert!(c.is_active);
        assert_eq!(c.created_at, c.updated_at);
        assert_eq!(c.metadata_json, "{}");
        assert!(Concept::new(
            EntityId::new("c2").unwrap(),
            EntityId::new("p1").unwrap(),
            TypeRef::quest("main").unwrap(),
            "wrong namespace",
            at("2026-09-27T09:00:00Z")
        )
        .is_err());
    }
    #[test]
    fn relationships_reject_self_links_and_keep_database_defined_types() {
        let now = at("2026-09-27T09:00:00Z");
        assert!(ConceptRelationship::new(
            EntityId::new("r1").unwrap(),
            EntityId::new("p1").unwrap(),
            EntityId::new("c1").unwrap(),
            EntityId::new("c1").unwrap(),
            TypeRef::new("concept_relationship", "part_of").unwrap(),
            now.clone()
        )
        .is_err());
        let rel = ConceptRelationship::new(
            EntityId::new("r2").unwrap(),
            EntityId::new("p1").unwrap(),
            EntityId::new("c1").unwrap(),
            EntityId::new("c2").unwrap(),
            TypeRef::new("concept_relationship", "depends_on").unwrap(),
            now,
        )
        .unwrap();
        assert_eq!(rel.relationship_type.code, "depends_on");
    }
    #[test]
    fn progress_track_semantics_are_distinct_bounded_and_recorded() {
        let now = at("2026-09-27T09:00:00Z");
        let concept = EntityId::new("c1").unwrap();
        let mastery = ProgressTrackDefinition::new(
            EntityId::new("d1").unwrap(),
            "mastery",
            "Mastery",
            ProgressSemantics::Percentage,
            None,
            None,
            now.clone(),
        )
        .unwrap();
        assert!(mastery.validate_value(101.0).is_err());
        let xp = ProgressTrackDefinition::new(
            EntityId::new("d2").unwrap(),
            "xp",
            "XP",
            ProgressSemantics::Experience,
            Some(0.0),
            None,
            now.clone(),
        )
        .unwrap();
        assert!(xp.validate_value(-1.0).is_err());
        assert!(xp.validate_value(1.5).is_err());
        xp.validate_value(350.0).unwrap();
        let percentage = ProgressTrackDefinition::new(
            EntityId::new("d3").unwrap(),
            "percent",
            "Percent",
            ProgressSemantics::Percentage,
            None,
            None,
            now.clone(),
        )
        .unwrap();
        assert!(percentage.validate_value(101.0).is_err());
        let level = ProgressTrackDefinition::new(
            EntityId::new("d4").unwrap(),
            "level",
            "Level",
            ProgressSemantics::Level,
            None,
            None,
            now.clone(),
        )
        .unwrap();
        assert!(level.validate_value(2.5).is_err());
        assert!(level.validate_value(0.0).is_err());
        level.validate_value(3.0).unwrap();
        assert!(ConceptProgressEntry::new(
            EntityId::new("future").unwrap(),
            &ConceptProgressTrack::new(
                EntityId::new("t2").unwrap(),
                EntityId::new("c1").unwrap(),
                &mastery,
                60.0,
                None,
                now.clone()
            )
            .unwrap(),
            None,
            at("2026-09-27T11:00:00Z"),
            now.clone()
        )
        .is_err());
        let mut track = ConceptProgressTrack::new(
            EntityId::new("t1").unwrap(),
            concept,
            &mastery,
            72.0,
            None,
            now.clone(),
        )
        .unwrap();
        track.change(&mastery, 80.0, None, now.clone()).unwrap();
        let entry = ConceptProgressEntry::new(
            EntityId::new("h1").unwrap(),
            &track,
            Some(72.0),
            now.clone(),
            now,
        )
        .unwrap();
        assert_eq!(entry.current_value, 80.0);
        assert_eq!(entry.previous_value, Some(72.0));
    }
}
