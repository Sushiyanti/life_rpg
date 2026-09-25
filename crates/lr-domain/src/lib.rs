//! # `lr-domain` — the DOMAIN layer
//!
//! This crate answers the question **"what exists in the world?"**.
//!
//! Hard rules for this crate (see `docs/ARCHITECTURE.md`):
//!
//! * It must **not** depend on the UI, on `lr-application`, on `lr-persistence`,
//!   on SQLite, or on Tauri. There is deliberately no `tauri` and no `rusqlite`
//!   in its dependency list — `cargo test -p lr-domain` compiles with zero
//!   infrastructure.
//! * It must **not** contain I/O. Domain code is pure: in → out.
//! * Phase 1 intentionally ships only identity/value objects and the domain
//!   error vocabulary. The Player/Quest/Skill/Transaction/Effect/narrative
//!   concepts are *deliberately not implemented yet*; Phase 1 is a foundation,
//!   not a feature drop. What we do ship here is the vocabulary that those
//!   later concepts will be built from, so that Phase 2 adds aggregates
//!   instead of rewriting primitives.
//!
//! Current state of the world model:
//!
//! | Concept          | Phase 1 status                          |
//! |------------------|-----------------------------------------|
//! | Identity         | [`EntityId`] — opaque, validated         |
//! | Schema version   | [`SchemaVersion`]                        |
//! | Timestamps       | [`Iso8601Timestamp`] — validated         |
//! | Aggregates       | not yet (Phase 2)                        |

pub mod error;
pub mod value;

pub use error::{DomainError, DomainResult};
pub use value::{EntityId, Iso8601Timestamp, SchemaVersion};

/// Human-readable name of this layer, used by status reporting.
pub const LAYER_NAME: &str = "domain";
