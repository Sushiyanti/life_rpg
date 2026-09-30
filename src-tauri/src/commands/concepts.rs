//! Minimal Tauri adapters for existing Concept application use cases.
use crate::state::AppState;
use lr_contracts::CommandErrorDto;
use lr_contracts::{
    semantics::ConceptProgressTrackDto,
    world::{ConceptDto, ConceptRelationshipDto},
};
use tauri::State;

#[tauri::command]
pub fn create_concept(
    state: State<'_, AppState>,
    player_id: String,
    type_code: String,
    name: String,
    description: Option<String>,
) -> Result<ConceptDto, CommandErrorDto> {
    state
        .concepts
        .create_concept(&player_id, &type_code, &name, description)
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_concepts(
    state: State<'_, AppState>,
    player_id: String,
) -> Result<Vec<ConceptDto>, CommandErrorDto> {
    state
        .concepts
        .list_concepts(&player_id)
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub fn get_concept(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<ConceptDto>, CommandErrorDto> {
    state
        .concepts
        .get_concept(&id)
        .map(|item| item.map(Into::into))
        .map_err(Into::into)
}

#[tauri::command]
pub fn set_concept_active(
    state: State<'_, AppState>,
    concept_id: String,
    active: bool,
) -> Result<ConceptDto, CommandErrorDto> {
    state
        .concepts
        .set_concept_active(&concept_id, active)
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_concept_progress(
    state: State<'_, AppState>,
    concept_id: String,
) -> Result<Vec<ConceptProgressTrackDto>, CommandErrorDto> {
    state
        .concepts
        .progress_tracks(&concept_id)
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub fn set_concept_progress(
    state: State<'_, AppState>,
    concept_id: String,
    track_code: String,
    value: f64,
    level: Option<i32>,
    occurred_at: Option<String>,
) -> Result<ConceptProgressTrackDto, CommandErrorDto> {
    state
        .concepts
        .set_progress(
            &concept_id,
            &track_code,
            value,
            level,
            occurred_at.as_deref(),
        )
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_concept_relationships(
    state: State<'_, AppState>,
    concept_id: String,
) -> Result<Vec<ConceptRelationshipDto>, CommandErrorDto> {
    state
        .concepts
        .relationship_list(&concept_id)
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_concept_relationship_types(
    state: State<'_, AppState>,
) -> Result<Vec<String>, CommandErrorDto> {
    state.concepts.relationship_types().map_err(Into::into)
}

#[tauri::command]
pub fn relate_concepts(
    state: State<'_, AppState>,
    source_concept_id: String,
    target_concept_id: String,
    relationship_code: String,
) -> Result<ConceptRelationshipDto, CommandErrorDto> {
    state
        .concepts
        .relate(&source_concept_id, &target_concept_id, &relationship_code)
        .map(Into::into)
        .map_err(Into::into)
}
