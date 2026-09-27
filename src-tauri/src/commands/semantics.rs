//! Thin IPC adapters for Phase 3.6 world semantics; presentation remains out of scope.
use crate::state::AppState;
use lr_contracts::{semantics::*, CommandErrorDto};
use lr_domain::{
    AssociatedEntityKind, ContentTargetKind, LifecycleState, RevisionTargetKind, SessionStatus,
};
use tauri::State;

fn domain_error(e: lr_domain::DomainError) -> CommandErrorDto {
    lr_application::AppError::from(e).into()
}
fn revision_kind(s: &str) -> Result<RevisionTargetKind, CommandErrorDto> {
    RevisionTargetKind::parse(s).map_err(domain_error)
}
fn lifecycle_state(s: &str) -> Result<LifecycleState, CommandErrorDto> {
    LifecycleState::parse(s).map_err(domain_error)
}
fn content_kind(s: &str) -> Result<ContentTargetKind, CommandErrorDto> {
    ContentTargetKind::parse(s).map_err(domain_error)
}
fn association_kind(s: &str) -> Result<AssociatedEntityKind, CommandErrorDto> {
    AssociatedEntityKind::parse(s).map_err(domain_error)
}
fn session_status(s: &str) -> Result<SessionStatus, CommandErrorDto> {
    SessionStatus::parse(s).map_err(domain_error)
}
fn progress_control(s: &str) -> Result<lr_domain::ProgressControl, CommandErrorDto> {
    lr_domain::ProgressControl::parse(s).map_err(domain_error)
}

#[tauri::command]
pub fn search_world(
    state: State<'_, AppState>,
    query: SearchQueryDto,
) -> Result<Vec<SearchHitDto>, CommandErrorDto> {
    use lr_application::{SearchEntityKind, SearchQuery, SearchSort};
    use lr_domain::{EntityId, Iso8601Timestamp};
    let kind = query
        .kind
        .as_deref()
        .map(|v| {
            SearchEntityKind::parse(v).ok_or_else(|| {
                domain_error(lr_domain::DomainError::invalid_value(
                    "search kind",
                    "unsupported entity kind",
                ))
            })
        })
        .transpose()?;
    let sort = match query.sort.as_str() {
        "relevance" => SearchSort::Relevance,
        "newest" => SearchSort::Newest,
        "oldest" => SearchSort::Oldest,
        "name" => SearchSort::Name,
        "progression" => SearchSort::Progression,
        _ => {
            return Err(domain_error(lr_domain::DomainError::invalid_value(
                "search sort",
                "unsupported sort",
            )))
        }
    };
    let parse_id = |v: Option<String>| -> Result<Option<EntityId>, CommandErrorDto> {
        v.map(EntityId::new).transpose().map_err(domain_error)
    };
    let parse_time =
        |v: Option<String>| -> Result<Option<lr_domain::Iso8601Timestamp>, CommandErrorDto> {
            v.map(Iso8601Timestamp::parse)
                .transpose()
                .map_err(domain_error)
        };
    let query = SearchQuery {
        text: query.text,
        kind,
        player_id: parse_id(query.player_id)?,
        concept_id: parse_id(query.concept_id)?,
        type_code: query.type_code,
        status: query.status,
        active: query.active,
        from: parse_time(query.from)?,
        through: parse_time(query.through)?,
        context: query.context,
        include_hidden: query.include_hidden,
        include_archived: query.include_archived,
        include_trashed: query.include_trashed,
        sort,
        limit: query.limit,
        offset: query.offset,
    };
    query
        .validate()
        .map_err(|e| domain_error(lr_domain::DomainError::invalid_value("search query", e)))?;
    state
        .semantics
        .search(&query)
        .map(|hits| hits.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub fn set_concept_progress_control(
    state: State<'_, AppState>,
    concept_id: String,
    track_code: String,
    control: String,
) -> Result<ConceptProgressTrackDto, CommandErrorDto> {
    let control = progress_control(&control)?;
    state
        .semantics
        .set_progress_control(&concept_id, &track_code, control)
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub fn create_quest_stage(
    state: State<'_, AppState>,
    player_id: String,
    quest_id: String,
    title: String,
    sort_order: i32,
) -> Result<QuestStageDto, CommandErrorDto> {
    state
        .semantics
        .create_stage(&player_id, &quest_id, &title, sort_order)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_quest_stages(
    state: State<'_, AppState>,
    quest_id: String,
) -> Result<Vec<QuestStageDto>, CommandErrorDto> {
    state
        .semantics
        .list_stages(&quest_id)
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
#[tauri::command]
pub fn create_quest_branch(
    state: State<'_, AppState>,
    stage_id: String,
    title: String,
    sort_order: i32,
) -> Result<QuestBranchDto, CommandErrorDto> {
    state
        .semantics
        .create_branch(&stage_id, &title, sort_order)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_quest_branches(
    state: State<'_, AppState>,
    stage_id: String,
) -> Result<Vec<QuestBranchDto>, CommandErrorDto> {
    state
        .semantics
        .list_branches(&stage_id)
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
#[tauri::command]
pub fn start_quest_session(
    state: State<'_, AppState>,
    player_id: String,
    quest_id: Option<String>,
    stage_id: Option<String>,
    branch_id: Option<String>,
    skill_id: Option<String>,
    concept_id: Option<String>,
    started_at: Option<String>,
) -> Result<QuestSessionDto, CommandErrorDto> {
    state
        .semantics
        .start_session(
            &player_id,
            quest_id.as_deref(),
            stage_id.as_deref(),
            branch_id.as_deref(),
            skill_id.as_deref(),
            concept_id.as_deref(),
            started_at.as_deref(),
        )
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn finish_quest_session(
    state: State<'_, AppState>,
    session_id: String,
    ended_at: Option<String>,
    status: String,
    result: Option<String>,
    notes: Option<String>,
) -> Result<QuestSessionDto, CommandErrorDto> {
    let status = session_status(&status)?;
    state
        .semantics
        .finish_session(&session_id, ended_at.as_deref(), status, result, notes)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_quest_sessions(
    state: State<'_, AppState>,
    player_id: String,
    quest_id: Option<String>,
    stage_id: Option<String>,
) -> Result<Vec<QuestSessionDto>, CommandErrorDto> {
    state
        .semantics
        .list_sessions(&player_id, quest_id.as_deref(), stage_id.as_deref())
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
#[tauri::command]
pub fn attach_content(
    state: State<'_, AppState>,
    player_id: String,
    content_id: String,
    target_kind: String,
    target_id: String,
    role: String,
) -> Result<ContentAttachmentDto, CommandErrorDto> {
    let kind = content_kind(&target_kind)?;
    state
        .semantics
        .attach_content(&player_id, &content_id, kind, &target_id, &role)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_attached_content(
    state: State<'_, AppState>,
    target_kind: String,
    target_id: String,
) -> Result<Vec<ContentAttachmentDto>, CommandErrorDto> {
    let kind = content_kind(&target_kind)?;
    state
        .semantics
        .content_for(kind, &target_id)
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
#[tauri::command]
pub fn associate_concept(
    state: State<'_, AppState>,
    concept_id: String,
    entity_kind: String,
    entity_id: String,
    role: String,
) -> Result<ConceptAssociationDto, CommandErrorDto> {
    let kind = association_kind(&entity_kind)?;
    state
        .semantics
        .associate(&concept_id, kind, &entity_id, &role)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_concept_associations(
    state: State<'_, AppState>,
    concept_id: String,
    entity_kind: Option<String>,
    entity_id: Option<String>,
) -> Result<Vec<ConceptAssociationDto>, CommandErrorDto> {
    let kind = entity_kind.as_deref().map(association_kind).transpose()?;
    state
        .semantics
        .associations(&concept_id, kind, entity_id.as_deref())
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
#[tauri::command]
pub fn set_concept_association_active(
    state: State<'_, AppState>,
    association_id: String,
    active: bool,
) -> Result<ConceptAssociationDto, CommandErrorDto> {
    state
        .semantics
        .set_association_active(&association_id, active)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn set_entity_lifecycle(
    state: State<'_, AppState>,
    target_kind: String,
    target_id: String,
    player_id: String,
    lifecycle: String,
    reason: Option<String>,
) -> Result<(), CommandErrorDto> {
    let kind = revision_kind(&target_kind)?;
    let lifecycle = lifecycle_state(&lifecycle)?;
    state
        .semantics
        .set_lifecycle(kind, &target_id, &player_id, lifecycle, reason.as_deref())
        .map_err(Into::into)
}
#[tauri::command]
pub fn get_entity_lifecycle(
    state: State<'_, AppState>,
    target_kind: String,
    target_id: String,
) -> Result<LifecycleStateDto, CommandErrorDto> {
    let kind = revision_kind(&target_kind)?;
    state
        .semantics
        .lifecycle(kind, &target_id)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_entity_revisions(
    state: State<'_, AppState>,
    target_kind: String,
    target_id: String,
) -> Result<Vec<EntityRevisionDto>, CommandErrorDto> {
    let kind = revision_kind(&target_kind)?;
    state
        .semantics
        .revisions(kind, &target_id)
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
#[tauri::command]
pub fn restore_entity_revision(
    state: State<'_, AppState>,
    revision_id: String,
    reason: Option<String>,
) -> Result<EntityRevisionDto, CommandErrorDto> {
    state
        .semantics
        .restore_revision(&revision_id, reason.as_deref())
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn set_presentation_preference(
    state: State<'_, AppState>,
    player_id: String,
    entity_kind: String,
    entity_id: String,
    context: String,
    is_visible: bool,
    sort_order: i32,
    is_pinned: bool,
    is_collapsed: Option<bool>,
    variant: Option<String>,
    density: Option<String>,
) -> Result<PresentationPreferenceDto, CommandErrorDto> {
    state
        .semantics
        .set_presentation(
            &player_id,
            &entity_kind,
            &entity_id,
            &context,
            is_visible,
            sort_order,
            is_pinned,
            is_collapsed,
            variant,
            density,
        )
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_presentation_preferences(
    state: State<'_, AppState>,
    player_id: String,
    context: String,
) -> Result<Vec<PresentationPreferenceDto>, CommandErrorDto> {
    state
        .semantics
        .presentation(&player_id, &context)
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
#[tauri::command]
pub fn suggest_concept_progress(
    state: State<'_, AppState>,
    player_id: String,
    concept_id: String,
    track_code: String,
    value: f64,
    level: Option<i32>,
    reason: Option<String>,
    source: String,
) -> Result<ProgressSuggestionDto, CommandErrorDto> {
    state
        .semantics
        .suggest_progress(
            &player_id,
            &concept_id,
            &track_code,
            value,
            level,
            reason,
            &source,
        )
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_progress_suggestions(
    state: State<'_, AppState>,
    concept_id: String,
    include_resolved: bool,
) -> Result<Vec<ProgressSuggestionDto>, CommandErrorDto> {
    state
        .semantics
        .suggestions(&concept_id, include_resolved)
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
#[tauri::command]
pub fn accept_progress_suggestion(
    state: State<'_, AppState>,
    player_id: String,
    suggestion_id: String,
) -> Result<ProgressSuggestionDto, CommandErrorDto> {
    state
        .semantics
        .accept_suggestion(&player_id, &suggestion_id)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn reject_progress_suggestion(
    state: State<'_, AppState>,
    player_id: String,
    suggestion_id: String,
) -> Result<ProgressSuggestionDto, CommandErrorDto> {
    state
        .semantics
        .reject_suggestion(&player_id, &suggestion_id)
        .map(Into::into)
        .map_err(Into::into)
}
