-- Phase 5: workspaces are per-Player presentation settings, not domain entities.
CREATE TABLE workspaces (
 id TEXT PRIMARY KEY,
 player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
 name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 80),
 template TEXT NOT NULL CHECK(template IN ('overview','focus','learning','health','review','custom')),
 sort_order INTEGER NOT NULL DEFAULT 0,
 is_default INTEGER NOT NULL DEFAULT 0 CHECK(is_default IN (0,1)),
 created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
 UNIQUE(player_id,name COLLATE NOCASE)
);
CREATE UNIQUE INDEX idx_workspaces_single_default ON workspaces(player_id) WHERE is_default=1;
CREATE INDEX idx_workspaces_player_order ON workspaces(player_id,sort_order,id);
CREATE TABLE workspace_panels (
 id TEXT PRIMARY KEY,
 workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
 panel_type TEXT NOT NULL CHECK(panel_type IN ('quests','skills','concepts','effects','activity','transactions','journal')),
 title TEXT CHECK(title IS NULL OR length(trim(title)) BETWEEN 1 AND 120),
 variant TEXT NOT NULL CHECK(variant IN ('cards','rows')),
 density TEXT NOT NULL CHECK(density IN ('cozy','compact')),
 filter_status TEXT CHECK(filter_status IS NULL OR filter_status IN ('active','in_progress','pending','completed','archived')),
 item_limit INTEGER NOT NULL DEFAULT 6 CHECK(item_limit BETWEEN 1 AND 50),
 sort_order INTEGER NOT NULL DEFAULT 0,
 is_pinned INTEGER NOT NULL DEFAULT 0 CHECK(is_pinned IN (0,1)),
 is_collapsed INTEGER NOT NULL DEFAULT 0 CHECK(is_collapsed IN (0,1)),
 created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
 UNIQUE(workspace_id,panel_type)
);
CREATE INDEX idx_workspace_panels_order ON workspace_panels(workspace_id,is_pinned DESC,sort_order,id);
