-- Phase 9: Timeline becomes a reusable, strictly declarative Workspace panel.
-- Rebuild preserves every existing panel ID, filter, ordering, and presentation bit.
DROP INDEX idx_workspace_panels_order;
DROP INDEX idx_workspace_panels_concept;
ALTER TABLE workspace_panels RENAME TO workspace_panels_v14;

CREATE TABLE workspace_panels (
 id TEXT PRIMARY KEY,
 workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
 panel_type TEXT NOT NULL CHECK(panel_type IN ('quests','skills','concepts','effects','activity','transactions','journal','player','progress','timeline')),
 title TEXT CHECK(title IS NULL OR length(trim(title)) BETWEEN 1 AND 120),
 variant TEXT NOT NULL CHECK(variant IN ('cards','rows','compact','detailed','timeline','tree','metrics')),
 density TEXT NOT NULL CHECK(density IN ('cozy','compact')),
 filter_status TEXT CHECK(filter_status IS NULL OR filter_status IN ('open','active','inactive','in_progress','paused','pending','completed','abandoned','interrupted','archived')),
 filter_active INTEGER CHECK(filter_active IS NULL OR filter_active IN (0,1)),
 filter_type_code TEXT CHECK(filter_type_code IS NULL OR (length(filter_type_code) BETWEEN 1 AND 160 AND filter_type_code NOT GLOB '*[^a-z0-9_]*')),
 filter_concept_id TEXT REFERENCES concepts(id) ON DELETE SET NULL,
 filter_recent_days INTEGER CHECK(filter_recent_days IS NULL OR filter_recent_days BETWEEN 1 AND 365),
 filter_timeline_category TEXT CHECK(filter_timeline_category IS NULL OR filter_timeline_category IN ('session','transaction','effect_history','content','comment','concept_progress','revision','snapshot','record_change','lifecycle','relationship_history')),
 filter_timeline_entity_kind TEXT CHECK(filter_timeline_entity_kind IS NULL OR filter_timeline_entity_kind IN ('player','concept','quest','quest_stage','quest_branch','quest_session','skill_tree','skill','effect','transaction','comment','narrative_entry','concept_progress')),
 filter_timeline_entity_id TEXT CHECK(filter_timeline_entity_id IS NULL OR length(filter_timeline_entity_id) BETWEEN 1 AND 160),
 filter_timeline_from TEXT CHECK(filter_timeline_from IS NULL OR filter_timeline_from GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
 filter_timeline_through TEXT CHECK(filter_timeline_through IS NULL OR filter_timeline_through GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
 sort_by TEXT NOT NULL DEFAULT 'name_asc' CHECK(sort_by IN ('name_asc','updated_desc','created_desc','status_asc','progress_desc','level_desc','started_desc','occurred_desc','timeline_newest','timeline_oldest')),
 item_limit INTEGER NOT NULL DEFAULT 6 CHECK(item_limit BETWEEN 1 AND 50),
 sort_order INTEGER NOT NULL DEFAULT 0,
 grid_span INTEGER NOT NULL DEFAULT 1 CHECK(grid_span IN (1,2)),
 is_visible INTEGER NOT NULL DEFAULT 1 CHECK(is_visible IN (0,1)),
 is_pinned INTEGER NOT NULL DEFAULT 0 CHECK(is_pinned IN (0,1)),
 is_collapsed INTEGER NOT NULL DEFAULT 0 CHECK(is_collapsed IN (0,1)),
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL,
 CHECK(filter_timeline_from IS NULL OR filter_timeline_through IS NULL OR filter_timeline_from <= filter_timeline_through),
 CHECK((panel_type='timeline' AND sort_by IN ('timeline_newest','timeline_oldest')) OR (panel_type<>'timeline' AND sort_by NOT IN ('timeline_newest','timeline_oldest'))),
 CHECK(filter_timeline_entity_id IS NULL OR filter_timeline_entity_kind IS NOT NULL),
 CHECK(panel_type='timeline' OR (filter_timeline_category IS NULL AND filter_timeline_entity_kind IS NULL AND filter_timeline_entity_id IS NULL AND filter_timeline_from IS NULL AND filter_timeline_through IS NULL))
);

INSERT INTO workspace_panels(
 id,workspace_id,panel_type,title,variant,density,filter_status,filter_active,filter_type_code,
 filter_concept_id,filter_recent_days,sort_by,item_limit,sort_order,grid_span,is_visible,is_pinned,is_collapsed,created_at,updated_at
)
SELECT
 id,workspace_id,panel_type,title,variant,density,filter_status,filter_active,filter_type_code,
 filter_concept_id,filter_recent_days,sort_by,item_limit,sort_order,grid_span,is_visible,is_pinned,is_collapsed,created_at,updated_at
FROM workspace_panels_v14;
DROP TABLE workspace_panels_v14;

CREATE INDEX idx_workspace_panels_order ON workspace_panels(workspace_id,is_visible DESC,is_pinned DESC,sort_order,id);
CREATE INDEX idx_workspace_panels_concept ON workspace_panels(filter_concept_id) WHERE filter_concept_id IS NOT NULL;
CREATE INDEX idx_timeline_concepts_player_created ON concepts(player_id,created_at DESC,id);
CREATE INDEX idx_timeline_skill_trees_player_created ON skill_trees(player_id,created_at DESC,id);
CREATE INDEX idx_timeline_quest_stages_player_created ON quest_stages(player_id,created_at DESC,id);
CREATE INDEX idx_timeline_quest_branches_player_created ON quest_branches(player_id,created_at DESC,id);
