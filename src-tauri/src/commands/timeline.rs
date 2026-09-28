//! Timeline IPC is a typed adapter only; query semantics live in application.

use crate::state::AppState;
use lr_contracts::timeline::{TimelineItemDto, TimelineQueryDto};
use lr_contracts::CommandErrorDto;
use tauri::State;

#[tauri::command]
pub fn query_timeline(
    state: State<'_, AppState>,
    query: TimelineQueryDto,
) -> Result<Vec<TimelineItemDto>, CommandErrorDto> {
    let query = lr_application::TimelineQuery::try_from(query)
        .map_err(|error| CommandErrorDto::from(lr_application::AppError::from(error)))?;
    state
        .timeline
        .query(&query)
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
