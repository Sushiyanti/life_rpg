//! # `lr-domain` — pure world entities and their invariants.
//!
//! This crate deliberately knows nothing about SQLite, Tauri, React, or I/O.
//! Phase 2 separates cached state (aggregates), immutable history (transactions
//! and snapshots), and user/game content (comments and narrative entries).

pub mod comment;
pub mod effect;
pub mod error;
pub mod narrative;
pub mod player;
pub mod quest;
pub mod skill;
pub mod snapshot;
pub mod transaction;
pub mod type_definition;
pub mod value;

pub use comment::{Comment, CommentTargetKind};
pub use effect::Effect;
pub use error::{DomainError, DomainResult};
pub use narrative::NarrativeEntry;
pub use player::{Player, XP_PER_LEVEL};
pub use quest::{Quest, QuestStatus};
pub use skill::{Skill, SkillStatus, SkillTree};
pub use snapshot::{PlayerStateSnapshot, SkillStateSnapshot};
pub use transaction::Transaction;
pub use type_definition::{TypeDefinition, TypeRef};
pub use value::{DateValue, EntityId, Iso8601Timestamp, SchemaVersion};

/// Human-readable name of this layer, used by status reporting.
pub const LAYER_NAME: &str = "domain";
