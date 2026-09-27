//! # `lr-application` — use cases and storage ports.
//!
//! This layer depends only on `lr-domain`; it contains no SQL, SQLite, Tauri, or
//! React concepts. Persistence adapters implement the traits in [`ports`].

pub mod error;
pub mod ports;
pub mod rules;
pub mod search;
pub mod services;

pub use error::{AppError, StorageError};
pub use ports::{
    Clock, ConceptStore, HealthStore, MigrationRecord, RoundTripProof, SchemaReport, SearchStore,
    StoreDiagnostics, WorldStore,
};
pub use rules::{
    Comparison, EventKind, NumericSubject, Rule, RuleAction, RuleCondition, RuleDefinition,
    RuleEvent, RuleExecutionError, RuleExecutionRecord, RuleOperation, TextComparison, TextSubject,
    MAX_ACTIONS_PER_CHAIN, MAX_ACTIONS_PER_RULE, MAX_CONDITION_DEPTH, MAX_RULE_CHAIN_DEPTH,
    MAX_RULE_EVALUATIONS_PER_CHAIN, RULE_SCHEMA_VERSION,
};
pub use search::{SearchEntityKind, SearchHit, SearchQuery, SearchSort};
pub use services::concepts::{ConceptDetail, ConceptService};
pub use services::health::{
    ApplicationInfo, DatabaseInfo, HealthReport, HealthService, HealthStatus, RoundTripInfo,
};
pub use services::world::{AwardXpOutcome, WorldOverview, WorldService, DEFAULT_LEDGER_LIMIT};

pub const LAYER_NAME: &str = "application";
