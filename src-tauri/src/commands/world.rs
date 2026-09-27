//! Thin IPC adapters for the Phase 2 application world service.

use crate::state::AppState;
use lr_application::DEFAULT_LEDGER_LIMIT;
use lr_contracts::world::{
    AwardXpOutcomeDto, CommentDto, NarrativeEntryDto, PlayerDto, PlayerSnapshotDto, QuestDto,
    SkillDto, SkillTreeDto, TransactionDto, WorldOverviewDto,
};
use lr_contracts::CommandErrorDto;
use tauri::State;

#[tauri::command]
pub fn create_player(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
) -> Result<PlayerDto, CommandErrorDto> {
    state
        .world
        .create_player(&name, description)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn get_player(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<PlayerDto>, CommandErrorDto> {
    state
        .world
        .get_player(&id)
        .map(|v| v.map(Into::into))
        .map_err(Into::into)
}
#[tauri::command]
pub fn award_xp(
    state: State<'_, AppState>,
    player_id: String,
    amount: i64,
    reason: Option<String>,
    description: Option<String>,
) -> Result<AwardXpOutcomeDto, CommandErrorDto> {
    state
        .world
        .award_xp(&player_id, amount, reason, description)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn create_quest(
    state: State<'_, AppState>,
    player_id: String,
    type_code: String,
    title: String,
    parent_quest_id: Option<String>,
    skill_id: Option<String>,
    difficulty: Option<i32>,
    xp_reward: Option<i64>,
) -> Result<QuestDto, CommandErrorDto> {
    state
        .world
        .create_quest(
            &player_id,
            &type_code,
            &title,
            parent_quest_id,
            skill_id,
            difficulty,
            xp_reward,
        )
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn start_quest(
    state: State<'_, AppState>,
    quest_id: String,
) -> Result<QuestDto, CommandErrorDto> {
    state
        .world
        .start_quest(&quest_id)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn complete_quest(
    state: State<'_, AppState>,
    quest_id: String,
) -> Result<QuestDto, CommandErrorDto> {
    state
        .world
        .complete_quest(&quest_id)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn create_skill_tree(
    state: State<'_, AppState>,
    player_id: String,
    type_code: String,
    name: String,
) -> Result<SkillTreeDto, CommandErrorDto> {
    state
        .world
        .create_skill_tree(&player_id, &type_code, &name)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn add_skill(
    state: State<'_, AppState>,
    tree_id: String,
    type_code: String,
    name: String,
    parent_skill_id: Option<String>,
) -> Result<SkillDto, CommandErrorDto> {
    state
        .world
        .add_skill(&tree_id, &type_code, &name, parent_skill_id)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn invest_skill_time(
    state: State<'_, AppState>,
    skill_id: String,
    minutes: i64,
) -> Result<SkillDto, CommandErrorDto> {
    state
        .world
        .invest_skill_time(&skill_id, minutes)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn capture_player_snapshot(
    state: State<'_, AppState>,
    player_id: String,
) -> Result<PlayerSnapshotDto, CommandErrorDto> {
    state
        .world
        .capture_player_snapshot(&player_id)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn add_comment(
    state: State<'_, AppState>,
    author_player_id: Option<String>,
    target_kind: String,
    target_id: String,
    body: String,
) -> Result<CommentDto, CommandErrorDto> {
    state
        .world
        .add_comment(author_player_id, &target_kind, &target_id, &body)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn write_narrative(
    state: State<'_, AppState>,
    player_id: String,
    kind: String,
    title: String,
    content: String,
) -> Result<NarrativeEntryDto, CommandErrorDto> {
    state
        .world
        .write_narrative(&player_id, &kind, &title, &content)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn get_world_overview(
    state: State<'_, AppState>,
    player_id: String,
) -> Result<WorldOverviewDto, CommandErrorDto> {
    state
        .world
        .world_overview(&player_id)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_transactions(
    state: State<'_, AppState>,
    player_id: String,
    limit: Option<u32>,
) -> Result<Vec<TransactionDto>, CommandErrorDto> {
    let overview = state
        .world
        .world_overview(&player_id)
        .map_err(CommandErrorDto::from)?;
    let limit = limit.unwrap_or(DEFAULT_LEDGER_LIMIT) as usize;
    Ok(overview
        .recent_transactions
        .into_iter()
        .take(limit)
        .map(Into::into)
        .collect())
}
