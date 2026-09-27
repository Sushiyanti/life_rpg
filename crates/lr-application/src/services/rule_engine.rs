//! Trusted, deterministic interpreter for the closed declarative Rule vocabulary.
//! All effects are planned first and committed as one WorldStore operation batch.
use crate::{
    rules::{
        Rule, RuleAction, RuleEvent, RuleExecutionError, RuleExecutionRecord, RuleOperation,
        MAX_ACTIONS_PER_CHAIN, MAX_RULE_CHAIN_DEPTH, MAX_RULE_EVALUATIONS_PER_CHAIN,
    },
    AppError, WorldStore,
};
use lr_domain::{EntityId, Iso8601Timestamp, Player, PlayerStat, Quest, Transaction};
use std::collections::{HashSet, VecDeque};

fn json<T: serde::Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string(value)
        .map_err(|e| AppError::Internal(format!("cannot serialize rule audit data: {e}")))
}
fn execution(
    rule: &Rule,
    event: &RuleEvent,
    chain_id: &str,
    depth: u16,
    at: &Iso8601Timestamp,
    passed: Option<bool>,
    status: &str,
    error: Option<String>,
    id: &mut impl FnMut() -> Result<String, AppError>,
) -> Result<RuleExecutionRecord, AppError> {
    Ok(RuleExecutionRecord {
        id: id()?,
        chain_id: chain_id.to_owned(),
        rule_id: rule.id.to_string(),
        event_kind: event.kind(),
        event_json: json(event)?,
        condition_passed: passed,
        actions_json: json(&rule.definition.actions)?,
        status: status.into(),
        error,
        depth,
        executed_at: at.clone(),
    })
}
fn pending_player<S: WorldStore>(
    store: &S,
    player_id: &EntityId,
    operations: &[RuleOperation],
) -> Result<Player, AppError> {
    for op in operations.iter().rev() {
        match op {
            RuleOperation::PlayerXp { player, .. } if player.id == *player_id => {
                return Ok(player.clone())
            }
            RuleOperation::CompleteQuest {
                reward: Some((player, _)),
                ..
            } if player.id == *player_id => return Ok(player.clone()),
            _ => {}
        }
    }
    store
        .get_player(player_id)?
        .ok_or_else(|| AppError::Internal(format!("player `{player_id}` not found")))
}
fn pending_quest<S: WorldStore>(
    store: &S,
    quest_id: &EntityId,
    operations: &[RuleOperation],
) -> Result<Quest, AppError> {
    for op in operations.iter().rev() {
        if let RuleOperation::CompleteQuest { quest, .. } = op {
            if quest.id == *quest_id {
                return Ok(quest.clone());
            }
        }
    }
    store
        .get_quest(quest_id)?
        .ok_or_else(|| AppError::Internal(format!("quest `{quest_id}` not found")))
}
fn pending_stat<S: WorldStore>(
    store: &S,
    player_id: &EntityId,
    code: &str,
    operations: &[RuleOperation],
) -> Result<Option<PlayerStat>, AppError> {
    for op in operations.iter().rev() {
        if let RuleOperation::PlayerStat { stat, .. } = op {
            if stat.player_id == *player_id && stat.stat_code == code {
                return Ok(Some(stat.clone()));
            }
        }
    }
    Ok(store
        .list_player_stats(player_id)?
        .into_iter()
        .find(|s| s.stat_code == code))
}

fn pending_concept_progress<S: WorldStore>(
    store: &S,
    concept_id: &EntityId,
    code: &str,
    operations: &[RuleOperation],
) -> Result<Option<lr_domain::ConceptProgressTrack>, AppError> {
    for operation in operations.iter().rev() {
        if let RuleOperation::ConceptProgress { track, .. } = operation {
            if track.concept_id == *concept_id && track.track_code == code {
                return Ok(Some(track.clone()));
            }
        }
    }
    Ok(store
        .list_progress_for_rule(concept_id)?
        .into_iter()
        .find(|track| track.track_code == code))
}

fn plan_action<S: WorldStore>(
    store: &S,
    event: &RuleEvent,
    rule: &Rule,
    action: &RuleAction,
    operations: &mut Vec<RuleOperation>,
    now: &Iso8601Timestamp,
    next_id: &mut impl FnMut() -> Result<String, AppError>,
) -> Result<Vec<RuleEvent>, AppError> {
    let player_id = EntityId::new(event.player_id())?;
    let mut generated = Vec::new();
    match action {
        RuleAction::AwardXp { amount, reason } => {
            let mut player = pending_player(store, &player_id, operations)?;
            let previous_xp = player.current_xp;
            player.apply_xp(*amount, now.clone())?;
            let applied_amount = player.current_xp - previous_xp;
            let mut tx = Transaction::xp_adjustment(
                player.id.clone(),
                *amount,
                applied_amount,
                now.clone(),
            )?;
            tx.reason = reason.clone().or_else(|| Some("rule_action".into()));
            tx.source_kind = Some("rule".into());
            tx.source_id = Some(rule.id.to_string());
            generated.push(RuleEvent::PlayerXpChanged {
                player_id: player.id.to_string(),
                previous_xp,
                current_xp: player.current_xp,
                requested_amount: *amount,
                applied_amount,
                player_level: player.level,
            });
            operations.push(RuleOperation::PlayerXp {
                player,
                previous_xp,
                transaction: tx,
            });
        }
        RuleAction::CompleteQuest { quest_id } => {
            let id = EntityId::new(quest_id)?;
            let mut quest = pending_quest(store, &id, operations)?;
            if quest.player_id != player_id {
                return Err(lr_domain::DomainError::invalid_value(
                    "rule Quest action",
                    "target Quest must belong to the event Player",
                )
                .into());
            }
            let expected_status = quest.status;
            quest.complete(now.clone())?;
            let reward = if quest.xp_reward > 0 {
                let mut player = pending_player(store, &quest.player_id, operations)?;
                let previous_xp = player.current_xp;
                player.apply_xp(quest.xp_reward, now.clone())?;
                let applied_amount = player.current_xp - previous_xp;
                let mut tx = Transaction::xp_adjustment(
                    player.id.clone(),
                    quest.xp_reward,
                    applied_amount,
                    now.clone(),
                )?;
                tx.reason = Some("quest_reward".into());
                tx.source_kind = Some("quest".into());
                tx.source_id = Some(quest.id.to_string());
                generated.push(RuleEvent::PlayerXpChanged {
                    player_id: player.id.to_string(),
                    previous_xp,
                    current_xp: player.current_xp,
                    requested_amount: quest.xp_reward,
                    applied_amount,
                    player_level: player.level,
                });
                Some((player, tx))
            } else {
                None
            };
            generated.push(RuleEvent::QuestCompleted {
                player_id: quest.player_id.to_string(),
                quest_id: quest.id.to_string(),
                quest_type: quest.quest_type.code.clone(),
                progress: quest.progress,
                xp_reward: quest.xp_reward,
            });
            operations.push(RuleOperation::CompleteQuest {
                quest,
                expected_status,
                reward,
            });
        }
        RuleAction::SetPlayerStat { stat_code, value } => {
            let definition = store
                .list_stat_definitions()?
                .into_iter()
                .find(|d| d.code == *stat_code)
                .ok_or_else(|| {
                    AppError::Internal(format!(
                        "rule refers to missing stat definition `{stat_code}`"
                    ))
                })?;
            let previous = pending_stat(store, &player_id, stat_code, operations)?;
            let stat = PlayerStat::new(player_id.clone(), &definition, *value, now.clone())?;
            generated.push(RuleEvent::StatChanged {
                player_id: player_id.to_string(),
                stat_code: stat_code.clone(),
                previous_value: previous.as_ref().map(|s| s.current_value),
                current_value: stat.current_value,
            });
            operations.push(RuleOperation::PlayerStat {
                stat,
                expected_previous: previous.map(|s| s.current_value),
            });
        }
        RuleAction::ModifyPlayerStat { stat_code, delta } => {
            let definition = store
                .list_stat_definitions()?
                .into_iter()
                .find(|d| d.code == *stat_code)
                .ok_or_else(|| {
                    AppError::Internal(format!(
                        "rule refers to missing stat definition `{stat_code}`"
                    ))
                })?;
            let previous = pending_stat(store, &player_id, stat_code, operations)?;
            let current = previous.as_ref().map(|s| s.current_value).unwrap_or(0.0);
            let next = current + delta;
            if !next.is_finite() {
                return Err(lr_domain::DomainError::invalid_value(
                    "rule stat action",
                    "result must be finite",
                )
                .into());
            }
            let stat = PlayerStat::new(player_id.clone(), &definition, next, now.clone())?;
            generated.push(RuleEvent::StatChanged {
                player_id: player_id.to_string(),
                stat_code: stat_code.clone(),
                previous_value: previous.as_ref().map(|s| s.current_value),
                current_value: stat.current_value,
            });
            operations.push(RuleOperation::PlayerStat {
                stat,
                expected_previous: previous.map(|s| s.current_value),
            });
        }
        RuleAction::SetConceptProgress {
            concept_id,
            track_code,
            value,
            level,
        } => {
            let concept_key = EntityId::new(concept_id)?;
            let concept = store.get_concept_for_rule(&concept_key)?.ok_or_else(|| {
                AppError::Internal(format!("rule refers to missing Concept `{concept_id}`"))
            })?;
            if concept.player_id != player_id {
                return Err(lr_domain::DomainError::invalid_value(
                    "rule Concept progress action",
                    "target Concept must belong to the event Player",
                )
                .into());
            }
            let definition = store
                .list_progress_definitions_for_rule()?
                .into_iter()
                .find(|d| d.code == *track_code && d.is_active)
                .ok_or_else(|| {
                    AppError::Internal(format!(
                        "rule refers to missing progress definition `{track_code}`"
                    ))
                })?;
            let previous = pending_concept_progress(store, &concept_key, track_code, operations)?;
            let track = match previous.as_ref() {
                Some(current) => {
                    if current.control != lr_domain::ProgressControl::RuleControlled {
                        return Err(lr_domain::DomainError::invalid_value(
                            "rule Concept progress action",
                            "target progress track is manual; explicitly delegate it to Rules first",
                        )
                        .into());
                    }
                    let mut next = current.clone();
                    next.change(&definition, *value, *level, now.clone())?;
                    next
                }
                None => {
                    return Err(lr_domain::DomainError::invalid_value(
                        "rule Concept progress action",
                        "target progress track must exist and be explicitly delegated to Rules",
                    )
                    .into())
                }
            };
            let history = lr_domain::ConceptProgressEntry::new(
                EntityId::new(next_id()?)?,
                &track,
                previous.as_ref().map(|p| p.current_value),
                now.clone(),
                now.clone(),
            )?;
            generated.push(RuleEvent::ConceptProgressChanged {
                player_id: player_id.to_string(),
                concept_id: concept.id.to_string(),
                concept_type: concept.concept_type.code,
                track_code: track_code.clone(),
                previous_value: previous.as_ref().map(|p| p.current_value),
                current_value: track.current_value,
                level: track.level,
            });
            operations.push(RuleOperation::ConceptProgress {
                player_id: player_id.clone(),
                source: crate::rules::ProgressMutationSource::Rule,
                track,
                expected_previous: previous.map(|p| p.current_value),
                history,
            });
        }
    }
    Ok(generated)
}

fn abort_records(records: &mut [RuleExecutionRecord], reason: &str) {
    for record in records {
        if record.status == "succeeded" {
            record.status = "guard_aborted".into();
            record.error = Some(reason.to_owned());
        }
    }
}
fn record_abort<S: WorldStore>(
    store: &S,
    records: &mut Vec<RuleExecutionRecord>,
    current: RuleExecutionRecord,
    reason: &str,
) -> Result<(), AppError> {
    abort_records(records, reason);
    records.push(current);
    store.record_rule_executions(records)?;
    Ok(())
}

/// Process an already-planned root change plus its typed domain event. Nothing is
/// persisted until the queue drains; any failure records an audit outcome and
/// leaves every state/ledger/rule-action write from the chain unapplied.
pub fn execute<S: WorldStore>(
    store: &S,
    root_events: Vec<RuleEvent>,
    root_operations: Vec<RuleOperation>,
    chain_id: String,
    now: Iso8601Timestamp,
    mut next_id: impl FnMut() -> Result<String, AppError>,
) -> Result<Vec<RuleOperation>, AppError> {
    let mut queue = root_events
        .into_iter()
        .map(|event| (event, 0u16))
        .collect::<VecDeque<_>>();
    let mut operations = root_operations;
    let mut records = Vec::<RuleExecutionRecord>::new();
    let mut seen = HashSet::<(String, String)>::new();
    let mut evaluations = 0usize;
    let mut actions = 0usize;
    while let Some((event, depth)) = queue.pop_front() {
        let rules = store.list_rules_for_event(event.kind())?;
        if rules.is_empty() {
            continue;
        }
        if depth as usize > MAX_RULE_CHAIN_DEPTH {
            let rule = &rules[0];
            let e = RuleExecutionError::MaxDepth(MAX_RULE_CHAIN_DEPTH);
            let record = execution(
                rule,
                &event,
                &chain_id,
                depth,
                &now,
                None,
                "guard_aborted",
                Some(e.to_string()),
                &mut next_id,
            )?;
            record_abort(store, &mut records, record, &e.to_string())?;
            return Err(e.into());
        }
        for rule in rules {
            evaluations += 1;
            if evaluations > MAX_RULE_EVALUATIONS_PER_CHAIN {
                let e = RuleExecutionError::MaxEvaluations(MAX_RULE_EVALUATIONS_PER_CHAIN);
                let record = execution(
                    &rule,
                    &event,
                    &chain_id,
                    depth,
                    &now,
                    None,
                    "guard_aborted",
                    Some(e.to_string()),
                    &mut next_id,
                )?;
                record_abort(store, &mut records, record, &e.to_string())?;
                return Err(e.into());
            }
            let key = (rule.id.to_string(), event.loop_key());
            if !seen.insert(key) {
                let e = RuleExecutionError::LoopDetected(
                    rule.id.to_string(),
                    event.kind().as_str().into(),
                );
                let record = execution(
                    &rule,
                    &event,
                    &chain_id,
                    depth,
                    &now,
                    None,
                    "guard_aborted",
                    Some(e.to_string()),
                    &mut next_id,
                )?;
                record_abort(store, &mut records, record, &e.to_string())?;
                return Err(e.into());
            }
            let passed = rule.definition.condition.evaluate(&event);
            if !passed {
                records.push(execution(
                    &rule,
                    &event,
                    &chain_id,
                    depth,
                    &now,
                    Some(false),
                    "condition_failed",
                    None,
                    &mut next_id,
                )?);
                continue;
            }
            if actions + rule.definition.actions.len() > MAX_ACTIONS_PER_CHAIN {
                let e = RuleExecutionError::MaxActions(MAX_ACTIONS_PER_CHAIN);
                let record = execution(
                    &rule,
                    &event,
                    &chain_id,
                    depth,
                    &now,
                    Some(true),
                    "guard_aborted",
                    Some(e.to_string()),
                    &mut next_id,
                )?;
                record_abort(store, &mut records, record, &e.to_string())?;
                return Err(e.into());
            }
            let mut rule_events = Vec::new();
            for (index, action) in rule.definition.actions.iter().enumerate() {
                actions += 1;
                match plan_action(
                    store,
                    &event,
                    &rule,
                    action,
                    &mut operations,
                    &now,
                    &mut next_id,
                ) {
                    Ok(mut generated) => rule_events.append(&mut generated),
                    Err(error) => {
                        let message = error.to_string();
                        let e = RuleExecutionError::ActionFailed {
                            rule_id: rule.id.to_string(),
                            action_index: index,
                            message: message.clone(),
                        };
                        let record = execution(
                            &rule,
                            &event,
                            &chain_id,
                            depth,
                            &now,
                            Some(true),
                            "failed",
                            Some(e.to_string()),
                            &mut next_id,
                        )?;
                        records.push(record);
                        // Previous planned successes are not committed after an action failure.
                        for prior in &mut records {
                            if prior.status == "succeeded" {
                                prior.status = "guard_aborted".into();
                                prior.error =
                                    Some("chain rolled back after a later action failure".into());
                            }
                        }
                        store.record_rule_executions(&records)?;
                        return Err(e.into());
                    }
                }
            }
            records.push(execution(
                &rule,
                &event,
                &chain_id,
                depth,
                &now,
                Some(true),
                "succeeded",
                None,
                &mut next_id,
            )?);
            for generated in rule_events {
                queue.push_back((generated, depth.saturating_add(1)));
            }
        }
    }
    match store.apply_rule_chain(&operations, &records) {
        Ok(stored) => Ok(stored),
        Err(storage) => {
            let message = storage.to_string();
            for record in &mut records {
                if record.status == "succeeded" {
                    record.status = "guard_aborted".into();
                    record.error =
                        Some("atomic chain rolled back after persistence failure".into());
                }
            }
            if let Some(last) = records
                .iter_mut()
                .rev()
                .find(|r| r.status == "guard_aborted")
            {
                last.status = "failed".into();
                last.error = Some(message.clone());
            }
            store.record_rule_executions(&records)?;
            Err(AppError::Storage(storage))
        }
    }
}
