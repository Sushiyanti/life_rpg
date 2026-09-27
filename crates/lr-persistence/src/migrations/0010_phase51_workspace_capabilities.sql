-- Phase 5.1: reusable panel instances and bounded declarative query/layout options.
-- Rebuild in one migration so v9 panels retain their IDs and all presentation state.
DROP INDEX idx_workspace_panels_order;
ALTER TABLE workspace_panels RENAME TO workspace_panels_v9;
CREATE TABLE workspace_panels (
 id TEXT PRIMARY KEY,
 workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
 panel_type TEXT NOT NULL CHECK(panel_type IN ('quests','skills','concepts','effects','activity','transactions','journal','player','progress')),
 title TEXT CHECK(title IS NULL OR length(trim(title)) BETWEEN 1 AND 120),
 variant TEXT NOT NULL CHECK(variant IN ('cards','rows','compact','detailed','timeline','tree','metrics')),
 density TEXT NOT NULL CHECK(density IN ('cozy','compact')),
	filter_status TEXT CHECK(filter_status IS NULL OR filter_status IN ('open','active','inactive','in_progress','paused','pending','completed','abandoned','interrupted','archived')),
 filter_active INTEGER CHECK(filter_active IS NULL OR filter_active IN (0,1)),
 filter_type_code TEXT CHECK(filter_type_code IS NULL OR (length(filter_type_code) BETWEEN 1 AND 160 AND filter_type_code NOT GLOB '*[^a-z0-9_]*')),
 filter_concept_id TEXT REFERENCES concepts(id) ON DELETE SET NULL,
 filter_recent_days INTEGER CHECK(filter_recent_days IS NULL OR filter_recent_days BETWEEN 1 AND 365),
 sort_by TEXT NOT NULL DEFAULT 'name_asc' CHECK(sort_by IN ('name_asc','updated_desc','created_desc','status_asc','progress_desc','level_desc','started_desc','occurred_desc')),
 item_limit INTEGER NOT NULL DEFAULT 6 CHECK(item_limit BETWEEN 1 AND 50),
 sort_order INTEGER NOT NULL DEFAULT 0,
 grid_span INTEGER NOT NULL DEFAULT 1 CHECK(grid_span IN (1,2)),
 is_visible INTEGER NOT NULL DEFAULT 1 CHECK(is_visible IN (0,1)),
 is_pinned INTEGER NOT NULL DEFAULT 0 CHECK(is_pinned IN (0,1)),
 is_collapsed INTEGER NOT NULL DEFAULT 0 CHECK(is_collapsed IN (0,1)),
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
INSERT INTO workspace_panels(
 id,workspace_id,panel_type,title,variant,density,filter_status,item_limit,sort_order,is_pinned,is_collapsed,created_at,updated_at
)
SELECT id,workspace_id,panel_type,title,variant,density,
 CASE
  WHEN panel_type='quests' AND filter_status='in_progress' THEN 'active'
  WHEN panel_type='quests' AND filter_status='pending' THEN 'open'
  WHEN panel_type='activity' AND filter_status='active' THEN 'in_progress'
  WHEN panel_type='quests' AND filter_status NOT IN ('open','active','completed','abandoned') THEN NULL
  WHEN panel_type='skills' AND filter_status NOT IN ('active','paused','completed','archived') THEN NULL
  WHEN panel_type IN ('concepts','progress') AND filter_status NOT IN ('active','archived') THEN NULL
  WHEN panel_type='effects' AND filter_status NOT IN ('active','inactive') THEN NULL
  WHEN panel_type='activity' AND filter_status NOT IN ('in_progress','completed','interrupted') THEN NULL
  WHEN panel_type IN ('player','transactions','journal') THEN NULL
  ELSE filter_status
 END,
 item_limit,sort_order,is_pinned,is_collapsed,created_at,updated_at
FROM workspace_panels_v9;
DROP TABLE workspace_panels_v9;
CREATE INDEX idx_workspace_panels_order ON workspace_panels(workspace_id,is_visible DESC,is_pinned DESC,sort_order,id);
CREATE INDEX idx_workspace_panels_concept ON workspace_panels(filter_concept_id) WHERE filter_concept_id IS NOT NULL;
