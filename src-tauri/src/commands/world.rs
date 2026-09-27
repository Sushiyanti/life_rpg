//! Thin IPC adapters for the Phase 2 application world service.

use crate::state::AppState;
use lr_application::{NarrativeWrite, RuleDefinition, DEFAULT_LEDGER_LIMIT};
use lr_contracts::world::{
    AwardXpOutcomeDto, CommentDto, NarrativeEntryDto, PlayerDto, PlayerSnapshotDto, PlayerStatDto,
    QuestDto, RuleDto, RuleExecutionDto, SkillDto, SkillSnapshotDto, SkillTreeDto,
    StatDefinitionDto, TransactionDto, TypeDefinitionDto, WorldOverviewDto,
};
use lr_contracts::CommandErrorDto;
use tauri::State;

#[tauri::command]
pub fn list_rules(state: State<'_, AppState>) -> Result<Vec<RuleDto>, CommandErrorDto> {
    state
        .world
        .list_rules()
        .map(|rules| rules.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
#[tauri::command]
pub fn create_rule(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
    priority: i32,
    definition: RuleDefinition,
) -> Result<RuleDto, CommandErrorDto> {
    state
        .world
        .create_rule_definition(&name, description, priority, definition)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn set_rule_enabled(
    state: State<'_, AppState>,
    rule_id: String,
    enabled: bool,
) -> Result<RuleDto, CommandErrorDto> {
    state
        .world
        .set_rule_enabled(&rule_id, enabled)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_rule_executions(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<RuleExecutionDto>, CommandErrorDto> {
    state
        .world
        .list_rule_executions(limit.unwrap_or(100))
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_type_definitions(
    state: State<'_, AppState>,
    namespace: Option<String>,
) -> Result<Vec<TypeDefinitionDto>, CommandErrorDto> {
    state
        .world
        .list_type_definitions(namespace.as_deref())
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

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
/// Set authored Player progression independently from the XP ledger.
#[tauri::command]
pub fn set_player_progression(
    state: State<'_, AppState>,
    player_id: String,
    level: i32,
    level_name: Option<String>,
    progression_label: Option<String>,
) -> Result<PlayerDto, CommandErrorDto> {
    state
        .world
        .set_player_progression(&player_id, level, level_name, progression_label)
        .map(Into::into)
        .map_err(Into::into)
}
/// Set authored Skill progression independently from the XP ledger.
#[tauri::command]
pub fn set_skill_progression(
    state: State<'_, AppState>,
    skill_id: String,
    level: i32,
    level_name: Option<String>,
    progression_label: Option<String>,
) -> Result<SkillDto, CommandErrorDto> {
    state
        .world
        .set_skill_progression(&skill_id, level, level_name, progression_label)
        .map(Into::into)
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
    description: Option<String>,
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
            description,
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
pub fn list_comments(
    state: State<'_, AppState>,
    target_kind: String,
    target_id: String,
) -> Result<Vec<CommentDto>, CommandErrorDto> {
    state
        .world
        .list_comments(&target_kind, &target_id)
        .map(|values| values.into_iter().map(Into::into).collect())
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
#[allow(clippy::too_many_arguments)]
pub fn create_narrative(
    state: State<'_, AppState>,
    player_id: String,
    kind: String,
    title: String,
    content: String,
    author: Option<String>,
    source_kind: Option<String>,
    source_id: Option<String>,
) -> Result<NarrativeEntryDto, CommandErrorDto> {
    state
        .world
        .create_narrative(
            &player_id,
            NarrativeWrite {
                kind,
                title,
                content,
                author,
                source_kind,
                source_id,
            },
        )
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn get_narrative_entry(
    state: State<'_, AppState>,
    player_id: String,
    content_id: String,
) -> Result<Option<NarrativeEntryDto>, CommandErrorDto> {
    state
        .world
        .get_narrative(&player_id, &content_id)
        .map(|value| value.map(Into::into))
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_narrative_entries(
    state: State<'_, AppState>,
    player_id: String,
) -> Result<Vec<NarrativeEntryDto>, CommandErrorDto> {
    state
        .world
        .world_overview(&player_id)
        .map(|world| world.narratives.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_narrative(
    state: State<'_, AppState>,
    player_id: String,
    content_id: String,
    kind: String,
    title: String,
    content: String,
    author: Option<String>,
    source_kind: Option<String>,
    source_id: Option<String>,
) -> Result<NarrativeEntryDto, CommandErrorDto> {
    state
        .world
        .update_narrative(
            &player_id,
            &content_id,
            NarrativeWrite {
                kind,
                title,
                content,
                author,
                source_kind,
                source_id,
            },
        )
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

#[tauri::command]
pub fn capture_skill_snapshot(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<SkillSnapshotDto, CommandErrorDto> {
    state
        .world
        .capture_skill_snapshot(&skill_id)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_player_snapshots(
    state: State<'_, AppState>,
    player_id: String,
) -> Result<Vec<PlayerSnapshotDto>, CommandErrorDto> {
    state
        .world
        .list_player_snapshots(&player_id)
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_skill_snapshots(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<Vec<SkillSnapshotDto>, CommandErrorDto> {
    state
        .world
        .list_skill_snapshots(&skill_id)
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
#[tauri::command]
pub fn define_stat(
    state: State<'_, AppState>,
    code: String,
    name: String,
    description: Option<String>,
    unit: Option<String>,
    minimum: Option<f64>,
    maximum: Option<f64>,
) -> Result<StatDefinitionDto, CommandErrorDto> {
    state
        .world
        .define_stat(&code, &name, description, unit, minimum, maximum)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_stat_definitions(
    state: State<'_, AppState>,
) -> Result<Vec<StatDefinitionDto>, CommandErrorDto> {
    state
        .world
        .list_stat_definitions()
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
#[tauri::command]
pub fn set_player_stat(
    state: State<'_, AppState>,
    player_id: String,
    stat_code: String,
    value: f64,
) -> Result<PlayerStatDto, CommandErrorDto> {
    state
        .world
        .set_player_stat(&player_id, &stat_code, value)
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub fn list_player_stats(
    state: State<'_, AppState>,
    player_id: String,
) -> Result<Vec<PlayerStatDto>, CommandErrorDto> {
    state
        .world
        .list_player_stats(&player_id)
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
