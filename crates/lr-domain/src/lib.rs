//! # `lr-domain` — pure world entities and their invariants.
//!
//! This crate deliberately knows nothing about SQLite, Tauri, React, or I/O.
//! Phase 2 separates cached state (aggregates), immutable history (transactions
//! and snapshots), and user/game content (comments and narrative entries).

pub mod association;
pub mod comment;
pub mod concept;
pub mod effect;
pub mod effect_history;
pub mod error;
pub mod narrative;
pub mod player;
pub mod presentation;
pub mod progress_suggestion;
pub mod quest;
pub mod quest_activity;
pub mod recovery;
pub mod skill;
pub mod skill_history;
pub mod snapshot;
pub mod stat;
pub mod tag;
pub mod transaction;
pub mod type_definition;
pub mod value;
pub mod workspace;

pub use association::{AssociatedEntityKind, ConceptAssociation};
pub use comment::{Comment, CommentTargetKind};
pub use concept::{
    Concept, ConceptEntityKind, ConceptEntityLink, ConceptProgressEntry, ConceptProgressTrack,
    ConceptRelationship, ConceptStateSnapshot, ProgressControl, ProgressSemantics,
    ProgressTrackDefinition,
};
pub use effect::{Effect, EffectDeactivationSource, EffectLifecycle, EffectTargetKind};
pub use effect_history::{EffectHistoryEntry, EffectHistoryKind, SessionEffect, SessionEffectRole};
pub use error::{DomainError, DomainResult};
pub use narrative::NarrativeEntry;
pub use player::Player;
pub use presentation::PresentationPreference;
pub use progress_suggestion::{ProgressSuggestion, SuggestionStatus};
pub use quest::{Quest, QuestStatus};
pub use quest_activity::{
    BranchStatus, ContentAttachment, ContentTargetKind, QuestBranch, QuestSession, QuestStage,
    SessionStatus, StageStatus,
};
pub use recovery::{EntityRevision, LifecycleState, RevisionTargetKind};
pub use skill::{Skill, SkillAvailability, SkillAvailabilityControl, SkillStatus, SkillTree};
pub use skill_history::{SkillHistoryEntry, SkillHistoryKind, SkillHistorySource};
pub use snapshot::{PlayerStateSnapshot, SkillStateSnapshot};
pub use stat::{PlayerStat, StatDefinition};
pub use tag::{
    normalize_tag_name, validate_tag_filter, Tag, TagMatchMode, TagRelationship, TagTargetKind,
    TagTargetReference, TaggedRecord,
};
pub use transaction::Transaction;
pub use type_definition::{TypeDefinition, TypeRef};
pub use value::{DateValue, EntityId, Iso8601Timestamp, SchemaVersion};
pub use workspace::{Workspace, WorkspacePanel};

/// Human-readable name of this layer, used by status reporting.
pub const LAYER_NAME: &str = "domain";
