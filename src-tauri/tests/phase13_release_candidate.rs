use life_rpg_lib::{bootstrap, WORLD_DB_FILENAME};
use lr_application::{
    Comparison, EventKind, NumericSubject, RuleAction, RuleCondition, SearchQuery, SearchSort,
    TimelineCategory, TimelineQuery, TimelineSort, WorkspacePanelImport, WorldStore,
};
use lr_domain::{
    ContentTargetKind, LifecycleState, ProgressSemantics, RevisionTargetKind, SessionStatus,
    SkillAvailability, SkillAvailabilityControl, TagMatchMode, TagTargetKind,
};
use lr_persistence::SqliteHealthStore;
use rusqlite::{Connection, ErrorCode};
use std::{fs, time::Instant};

const NOW: &str = "2026-09-29T11:00:00Z";

fn workspace_panel(tag_id: &str) -> WorkspacePanelImport {
    WorkspacePanelImport {
        panel_type: "quests".into(),
        title: Some("Active milestones".into()),
        variant: "detailed".into(),
        density: "compact".into(),
        filter_status: Some("active".into()),
        filter_active: None,
        filter_type_code: Some("main".into()),
        filter_concept_id: None,
        filter_tag_ids: vec![tag_id.into()],
        filter_tag_match: TagMatchMode::All,
        filter_recent_days: Some(30),
        filter_timeline_category: None,
        filter_timeline_entity_kind: None,
        filter_timeline_entity_id: None,
        filter_timeline_from: None,
        filter_timeline_through: None,
        sort_by: "updated_desc".into(),
        item_limit: 50,
        sort_order: 0,
        grid_span: 2,
        is_visible: true,
        is_pinned: true,
        is_collapsed: false,
    }
}

#[test]
fn populated_player_workflow_survives_backup_restore_and_repeated_restarts() {
    let root = tempfile::tempdir().expect("disposable app data directory");
    let db_path = root.path().join(WORLD_DB_FILENAME);
    let backup_path = root.path().join("populated-world.liferpg");
    let player_id;
    let quest_id;
    let skill_id;
    let concept_id;
    let tag_id;
    let workspace_id;
    let content_id;
    let effect_id;
    let session_id;

    {
        let state = bootstrap(root.path(), NOW);
        assert!(state.health.store().is_available());
        let player = state.world.create_player("Ada", None).unwrap();
        player_id = player.id.to_string();

        state
            .world
            .define_stat(
                "focus",
                "Focus",
                Some("Sustained attention".into()),
                Some("points".into()),
                Some(0.0),
                Some(10.0),
            )
            .unwrap();
        state
            .world
            .set_player_stat(&player_id, "focus", 4.0)
            .unwrap();

        let tree = state
            .world
            .create_skill_tree(&player_id, "programming", "Programming")
            .unwrap();
        let skill = state
            .world
            .add_skill(tree.id.as_str(), "core", "Rust", None)
            .unwrap();
        skill_id = skill.id.to_string();
        state
            .world
            .set_skill_availability(&skill_id, false)
            .unwrap();
        state
            .world
            .set_skill_availability_control(&skill_id, SkillAvailabilityControl::RuleControlled)
            .unwrap();
        let adjacent_skill = state
            .world
            .add_skill(
                tree.id.as_str(),
                "advanced",
                "Testing",
                Some(skill_id.clone()),
            )
            .unwrap();

        let concept = state
            .concepts
            .create_concept(
                &player_id,
                "project",
                "Release Candidate",
                Some("Validate the complete local-first workflow".into()),
            )
            .unwrap();
        concept_id = concept.id.to_string();
        let track = state
            .concepts
            .define_progress_track(
                "completion",
                "Completion",
                ProgressSemantics::Percentage,
                Some(0.0),
                Some(100.0),
            )
            .unwrap();
        assert_eq!(track.code, "completion");
        state
            .concepts
            .set_progress(&concept_id, "completion", 35.0, None, None)
            .unwrap();
        state
            .concepts
            .set_progress(&concept_id, "completion", 70.0, None, None)
            .unwrap();
        state
            .concepts
            .capture_snapshot(&concept_id, "2026-09-29")
            .unwrap();

        let quest = state
            .world
            .create_quest(
                &player_id,
                "main",
                "Ship the first release candidate",
                Some("Exercise persistence, progression, and recovery together".into()),
                None,
                Some(skill_id.clone()),
                Some(4),
                Some(40),
            )
            .unwrap();
        quest_id = quest.id.to_string();
        let stage = state
            .semantics
            .create_stage(&player_id, &quest_id, "Validation", 0)
            .unwrap();
        let branch = state
            .semantics
            .create_branch(stage.id.as_str(), "Linux release path", 0)
            .unwrap();
        state.world.start_quest(&quest_id).unwrap();

        let session = state
            .semantics
            .start_session(
                &player_id,
                Some(&quest_id),
                Some(stage.id.as_str()),
                Some(branch.id.as_str()),
                Some(&skill_id),
                Some(&concept_id),
                None,
            )
            .unwrap();
        session_id = session.id.to_string();

        let main_tag = state.tags.create_tag(&player_id, "Release", None).unwrap();
        tag_id = main_tag.id.to_string();
        let second_tag = state.tags.create_tag(&player_id, "Linux", None).unwrap();
        let quest_tag = state
            .tags
            .attach_tag(&player_id, &tag_id, TagTargetKind::Quest, &quest_id)
            .unwrap();
        state
            .tags
            .attach_tag(
                &player_id,
                second_tag.id.as_str(),
                TagTargetKind::Quest,
                &quest_id,
            )
            .unwrap();
        assert_eq!(
            state
                .tags
                .tags_for_target(&player_id, TagTargetKind::Quest, &quest_id)
                .unwrap()
                .len(),
            2
        );

        let workspace = state
            .semantics
            .import_workspace(
                &player_id,
                "Release dashboard",
                "learning",
                vec![workspace_panel(&tag_id)],
            )
            .unwrap();
        assert_eq!(workspace.1.len(), 1);
        assert!(workspace.1[0].is_pinned);
        workspace_id = workspace.0.id.to_string();

        let content = state
            .world
            .write_narrative(
                &player_id,
                "briefing",
                "Release checklist",
                "Validate the complete workflow and retain a known-good recovery point.",
            )
            .unwrap();
        content_id = content.id.to_string();
        state
            .semantics
            .attach_content(
                &player_id,
                &content_id,
                ContentTargetKind::Quest,
                &quest_id,
                "intro",
            )
            .unwrap();
        state
            .semantics
            .attach_content(
                &player_id,
                &content_id,
                ContentTargetKind::Session,
                &session_id,
                "reading",
            )
            .unwrap();
        state
            .world
            .add_comment(
                Some(player_id.clone()),
                "quest",
                &quest_id,
                "The end-to-end scenario is in progress.",
            )
            .unwrap();

        // A bounded cross-event chain: quest completion awards XP and creates
        // an Effect; XP awards Skill XP; Skill XP unlocks a delegated Skill.
        state
            .world
            .create_rule(
                "Quest completion reward",
                None,
                10,
                EventKind::QuestCompleted,
                RuleCondition::Always,
                vec![
                    RuleAction::AwardXp {
                        amount: 60,
                        reason: Some("release_scenario".into()),
                    },
                    RuleAction::ApplyEffect {
                        type_code: "buff".into(),
                        name: "Focused release work".into(),
                        description: Some("Created by the quest completion rule".into()),
                        target_concept_id: Some(concept_id.clone()),
                        intensity: 2,
                        expires_in_seconds: None,
                    },
                ],
            )
            .unwrap();
        state
            .world
            .create_rule(
                "XP builds the linked Skill",
                None,
                20,
                EventKind::PlayerXpChanged,
                RuleCondition::NumberCompare {
                    subject: NumericSubject::AppliedAmount,
                    comparison: Comparison::Equal,
                    value: 40.0,
                },
                vec![RuleAction::AwardSkillXp {
                    skill_id: skill_id.clone(),
                    delta: 15,
                    reason: Some("quest_chain".into()),
                }],
            )
            .unwrap();
        state
            .world
            .create_rule(
                "Skill XP unlock threshold",
                None,
                30,
                EventKind::SkillXpChanged,
                RuleCondition::NumberCompare {
                    subject: NumericSubject::CurrentSkillXp,
                    comparison: Comparison::GreaterOrEqual,
                    value: 15.0,
                },
                vec![RuleAction::UnlockSkill {
                    skill_id: skill_id.clone(),
                }],
            )
            .unwrap();

        state
            .semantics
            .finish_session(
                &session_id,
                None,
                SessionStatus::Completed,
                Some("Validated quest-stage and branch links".into()),
                Some("No records should be lost across restart".into()),
            )
            .unwrap();
        state.world.complete_quest(&quest_id).unwrap();

        let player_after = state.world.get_player(&player_id).unwrap().unwrap();
        assert_eq!(player_after.current_xp, 100, "40 quest XP + 60 rule XP");
        let unlocked = state
            .health
            .store()
            .get_skill(&lr_domain::EntityId::new(&skill_id).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(unlocked.current_xp, 15);
        assert_eq!(unlocked.availability, SkillAvailability::Available);
        assert_eq!(
            state
                .health
                .store()
                .get_skill(&adjacent_skill.id)
                .unwrap()
                .unwrap()
                .parent_skill_id
                .as_ref()
                .unwrap()
                .as_str(),
            skill_id
        );

        let effects = state.world.world_overview(&player_id).unwrap().effects;
        assert_eq!(effects.len(), 1);
        effect_id = effects[0].id.to_string();
        assert_eq!(
            effects[0].target_concept_id.as_ref().unwrap().as_str(),
            concept_id
        );
        assert_eq!(
            state
                .semantics
                .effect_history(&player_id, &effect_id)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            state.world.list_player_stats(&player_id).unwrap()[0].current_value,
            4.0
        );

        state
            .semantics
            .associate(
                &concept_id,
                lr_domain::AssociatedEntityKind::Quest,
                &quest_id,
                "related_to",
            )
            .unwrap();
        assert_eq!(
            state.concepts.progress_tracks(&concept_id).unwrap()[0].current_value,
            70.0
        );
        assert_eq!(
            state
                .semantics
                .associations(&concept_id, None, None)
                .unwrap()
                .len(),
            1
        );

        state
            .semantics
            .set_lifecycle(
                RevisionTargetKind::Quest,
                &quest_id,
                &player_id,
                LifecycleState::Archived,
                Some("completed release scenario"),
            )
            .unwrap();
        assert_eq!(
            state
                .semantics
                .lifecycle(RevisionTargetKind::Quest, &quest_id)
                .unwrap(),
            LifecycleState::Archived
        );
        let revisions = state
            .semantics
            .revisions(RevisionTargetKind::Quest, &quest_id)
            .unwrap();
        assert!(!revisions.is_empty());
        state
            .semantics
            .set_lifecycle(
                RevisionTargetKind::Quest,
                &quest_id,
                &player_id,
                LifecycleState::Active,
                Some("restore active workflow"),
            )
            .unwrap();

        state.world.capture_player_snapshot(&player_id).unwrap();
        state.world.capture_skill_snapshot(&skill_id).unwrap();
        assert_eq!(
            state.world.list_player_snapshots(&player_id).unwrap().len(),
            1
        );
        assert_eq!(
            state.world.list_skill_snapshots(&skill_id).unwrap().len(),
            1
        );

        let all_timeline = || {
            state
                .timeline
                .query(&TimelineQuery {
                    player_id: player.id.clone(),
                    category: None,
                    entity_kind: None,
                    entity_id: None,
                    concept_id: None,
                    from: None,
                    through: None,
                    sort: TimelineSort::Newest,
                    limit: 100,
                    offset: 0,
                })
                .unwrap()
        };
        let timeline_count = all_timeline().len();
        assert!(timeline_count >= 5);
        let timeline_transactions = state
            .timeline
            .query(&TimelineQuery {
                player_id: player.id.clone(),
                category: Some(TimelineCategory::Transaction),
                entity_kind: None,
                entity_id: None,
                concept_id: None,
                from: None,
                through: None,
                sort: TimelineSort::Newest,
                limit: 100,
                offset: 0,
            })
            .unwrap();
        assert!(!timeline_transactions.is_empty());
        assert_eq!(
            all_timeline().len(),
            timeline_count,
            "Timeline reads are not world mutations"
        );

        let search = state
            .semantics
            .search(&SearchQuery {
                text: Some("release".into()),
                player_id: Some(player.id.clone()),
                tag_ids: vec![lr_domain::EntityId::new(&tag_id).unwrap()],
                tag_match: TagMatchMode::All,
                sort: SearchSort::Relevance,
                limit: 50,
                offset: 0,
                ..SearchQuery::default()
            })
            .unwrap();
        assert!(!search.is_empty());
        assert_eq!(
            state.world.list_comments("quest", &quest_id).unwrap().len(),
            1
        );
        assert_eq!(
            state
                .semantics
                .content_for(&player_id, ContentTargetKind::Quest, &quest_id, false)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            state
                .tags
                .targets_for_tag(&player_id, &tag_id, false)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(state.world.list_rules().unwrap().len(), 3);

        let integrity = state.health.store().check_integrity().unwrap();
        assert!(integrity.healthy, "{integrity:?}");
        let backup_info = state
            .health
            .store()
            .create_backup(&backup_path, NOW)
            .unwrap();
        assert!(backup_info.integrity.healthy);
        let inspected = SqliteHealthStore::inspect_backup(&backup_path).unwrap();
        assert_eq!(inspected.sha256, backup_info.sha256);
        let restore_info = state
            .health
            .store()
            .restore_backup(
                &backup_path,
                &root.path().join("safety-backups"),
                &backup_info.sha256,
                false,
                NOW,
            )
            .unwrap();
        assert_eq!(restore_info.restored_schema_version, 17);
        assert_eq!(
            state
                .world
                .get_player(&player_id)
                .unwrap()
                .unwrap()
                .current_xp,
            100
        );
        assert_eq!(
            state
                .world
                .world_overview(&player_id)
                .unwrap()
                .effects
                .len(),
            1
        );
        assert!(state.health.store().check_integrity().unwrap().healthy);

        state
            .semantics
            .deactivate_effect(&player_id, &effect_id, None)
            .unwrap();
        assert_eq!(
            state
                .semantics
                .effect_history(&player_id, &effect_id)
                .unwrap()
                .len(),
            2
        );

        assert!(fs::metadata(&db_path).unwrap().len() > 0);
        assert!(!quest_tag.id.as_str().is_empty());
    }

    // Re-open the actual database repeatedly. Stable entity IDs, state,
    // history, relationship counts, and backup capability must survive each.
    for cycle in 1..=4 {
        let state = bootstrap(root.path(), NOW);
        assert!(state.health.store().is_available(), "restart {cycle}");
        assert_eq!(
            state
                .world
                .get_player(&player_id)
                .unwrap()
                .unwrap()
                .current_xp,
            100
        );
        assert_eq!(
            state
                .health
                .store()
                .get_quest(&lr_domain::EntityId::new(&quest_id).unwrap())
                .unwrap()
                .unwrap()
                .title,
            "Ship the first release candidate"
        );
        assert_eq!(
            state
                .health
                .store()
                .get_skill(&lr_domain::EntityId::new(&skill_id).unwrap())
                .unwrap()
                .unwrap()
                .current_xp,
            15
        );
        assert_eq!(
            state
                .health
                .store()
                .get_skill(&lr_domain::EntityId::new(&skill_id).unwrap())
                .unwrap()
                .unwrap()
                .availability,
            SkillAvailability::Available
        );
        assert_eq!(
            state
                .concepts
                .get_concept(&concept_id)
                .unwrap()
                .unwrap()
                .name,
            "Release Candidate"
        );
        assert_eq!(
            state
                .world
                .get_narrative(&player_id, &content_id)
                .unwrap()
                .unwrap()
                .title,
            "Release checklist"
        );
        assert_eq!(
            state
                .world
                .world_overview(&player_id)
                .unwrap()
                .effects
                .len(),
            1
        );
        assert_eq!(
            state
                .semantics
                .list_sessions(&player_id, None, None)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            state.world.list_comments("quest", &quest_id).unwrap().len(),
            1
        );
        assert_eq!(
            state
                .tags
                .tags_for_target(&player_id, TagTargetKind::Quest, &quest_id)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            state.semantics.list_workspaces(&player_id).unwrap().len(),
            1
        );
        assert_eq!(
            state
                .semantics
                .list_workspace_panels(&player_id, &workspace_id)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            state
                .semantics
                .effect_history(&player_id, &effect_id)
                .unwrap()
                .len(),
            2
        );
        assert!(state.health.store().check_integrity().unwrap().healthy);
        assert!(
            state
                .timeline
                .query(&TimelineQuery {
                    player_id: lr_domain::EntityId::new(&player_id).unwrap(),
                    category: None,
                    entity_kind: None,
                    entity_id: None,
                    concept_id: None,
                    from: None,
                    through: None,
                    sort: TimelineSort::Newest,
                    limit: 100,
                    offset: 0,
                })
                .unwrap()
                .len()
                >= 5
        );
        if cycle == 1 {
            let verified = state
                .health
                .store()
                .create_backup(&root.path().join("restart-check.liferpg"), NOW)
                .unwrap();
            assert!(verified.integrity.healthy);
        }
    }

    // The on-disk database must remain present and independently openable after
    // all service handles have been dropped.
    let final_state = bootstrap(root.path(), NOW);
    let final_integrity = final_state.health.store().check_integrity().unwrap();
    assert!(final_integrity.integrity_check_ok);
    assert_eq!(final_integrity.foreign_key_violations, 0);
    assert_eq!(final_integrity.schema_version, 17);
    assert!(db_path.is_file());
}

#[test]
fn bounded_large_world_search_timeline_workspace_and_backup_trial() {
    let root = tempfile::tempdir().unwrap();
    let db_path = root.path().join(WORLD_DB_FILENAME);
    let started = Instant::now();
    let state = bootstrap(root.path(), NOW);
    let startup_ms = started.elapsed().as_millis();
    let player = state.world.create_player("Stress Sample", None).unwrap();
    let player_id = player.id.to_string();
    let all_tag = state
        .tags
        .create_tag(&player_id, "stress-all", None)
        .unwrap();
    let subset_tag = state
        .tags
        .create_tag(&player_id, "stress-subset", None)
        .unwrap();

    let populate_started = Instant::now();
    let mut quest_ids = Vec::with_capacity(500);
    for index in 0..500 {
        let quest = state
            .world
            .create_quest(
                &player_id,
                "side",
                &format!("Stress campaign task {index:04}"),
                Some(format!(
                    "Synthetic medium-world record {index:04}; broad stress phrase."
                )),
                None,
                None,
                Some(2),
                Some(2),
            )
            .unwrap();
        quest_ids.push(quest.id.to_string());
        state.world.start_quest(&quest_ids[index]).unwrap();
        state.world.complete_quest(&quest_ids[index]).unwrap();
        state
            .tags
            .attach_tag(
                &player_id,
                all_tag.id.as_str(),
                TagTargetKind::Quest,
                &quest_ids[index],
            )
            .unwrap();
        if index % 5 == 0 {
            state
                .tags
                .attach_tag(
                    &player_id,
                    subset_tag.id.as_str(),
                    TagTargetKind::Quest,
                    &quest_ids[index],
                )
                .unwrap();
        }
    }
    let content_ids: Vec<String> = (0..100)
        .map(|index| {
            state
                .world
                .write_narrative(
                    &player_id,
                    "note",
                    &format!("Stress field note {index:03}"),
                    &format!("Reusable content for review batch {index:03}; broad stress phrase."),
                )
                .unwrap()
                .id
                .to_string()
        })
        .collect();
    for (content_index, content_id) in content_ids.iter().enumerate() {
        for slot in 0..5 {
            let quest_index = (content_index * 5 + slot) % quest_ids.len();
            state
                .semantics
                .attach_content(
                    &player_id,
                    content_id,
                    ContentTargetKind::Quest,
                    &quest_ids[quest_index],
                    "notes",
                )
                .unwrap();
        }
    }
    let populate_ms = populate_started.elapsed().as_millis();

    let panel = workspace_panel(all_tag.id.as_str());
    let workspace_started = Instant::now();
    let (_, panels) = state
        .semantics
        .import_workspace(&player_id, "Stress workspace", "learning", vec![panel; 30])
        .unwrap();
    let workspace_ms = workspace_started.elapsed().as_millis();
    assert_eq!(panels.len(), 30);
    assert_eq!(
        state.semantics.list_workspaces(&player_id).unwrap().len(),
        1
    );

    let mut broad = SearchQuery::default();
    broad.text = Some("stress".into());
    broad.player_id = Some(player.id.clone());
    broad.tag_ids = vec![all_tag.id.clone()];
    broad.tag_match = TagMatchMode::Any;
    broad.limit = 100;
    let search_started = Instant::now();
    let first_page = state.semantics.search(&broad).unwrap();
    let search_ms = search_started.elapsed().as_millis();
    assert_eq!(first_page.len(), 100, "search is bounded to requested page");
    let mut next_page_query = broad.clone();
    next_page_query.offset = 100;
    let next_page = state.semantics.search(&next_page_query).unwrap();
    assert_eq!(next_page.len(), 100);
    let first_ids: std::collections::HashSet<_> = first_page.iter().map(|hit| &hit.id).collect();
    assert!(next_page.iter().all(|hit| !first_ids.contains(&hit.id)));

    let mut all_filter = broad.clone();
    all_filter.tag_ids = vec![all_tag.id.clone(), subset_tag.id.clone()];
    all_filter.tag_match = TagMatchMode::All;
    all_filter.limit = 200;
    let all_hits = state.semantics.search(&all_filter).unwrap();
    assert_eq!(
        all_hits.len(),
        100,
        "ANY/ALL filters scope to common tagged subset"
    );
    let rare = state
        .semantics
        .search(&SearchQuery {
            text: Some("0499".into()),
            player_id: Some(player.id.clone()),
            tag_ids: vec![all_tag.id.clone()],
            limit: 20,
            ..SearchQuery::default()
        })
        .unwrap();
    assert_eq!(rare.len(), 1, "rare term finds its single matching task");

    let timeline_started = Instant::now();
    let timeline_query = TimelineQuery {
        player_id: player.id.clone(),
        category: Some(TimelineCategory::Transaction),
        entity_kind: None,
        entity_id: None,
        concept_id: None,
        from: None,
        through: None,
        sort: TimelineSort::Newest,
        limit: 200,
        offset: 0,
    };
    let timeline_first = state.timeline.query(&timeline_query).unwrap();
    let timeline_ms = timeline_started.elapsed().as_millis();
    assert_eq!(timeline_first.len(), 200);
    let timeline_repeat = state.timeline.query(&timeline_query).unwrap();
    assert_eq!(
        timeline_first
            .iter()
            .map(|item| &item.source_id)
            .collect::<Vec<_>>(),
        timeline_repeat
            .iter()
            .map(|item| &item.source_id)
            .collect::<Vec<_>>(),
        "equal-timestamp rows retain deterministic source ordering"
    );
    let mut timeline_second_query = timeline_query.clone();
    timeline_second_query.offset = 200;
    let timeline_second = state.timeline.query(&timeline_second_query).unwrap();
    assert_eq!(timeline_second.len(), 200);
    let timeline_ids: std::collections::HashSet<_> = timeline_first
        .iter()
        .chain(&timeline_second)
        .map(|item| &item.source_id)
        .collect();
    assert_eq!(
        timeline_ids.len(),
        400,
        "Timeline pages contain no duplicate source rows"
    );

    let integrity = state.health.store().check_integrity().unwrap();
    assert!(integrity.healthy, "{integrity:?}");
    let backup_path = root.path().join("medium-world.liferpg");
    let backup_started = Instant::now();
    let backup = state
        .health
        .store()
        .create_backup(&backup_path, NOW)
        .unwrap();
    let backup_ms = backup_started.elapsed().as_millis();
    assert!(backup.integrity.healthy);
    let restore_started = Instant::now();
    let restored = state
        .health
        .store()
        .restore_backup(
            &backup_path,
            &root.path().join("safety"),
            &backup.sha256,
            false,
            NOW,
        )
        .unwrap();
    let restore_ms = restore_started.elapsed().as_millis();
    assert_eq!(restored.restored_schema_version, 17);
    assert_eq!(
        state.world.world_overview(&player_id).unwrap().quests.len(),
        500
    );
    assert_eq!(
        state
            .world
            .world_overview(&player_id)
            .unwrap()
            .narratives
            .len(),
        100
    );
    assert!(state.health.store().check_integrity().unwrap().healthy);

    eprintln!(
        "PHASE13_MEDIUM_WORLD startup_ms={startup_ms} populate_500_quests_100_content_500_links_ms={populate_ms} workspace_30_panels_ms={workspace_ms} search_100_hits_ms={search_ms} timeline_200_transactions_ms={timeline_ms} backup_bytes={} backup_ms={backup_ms} restore_ms={restore_ms}",
        backup.database_bytes
    );
    assert!(fs::metadata(db_path).unwrap().len() > 0);
}

#[cfg(unix)]
#[test]
fn abrupt_child_termination_during_rule_execution_rolls_back_quest_and_xp() {
    use std::{
        process::Command,
        thread,
        time::{Duration, Instant},
    };

    let root = tempfile::tempdir().unwrap();
    let database = root.path().join(WORLD_DB_FILENAME);
    let player_id;
    let quest_id;
    let rule_id;
    {
        let state = bootstrap(root.path(), NOW);
        let player = state
            .world
            .create_player("Rule crash fixture", None)
            .unwrap();
        player_id = player.id.to_string();
        let quest = state
            .world
            .create_quest(
                &player_id,
                "main",
                "Crash during transactional Rule execution",
                None,
                None,
                None,
                None,
                Some(40),
            )
            .unwrap();
        quest_id = quest.id.to_string();
        state.world.start_quest(&quest_id).unwrap();
        let rule = state
            .world
            .create_rule(
                "Slow audited XP Rule",
                None,
                1,
                EventKind::QuestCompleted,
                RuleCondition::Always,
                vec![RuleAction::AwardXp {
                    amount: 25,
                    reason: Some("phase13_crash_fixture".into()),
                }],
            )
            .unwrap();
        rule_id = rule.id.to_string();
    }

    // This trigger exists only in the disposable test DB. It deliberately holds
    // SQLite's write transaction in the Rule audit insert long enough for the
    // parent process to detect the locked writer and deliver SIGKILL.
    let connection = Connection::open(&database).unwrap();
    connection
        .execute_batch(&format!(
            "CREATE TRIGGER phase13_slow_rule_audit BEFORE INSERT ON rule_execution_history
             WHEN NEW.rule_id = '{rule_id}' BEGIN
                 SELECT (WITH RECURSIVE delay(n) AS (
                     VALUES(1) UNION ALL SELECT n + 1 FROM delay WHERE n < 5000000
                 ) SELECT sum(n) FROM delay);
             END;"
        ))
        .unwrap();
    drop(connection);

    let started_marker = root.path().join("rule-started");
    let completed_marker = root.path().join("rule-completed");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("phase13_rule_child_completes_quest")
        .arg("--ignored")
        .arg("--nocapture")
        .env("LR_PHASE13_RULE_DATA", root.path())
        .env("LR_PHASE13_RULE_QUEST", &quest_id)
        .env("LR_PHASE13_RULE_STARTED", &started_marker)
        .env("LR_PHASE13_RULE_COMPLETED", &completed_marker)
        .spawn()
        .expect("spawn Rule execution child");

    let marker_deadline = Instant::now() + Duration::from_secs(5);
    while !started_marker.exists() && Instant::now() < marker_deadline {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("Rule child exited before beginning completion: {status}");
        }
        thread::sleep(Duration::from_millis(2));
    }
    assert!(
        started_marker.exists(),
        "child reached QuestCompleted operation"
    );

    let probe = Connection::open(&database).unwrap();
    probe.busy_timeout(Duration::ZERO).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut continuously_locked_since = None;
    let mut killed_in_rule_transaction = false;
    while Instant::now() < deadline {
        if completed_marker.exists() {
            panic!("Rule operation completed before SIGKILL injection");
        }
        if let Some(status) = child.try_wait().unwrap() {
            panic!("Rule child exited before SIGKILL injection: {status}");
        }
        match probe.execute_batch("BEGIN IMMEDIATE") {
            Ok(()) => {
                probe.execute_batch("ROLLBACK").unwrap();
                continuously_locked_since = None;
            }
            Err(rusqlite::Error::SqliteFailure(error, _))
                if matches!(
                    error.code,
                    ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked
                ) =>
            {
                let since = continuously_locked_since.get_or_insert_with(Instant::now);
                if since.elapsed() >= Duration::from_millis(150) {
                    child.kill().expect("SIGKILL child in Rule transaction");
                    let status = child.wait().unwrap();
                    use std::os::unix::process::ExitStatusExt;
                    assert_eq!(status.signal(), Some(9));
                    killed_in_rule_transaction = true;
                    break;
                }
            }
            Err(error) => panic!("unexpected SQLite lock probe error: {error}"),
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(
        killed_in_rule_transaction,
        "observed the slow Rule audit transaction"
    );
    drop(probe);

    let state = bootstrap(root.path(), NOW);
    let player = state
        .health
        .store()
        .get_player(&lr_domain::EntityId::new(&player_id).unwrap())
        .unwrap()
        .unwrap();
    let quest = state
        .health
        .store()
        .get_quest(&lr_domain::EntityId::new(&quest_id).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        player.current_xp, 0,
        "root and Rule XP writes roll back together"
    );
    assert_eq!(
        quest.status.as_str(),
        "active",
        "quest transition rolls back"
    );
    assert!(state.world.list_rule_executions(100).unwrap().is_empty());
    assert!(state
        .health
        .store()
        .list_transactions(&lr_domain::EntityId::new(&player_id).unwrap(), 100)
        .unwrap()
        .is_empty());
    assert!(state.health.store().check_integrity().unwrap().healthy);
}

#[cfg(unix)]
#[test]
#[ignore = "spawned only by the deterministic parent Rule SIGKILL test"]
fn phase13_rule_child_completes_quest() {
    let Ok(data_dir) = std::env::var("LR_PHASE13_RULE_DATA") else {
        return;
    };
    let quest_id = std::env::var("LR_PHASE13_RULE_QUEST").unwrap();
    let started = std::env::var("LR_PHASE13_RULE_STARTED").unwrap();
    let completed = std::env::var("LR_PHASE13_RULE_COMPLETED").unwrap();
    let state = bootstrap(std::path::Path::new(&data_dir), NOW);
    fs::write(started, b"starting quest completion").unwrap();
    let result = state.world.complete_quest(&quest_id);
    fs::write(completed, format!("{result:?}")).unwrap();
    panic!("parent should SIGKILL the child during Rule audit persistence");
}
