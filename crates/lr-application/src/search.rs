//! Typed global search/query API. Results remain domain entity references, not generic database rows.
use lr_domain::{
    validate_tag_filter, ContentTargetKind, EntityId, Iso8601Timestamp, LifecycleState,
    TagMatchMode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchEntityKind {
    Player,
    Concept,
    Quest,
    QuestStage,
    QuestBranch,
    QuestSession,
    SkillTree,
    Skill,
    Effect,
    Transaction,
    Comment,
    NarrativeEntry,
    ConceptProgress,
}
impl SearchEntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Concept => "concept",
            Self::Quest => "quest",
            Self::QuestStage => "quest_stage",
            Self::QuestBranch => "quest_branch",
            Self::QuestSession => "quest_session",
            Self::SkillTree => "skill_tree",
            Self::Skill => "skill",
            Self::Effect => "effect",
            Self::Transaction => "transaction",
            Self::Comment => "comment",
            Self::NarrativeEntry => "narrative_entry",
            Self::ConceptProgress => "concept_progress",
        }
    }
    pub fn parse(v: &str) -> Option<Self> {
        Some(match v {
            "player" => Self::Player,
            "concept" => Self::Concept,
            "quest" => Self::Quest,
            "quest_stage" => Self::QuestStage,
            "quest_branch" => Self::QuestBranch,
            "quest_session" => Self::QuestSession,
            "skill_tree" => Self::SkillTree,
            "skill" => Self::Skill,
            "effect" => Self::Effect,
            "transaction" => Self::Transaction,
            "comment" => Self::Comment,
            "narrative_entry" => Self::NarrativeEntry,
            "concept_progress" => Self::ConceptProgress,
            _ => return None,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchSort {
    Relevance,
    Newest,
    Oldest,
    Name,
    Progression,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    pub text: Option<String>,
    pub kind: Option<SearchEntityKind>,
    pub player_id: Option<EntityId>,
    pub concept_id: Option<EntityId>,
    /// Current explicit Tag assignments; evaluated relationally, never projected into FTS.
    pub tag_ids: Vec<EntityId>,
    pub tag_match: TagMatchMode,
    pub type_code: Option<String>,
    pub status: Option<String>,
    pub active: Option<bool>,
    /// When supplied, only Narrative Entries actively attached to this target kind match.
    pub target_kind: Option<ContentTargetKind>,
    pub from: Option<Iso8601Timestamp>,
    pub through: Option<Iso8601Timestamp>,
    /// Apply contextual visibility only when an actual UI context is requested.
    pub context: Option<String>,
    pub include_hidden: bool,
    pub include_archived: bool,
    pub include_trashed: bool,
    pub sort: SearchSort,
    pub limit: u32,
    pub offset: u32,
}
impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            text: None,
            kind: None,
            player_id: None,
            concept_id: None,
            tag_ids: vec![],
            tag_match: TagMatchMode::Any,
            type_code: None,
            status: None,
            active: None,
            target_kind: None,
            from: None,
            through: None,
            context: None,
            include_hidden: false,
            include_archived: false,
            include_trashed: false,
            sort: SearchSort::Newest,
            limit: 50,
            offset: 0,
        }
    }
}
impl SearchQuery {
    pub fn validate(&self) -> Result<(), String> {
        if self.text.as_ref().is_some_and(|v| v.chars().count() > 256) {
            return Err("search text must be at most 256 characters".into());
        }
        if self.text.as_ref().is_some_and(|v| !v.trim().is_empty()) && self.fts_query().is_none() {
            return Err("search text must contain at least one searchable term".into());
        }
        if self
            .type_code
            .as_ref()
            .is_some_and(|v| v.trim().is_empty() || v.len() > 160)
        {
            return Err("type code must contain 1 to 160 bytes".into());
        }
        if self
            .status
            .as_ref()
            .is_some_and(|v| v.trim().is_empty() || v.len() > 160)
        {
            return Err("status must contain 1 to 160 bytes".into());
        }
        if self
            .from
            .as_ref()
            .zip(self.through.as_ref())
            .is_some_and(|(a, b)| a > b)
        {
            return Err("search time range is reversed".into());
        }
        if self.limit == 0 || self.limit > 200 {
            return Err("search limit must be between 1 and 200".into());
        }
        if self.offset > 1_000_000 {
            return Err("search offset is too large".into());
        }
        if self.context.as_ref().is_some_and(|v| {
            v.is_empty()
                || v.len() > 160
                || !v
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        }) {
            return Err("search context must be a 1-160 character lowercase identifier".into());
        }
        validate_tag_filter(&self.tag_ids, self.tag_match).map_err(|error| error.to_string())?;
        if !self.tag_ids.is_empty() {
            if self.player_id.is_none() {
                return Err("Tag filtering requires a Player-scoped query".into());
            }
            if self.kind.is_some_and(|kind| {
                matches!(
                    kind,
                    SearchEntityKind::Player
                        | SearchEntityKind::Transaction
                        | SearchEntityKind::ConceptProgress
                )
            }) {
                return Err("Tag filtering is unsupported for this search record kind".into());
            }
        }
        Ok(())
    }
    /// Convert input into a quoted AND query; callers cannot inject FTS operators.
    pub fn fts_query(&self) -> Option<String> {
        let text = self.text.as_ref()?;
        let terms: Vec<String> = text
            .split_whitespace()
            .map(|s| {
                s.chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_')
                    .collect::<String>()
            })
            .filter(|s| !s.is_empty())
            .take(32)
            .map(|s| format!("\"{}\"", s.to_lowercase().replace('"', "")))
            .collect();
        (!terms.is_empty()).then(|| terms.join(" AND "))
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub kind: SearchEntityKind,
    /// String IDs intentionally support both opaque IDs and append-only integer history IDs.
    pub id: String,
    pub player_id: Option<EntityId>,
    pub concept_id: Option<EntityId>,
    pub type_code: Option<String>,
    pub status: Option<String>,
    pub active: Option<bool>,
    pub lifecycle: LifecycleState,
    pub visible: Option<bool>,
    pub occurred_at: Option<Iso8601Timestamp>,
    pub captured_at: Option<Iso8601Timestamp>,
    pub name: String,
    pub snippet: String,
    pub progression: Option<f64>,
    pub relevance: Option<f64>,
}
