-- 0014_phase8_timeline_indexes
-- The unified Timeline composes the existing canonical source tables. These
-- indexes keep player/entity-scoped timestamp range and chronological scans
-- bounded without introducing a copied timeline/event table.
CREATE INDEX idx_timeline_effect_history_player_time
    ON effect_history(player_id, recorded_at DESC, id DESC);
CREATE INDEX idx_timeline_revisions_player_time
    ON entity_revisions(player_id, recorded_at DESC, id DESC);
CREATE INDEX idx_timeline_concept_progress_time
    ON concept_progress_history(concept_id, occurred_at DESC, id DESC);
CREATE INDEX idx_timeline_content_player_updated
    ON narrative_entries(player_id, updated_at DESC, id DESC);
CREATE INDEX idx_timeline_player_snapshots_capture
    ON player_state_snapshots(player_id, created_at DESC, id DESC);
CREATE INDEX idx_timeline_skill_snapshots_capture
    ON skill_state_snapshots(skill_id, created_at DESC, id DESC);
CREATE INDEX idx_timeline_concept_snapshots_capture
    ON concept_state_snapshots(concept_id, captured_at DESC, id DESC);
CREATE INDEX idx_timeline_quests_started
    ON quests(player_id, started_at, id) WHERE started_at IS NOT NULL;
CREATE INDEX idx_timeline_quests_completed
    ON quests(player_id, completed_at, id) WHERE completed_at IS NOT NULL;
CREATE INDEX idx_timeline_skills_started
    ON skills(skill_tree_id, started_at, id) WHERE started_at IS NOT NULL;
CREATE INDEX idx_timeline_skills_completed
    ON skills(skill_tree_id, completed_at, id) WHERE completed_at IS NOT NULL;
CREATE INDEX idx_timeline_lifecycle_player_time
    ON entity_lifecycle_history(player_id, occurred_at DESC, id DESC);
CREATE INDEX idx_timeline_content_relationship_created
    ON content_attachments(player_id, created_at DESC, id DESC);
CREATE INDEX idx_timeline_content_relationship_removed
    ON content_attachments(player_id, removed_at DESC, id DESC) WHERE removed_at IS NOT NULL;
