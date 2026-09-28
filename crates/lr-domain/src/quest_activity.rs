//! Structured Quest activity. Simple Quests do not require any of these child records.
use crate::{DomainError, DomainResult, EntityId, Iso8601Timestamp};

fn text(field: &'static str, value: impl Into<String>) -> DomainResult<String> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageStatus {
    Pending,
    Active,
    Completed,
    Skipped,
}
impl StageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Skipped => "skipped",
        }
    }
    pub fn parse(v: &str) -> DomainResult<Self> {
        match v {
            "pending" => Ok(Self::Pending),
            "active" => Ok(Self::Active),
            "completed" => Ok(Self::Completed),
            "skipped" => Ok(Self::Skipped),
            _ => Err(DomainError::invalid_value("stage status", "unknown status")),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestStage {
    pub id: EntityId,
    pub player_id: EntityId,
    pub quest_id: EntityId,
    pub title: String,
    pub description: Option<String>,
    pub story: Option<String>,
    pub instructions: Option<String>,
    pub status: StageStatus,
    pub sort_order: i32,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl QuestStage {
    pub fn new(
        id: EntityId,
        player_id: EntityId,
        quest_id: EntityId,
        title: impl Into<String>,
        sort_order: i32,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        Ok(Self {
            id,
            player_id,
            quest_id,
            title: text("stage title", title)?,
            description: None,
            story: None,
            instructions: None,
            status: StageStatus::Pending,
            sort_order,
            is_active: true,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchStatus {
    Available,
    Chosen,
    Rejected,
    Completed,
}
impl BranchStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Chosen => "chosen",
            Self::Rejected => "rejected",
            Self::Completed => "completed",
        }
    }
    pub fn parse(v: &str) -> DomainResult<Self> {
        match v {
            "available" => Ok(Self::Available),
            "chosen" => Ok(Self::Chosen),
            "rejected" => Ok(Self::Rejected),
            "completed" => Ok(Self::Completed),
            _ => Err(DomainError::invalid_value(
                "branch status",
                "unknown status",
            )),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestBranch {
    pub id: EntityId,
    pub player_id: EntityId,
    pub quest_id: EntityId,
    pub stage_id: EntityId,
    pub title: String,
    pub description: Option<String>,
    pub status: BranchStatus,
    pub sort_order: i32,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl QuestBranch {
    pub fn new(
        id: EntityId,
        player_id: EntityId,
        quest_id: EntityId,
        stage_id: EntityId,
        title: impl Into<String>,
        sort_order: i32,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        Ok(Self {
            id,
            player_id,
            quest_id,
            stage_id,
            title: text("branch title", title)?,
            description: None,
            status: BranchStatus::Available,
            sort_order,
            is_active: true,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    InProgress,
    Completed,
    Interrupted,
}
impl SessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Interrupted => "interrupted",
        }
    }
    pub fn parse(v: &str) -> DomainResult<Self> {
        match v {
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "interrupted" => Ok(Self::Interrupted),
            _ => Err(DomainError::invalid_value(
                "session status",
                "unknown status",
            )),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestSession {
    pub id: EntityId,
    pub player_id: EntityId,
    pub quest_id: Option<EntityId>,
    pub stage_id: Option<EntityId>,
    pub branch_id: Option<EntityId>,
    pub skill_id: Option<EntityId>,
    pub concept_id: Option<EntityId>,
    pub started_at: Iso8601Timestamp,
    pub ended_at: Option<Iso8601Timestamp>,
    pub status: SessionStatus,
    pub progress_before: Option<i32>,
    pub progress_after: Option<i32>,
    pub result: Option<String>,
    pub notes: Option<String>,
    pub is_active: bool,
    pub metadata_json: String,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
}
impl QuestSession {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: EntityId,
        player_id: EntityId,
        quest_id: Option<EntityId>,
        stage_id: Option<EntityId>,
        branch_id: Option<EntityId>,
        skill_id: Option<EntityId>,
        concept_id: Option<EntityId>,
        started_at: Iso8601Timestamp,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        if quest_id.is_none()
            && stage_id.is_none()
            && branch_id.is_none()
            && skill_id.is_none()
            && concept_id.is_none()
        {
            return Err(DomainError::invalid_value(
                "session context",
                "at least one Quest, Stage, Branch, Skill, or Concept is required",
            ));
        }
        if branch_id.is_some() && stage_id.is_none() {
            return Err(DomainError::invalid_value(
                "session branch",
                "a branch requires its parent Stage",
            ));
        }
        Ok(Self {
            id,
            player_id,
            quest_id,
            stage_id,
            branch_id,
            skill_id,
            concept_id,
            started_at,
            ended_at: None,
            status: SessionStatus::InProgress,
            progress_before: None,
            progress_after: None,
            result: None,
            notes: None,
            is_active: true,
            metadata_json: "{}".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
    pub fn finish(
        &mut self,
        ended_at: Iso8601Timestamp,
        status: SessionStatus,
        now: Iso8601Timestamp,
    ) -> DomainResult<()> {
        if self.status != SessionStatus::InProgress || self.ended_at.is_some() {
            return Err(DomainError::Invariant("Session is already finished".into()));
        }
        if status == SessionStatus::InProgress {
            return Err(DomainError::invalid_value(
                "session status",
                "finish requires completed or interrupted",
            ));
        }
        if ended_at < self.started_at {
            return Err(DomainError::Invariant(
                "session end cannot precede its start".into(),
            ));
        }
        self.ended_at = Some(ended_at);
        self.status = status;
        self.updated_at = now;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentTargetKind {
    Player,
    Quest,
    Stage,
    Branch,
    Session,
    Skill,
    SkillTree,
    Concept,
    Effect,
}
impl ContentTargetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Quest => "quest",
            Self::Stage => "quest_stage",
            Self::Branch => "quest_branch",
            Self::Session => "quest_session",
            Self::Skill => "skill",
            Self::SkillTree => "skill_tree",
            Self::Concept => "concept",
            Self::Effect => "effect",
        }
    }
    pub fn parse(v: &str) -> DomainResult<Self> {
        match v {
            "player" => Ok(Self::Player),
            "quest" => Ok(Self::Quest),
            "quest_stage" => Ok(Self::Stage),
            "quest_branch" => Ok(Self::Branch),
            "quest_session" => Ok(Self::Session),
            "skill" => Ok(Self::Skill),
            "skill_tree" => Ok(Self::SkillTree),
            "concept" => Ok(Self::Concept),
            "effect" => Ok(Self::Effect),
            _ => Err(DomainError::invalid_value(
                "content target",
                "unsupported target kind",
            )),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentAttachment {
    /// Stable identifier makes each attach/remove cycle a historical fact.
    pub id: EntityId,
    pub content_id: EntityId,
    pub player_id: EntityId,
    pub target_kind: ContentTargetKind,
    pub target_id: EntityId,
    pub role_code: String,
    pub sort_order: i32,
    /// Compatibility projection for existing callers; it is `removed_at.is_none()`.
    pub is_active: bool,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
    pub removed_at: Option<Iso8601Timestamp>,
}
impl ContentAttachment {
    pub fn new(
        id: EntityId,
        content_id: EntityId,
        player_id: EntityId,
        target_kind: ContentTargetKind,
        target_id: EntityId,
        role_code: impl Into<String>,
        sort_order: i32,
        now: Iso8601Timestamp,
    ) -> DomainResult<Self> {
        let code = role_code.into();
        if code.is_empty()
            || code.len() > 160
            || !code
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        {
            return Err(DomainError::invalid_value(
                "content role",
                "use 1-160 lowercase ASCII letters, digits, or underscores",
            ));
        }
        Ok(Self {
            id,
            content_id,
            player_id,
            target_kind,
            target_id,
            role_code: code,
            sort_order,
            is_active: true,
            created_at: now.clone(),
            updated_at: now,
            removed_at: None,
        })
    }

    pub fn remove(&mut self, removed_at: Iso8601Timestamp) -> DomainResult<()> {
        if self.removed_at.is_some() || !self.is_active {
            return Err(DomainError::invalid_value(
                "content relationship",
                "is already removed",
            ));
        }
        self.removed_at = Some(removed_at.clone());
        self.updated_at = removed_at;
        self.is_active = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn at(s: &str) -> Iso8601Timestamp {
        Iso8601Timestamp::parse(s).unwrap()
    }
    #[test]
    fn sessions_are_real_timed_records_and_validate_context_and_chronology() {
        let p = EntityId::new("p").unwrap();
        let quest = EntityId::new("q").unwrap();
        let start = at("2026-09-27T03:10:00Z");
        assert!(QuestSession::new(
            EntityId::new("bad").unwrap(),
            p.clone(),
            None,
            None,
            None,
            None,
            None,
            start.clone(),
            start.clone()
        )
        .is_err());
        assert!(QuestSession::new(
            EntityId::new("bad2").unwrap(),
            p.clone(),
            Some(quest.clone()),
            None,
            Some(EntityId::new("b").unwrap()),
            None,
            None,
            start.clone(),
            start.clone()
        )
        .is_err());
        let mut a = QuestSession::new(
            EntityId::new("s1").unwrap(),
            p.clone(),
            Some(quest.clone()),
            None,
            None,
            None,
            None,
            start.clone(),
            start.clone(),
        )
        .unwrap();
        let mut b = QuestSession::new(
            EntityId::new("s2").unwrap(),
            p,
            Some(quest),
            None,
            None,
            None,
            None,
            at("2026-09-27T14:20:00Z"),
            at("2026-09-27T14:20:00Z"),
        )
        .unwrap();
        assert!(a
            .finish(
                at("2026-09-27T03:09:00Z"),
                SessionStatus::Completed,
                start.clone()
            )
            .is_err());
        a.finish(
            at("2026-09-27T04:32:00Z"),
            SessionStatus::Completed,
            at("2026-09-27T04:32:00Z"),
        )
        .unwrap();
        b.finish(
            at("2026-09-27T15:05:00Z"),
            SessionStatus::Completed,
            at("2026-09-27T15:05:00Z"),
        )
        .unwrap();
        assert_ne!(a.started_at, b.started_at);
    }
}
