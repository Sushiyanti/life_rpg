//! Thin typed IPC adapters for explicit Player-owned Tag operations.
use crate::state::AppState;
use lr_contracts::{
    semantics::{TagDto, TagRelationshipDto, TagTargetReferenceDto, TaggedRecordDto},
    CommandErrorDto,
};
use lr_domain::{LifecycleState, TagTargetKind};
use tauri::State;

fn parse_target_kind(value: &str) -> Result<TagTargetKind, CommandErrorDto> {
    TagTargetKind::parse(value).map_err(|error| lr_application::AppError::from(error).into())
}
fn parse_lifecycle(value: &str) -> Result<LifecycleState, CommandErrorDto> {
    LifecycleState::parse(value).map_err(|error| lr_application::AppError::from(error).into())
}

#[tauri::command]
pub fn list_tags(
    state: State<'_, AppState>,
    player_id: String,
    query: Option<String>,
    include_archived: bool,
    include_trashed: bool,
) -> Result<Vec<TagDto>, CommandErrorDto> {
    state
        .tags
        .list_tags(
            &player_id,
            query.as_deref(),
            include_archived,
            include_trashed,
        )
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
#[tauri::command]
pub fn create_tag(
    state: State<'_, AppState>,
    player_id: String,
    name: String,
    description: Option<String>,
) -> Result<TagDto, CommandErrorDto> {
    state
        .tags
        .create_tag(&player_id, &name, description)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn rename_tag(
    state: State<'_, AppState>,
    player_id: String,
    tag_id: String,
    name: String,
    description: Option<String>,
) -> Result<TagDto, CommandErrorDto> {
    state
        .tags
        .rename_tag(&player_id, &tag_id, &name, description)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn set_tag_lifecycle(
    state: State<'_, AppState>,
    player_id: String,
    tag_id: String,
    lifecycle: String,
    reason: Option<String>,
) -> Result<TagDto, CommandErrorDto> {
    state
        .tags
        .set_lifecycle(
            &player_id,
            &tag_id,
            parse_lifecycle(&lifecycle)?,
            reason.as_deref(),
        )
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn attach_tag(
    state: State<'_, AppState>,
    player_id: String,
    tag_id: String,
    target_kind: String,
    target_id: String,
) -> Result<TagRelationshipDto, CommandErrorDto> {
    state
        .tags
        .attach_tag(
            &player_id,
            &tag_id,
            parse_target_kind(&target_kind)?,
            &target_id,
        )
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn detach_tag(
    state: State<'_, AppState>,
    player_id: String,
    relationship_id: String,
) -> Result<TagRelationshipDto, CommandErrorDto> {
    state
        .tags
        .detach_tag(&player_id, &relationship_id)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_tags_for_target(
    state: State<'_, AppState>,
    player_id: String,
    target_kind: String,
    target_id: String,
) -> Result<Vec<TaggedRecordDto>, CommandErrorDto> {
    state
        .tags
        .tags_for_target(&player_id, parse_target_kind(&target_kind)?, &target_id)
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_tag_targets(
    state: State<'_, AppState>,
    player_id: String,
    tag_id: String,
    include_removed: bool,
) -> Result<Vec<TagTargetReferenceDto>, CommandErrorDto> {
    state
        .tags
        .targets_for_tag(&player_id, &tag_id, include_removed)
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::{parse_lifecycle, parse_target_kind};

    #[test]
    fn ipc_rejects_unknown_or_immutable_tag_target_kinds() {
        assert!(parse_target_kind("quest").is_ok());
        assert!(parse_target_kind("comment").is_ok());
        assert!(parse_target_kind("transaction").is_err());
        assert!(parse_target_kind("revision").is_err());
        assert!(parse_target_kind("workspace").is_err());
    }

    #[test]
    fn ipc_rejects_unknown_tag_lifecycle_states() {
        assert!(parse_lifecycle("active").is_ok());
        assert!(parse_lifecycle("archived").is_ok());
        assert!(parse_lifecycle("permanently_deleted").is_err());
    }
}
