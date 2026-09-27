-- 0008_phase36_world_semantics
-- Preserve Phase 3.5 history. These are bounded world types and named entity
-- references, not a universal entity/property or graph database.

-- Experience is independent of manually authored level state.
ALTER TABLE players ADD COLUMN level_name TEXT;
ALTER TABLE players ADD COLUMN progression_label TEXT;
ALTER TABLE skills ADD COLUMN level_name TEXT;
ALTER TABLE skills ADD COLUMN progression_label TEXT;
ALTER TABLE concept_progress_tracks ADD COLUMN level_name TEXT;
ALTER TABLE concept_progress_tracks ADD COLUMN progression_label TEXT;
ALTER TABLE concept_progress_tracks ADD COLUMN control TEXT NOT NULL DEFAULT 'manual'
 CHECK(control IN ('manual','rule_controlled'));

-- NarrativeEntry is the backward-compatible intentional Content entity.
ALTER TABLE narrative_entries ADD COLUMN is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0,1));

CREATE TABLE quest_stages (
 id TEXT PRIMARY KEY,
 player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
 quest_id TEXT NOT NULL REFERENCES quests(id) ON DELETE CASCADE,
 title TEXT NOT NULL CHECK(length(trim(title)) BETWEEN 1 AND 512),
 description TEXT, story TEXT, instructions TEXT,
 status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','active','completed','skipped')),
 sort_order INTEGER NOT NULL DEFAULT 0,
 is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0,1)),
 metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),
 created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
 UNIQUE(id,player_id,quest_id)
);
CREATE INDEX idx_quest_stages_order ON quest_stages(quest_id,is_active,sort_order,id);
CREATE TRIGGER quest_stage_owner_insert BEFORE INSERT ON quest_stages BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM quests WHERE id=NEW.quest_id AND player_id=NEW.player_id) THEN RAISE(ABORT,'Stage Quest must belong to Player') END;
END;
CREATE TRIGGER quest_stage_owner_update BEFORE UPDATE OF player_id,quest_id ON quest_stages BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM quests WHERE id=NEW.quest_id AND player_id=NEW.player_id) THEN RAISE(ABORT,'Stage Quest must belong to Player') END;
END;

CREATE TABLE quest_branches (
 id TEXT PRIMARY KEY,
 player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
 quest_id TEXT NOT NULL REFERENCES quests(id) ON DELETE CASCADE,
 stage_id TEXT NOT NULL REFERENCES quest_stages(id) ON DELETE CASCADE,
 title TEXT NOT NULL CHECK(length(trim(title)) BETWEEN 1 AND 512),
 description TEXT,
 status TEXT NOT NULL DEFAULT 'available' CHECK(status IN ('available','chosen','rejected','completed')),
 sort_order INTEGER NOT NULL DEFAULT 0,
 is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0,1)),
 metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),
 created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
 UNIQUE(id,player_id,quest_id,stage_id)
);
CREATE INDEX idx_quest_branches_order ON quest_branches(stage_id,is_active,sort_order,id);
CREATE TRIGGER quest_branch_owner_insert BEFORE INSERT ON quest_branches BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM quest_stages s WHERE s.id=NEW.stage_id AND s.quest_id=NEW.quest_id AND s.player_id=NEW.player_id) THEN RAISE(ABORT,'Branch Stage/Quest must belong to Player') END;
END;
CREATE TRIGGER quest_branch_owner_update BEFORE UPDATE OF player_id,quest_id,stage_id ON quest_branches BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM quest_stages s WHERE s.id=NEW.stage_id AND s.quest_id=NEW.quest_id AND s.player_id=NEW.player_id) THEN RAISE(ABORT,'Branch Stage/Quest must belong to Player') END;
END;

-- One row is one actual period of activity; calendar-date snapshots remain separate.
CREATE TABLE quest_sessions (
 id TEXT PRIMARY KEY,
 player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
 quest_id TEXT REFERENCES quests(id) ON DELETE SET NULL,
 stage_id TEXT REFERENCES quest_stages(id) ON DELETE SET NULL,
 branch_id TEXT REFERENCES quest_branches(id) ON DELETE SET NULL,
 skill_id TEXT REFERENCES skills(id) ON DELETE SET NULL,
 concept_id TEXT REFERENCES concepts(id) ON DELETE SET NULL,
 started_at TEXT NOT NULL,
 ended_at TEXT,
 status TEXT NOT NULL DEFAULT 'in_progress' CHECK(status IN ('in_progress','completed','interrupted')),
 progress_before INTEGER CHECK(progress_before IS NULL OR progress_before BETWEEN 0 AND 100),
 progress_after INTEGER CHECK(progress_after IS NULL OR progress_after BETWEEN 0 AND 100),
 result TEXT, notes TEXT,
 is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0,1)),
 metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),
 created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
 CHECK(ended_at IS NULL OR ended_at>=started_at),
 CHECK(status='in_progress' OR ended_at IS NOT NULL),
 CHECK(quest_id IS NOT NULL OR stage_id IS NOT NULL OR branch_id IS NOT NULL OR skill_id IS NOT NULL OR concept_id IS NOT NULL),
 CHECK(branch_id IS NULL OR stage_id IS NOT NULL)
);
CREATE INDEX idx_quest_sessions_player_time ON quest_sessions(player_id,started_at DESC,id);
CREATE INDEX idx_quest_sessions_quest_time ON quest_sessions(quest_id,started_at DESC);
CREATE INDEX idx_quest_sessions_stage_time ON quest_sessions(stage_id,started_at DESC);
CREATE TRIGGER quest_session_owner_insert BEFORE INSERT ON quest_sessions BEGIN
 SELECT CASE WHEN NEW.quest_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM quests WHERE id=NEW.quest_id AND player_id=NEW.player_id) THEN RAISE(ABORT,'Session Quest must belong to Player') END;
 SELECT CASE WHEN NEW.stage_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM quest_stages WHERE id=NEW.stage_id AND player_id=NEW.player_id AND (NEW.quest_id IS NULL OR quest_id=NEW.quest_id)) THEN RAISE(ABORT,'Session Stage must belong to Player/Quest') END;
 SELECT CASE WHEN NEW.branch_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM quest_branches WHERE id=NEW.branch_id AND player_id=NEW.player_id AND stage_id=NEW.stage_id) THEN RAISE(ABORT,'Session Branch must belong to Stage/Player') END;
 SELECT CASE WHEN NEW.skill_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE s.id=NEW.skill_id AND t.player_id=NEW.player_id) THEN RAISE(ABORT,'Session Skill must belong to Player') END;
 SELECT CASE WHEN NEW.concept_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM concepts WHERE id=NEW.concept_id AND player_id=NEW.player_id) THEN RAISE(ABORT,'Session Concept must belong to Player') END;
END;
CREATE TRIGGER quest_session_owner_update BEFORE UPDATE OF player_id,quest_id,stage_id,branch_id,skill_id,concept_id ON quest_sessions BEGIN
 SELECT CASE WHEN NEW.quest_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM quests WHERE id=NEW.quest_id AND player_id=NEW.player_id) THEN RAISE(ABORT,'Session Quest must belong to Player') END;
 SELECT CASE WHEN NEW.stage_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM quest_stages WHERE id=NEW.stage_id AND player_id=NEW.player_id AND (NEW.quest_id IS NULL OR quest_id=NEW.quest_id)) THEN RAISE(ABORT,'Session Stage must belong to Player/Quest') END;
 SELECT CASE WHEN NEW.branch_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM quest_branches WHERE id=NEW.branch_id AND player_id=NEW.player_id AND stage_id=NEW.stage_id) THEN RAISE(ABORT,'Session Branch must belong to Stage/Player') END;
 SELECT CASE WHEN NEW.skill_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE s.id=NEW.skill_id AND t.player_id=NEW.player_id) THEN RAISE(ABORT,'Session Skill must belong to Player') END;
 SELECT CASE WHEN NEW.concept_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM concepts WHERE id=NEW.concept_id AND player_id=NEW.player_id) THEN RAISE(ABORT,'Session Concept must belong to Player') END;
END;

-- A bounded, data-defined vocabulary for intentional content roles and associations.
CREATE TABLE concept_association_types(code TEXT PRIMARY KEY CHECK(length(code) BETWEEN 1 AND 160 AND code NOT GLOB '*[^a-z0-9_]*'),label TEXT NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL);
INSERT INTO concept_association_types VALUES
 ('about','About',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('related_to','Related to',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('reference','Reference',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('contains','Contains',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('prerequisite','Prerequisite',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('subject','Subject',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('result_of','Result of',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'));
CREATE TABLE concept_associations (
 id TEXT PRIMARY KEY, player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
 concept_id TEXT NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,
 entity_kind TEXT NOT NULL CHECK(entity_kind IN ('quest','quest_stage','quest_branch','quest_session','skill','skill_tree','effect','transaction','comment','narrative_entry','concept_progress')),
 entity_id TEXT NOT NULL, association_code TEXT NOT NULL REFERENCES concept_association_types(code) ON DELETE RESTRICT,
 is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0,1)), metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),
 created_at TEXT NOT NULL,updated_at TEXT NOT NULL, UNIQUE(concept_id,entity_kind,entity_id,association_code)
);
CREATE INDEX idx_concept_associations_entity ON concept_associations(entity_kind,entity_id,is_active,concept_id);
CREATE INDEX idx_concept_associations_concept ON concept_associations(concept_id,is_active,entity_kind);
CREATE TRIGGER concept_association_owner_insert BEFORE INSERT ON concept_associations BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM concepts WHERE id=NEW.concept_id AND player_id=NEW.player_id) THEN RAISE(ABORT,'association Concept must belong to Player') END;
 SELECT CASE WHEN NOT (
 (NEW.entity_kind='quest' AND EXISTS(SELECT 1 FROM quests WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
 (NEW.entity_kind='quest_stage' AND EXISTS(SELECT 1 FROM quest_stages WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
 (NEW.entity_kind='quest_branch' AND EXISTS(SELECT 1 FROM quest_branches WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
 (NEW.entity_kind='quest_session' AND EXISTS(SELECT 1 FROM quest_sessions WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
 (NEW.entity_kind='skill' AND EXISTS(SELECT 1 FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE s.id=NEW.entity_id AND t.player_id=NEW.player_id)) OR
 (NEW.entity_kind='skill_tree' AND EXISTS(SELECT 1 FROM skill_trees WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
 (NEW.entity_kind='effect' AND EXISTS(SELECT 1 FROM effects WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
 (NEW.entity_kind='transaction' AND EXISTS(SELECT 1 FROM transactions WHERE CAST(id AS TEXT)=NEW.entity_id AND player_id=NEW.player_id)) OR
 (NEW.entity_kind='comment' AND EXISTS(SELECT 1 FROM comments WHERE CAST(id AS TEXT)=NEW.entity_id AND author_player_id=NEW.player_id)) OR
 (NEW.entity_kind='narrative_entry' AND EXISTS(SELECT 1 FROM narrative_entries WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
 (NEW.entity_kind='concept_progress' AND EXISTS(SELECT 1 FROM concept_progress_history h JOIN concepts c ON c.id=h.concept_id WHERE h.id=NEW.entity_id AND c.player_id=NEW.player_id AND c.id=NEW.concept_id))
 ) THEN RAISE(ABORT,'Concept association target missing or outside Player world') END;
END;
CREATE TRIGGER concept_association_owner_update BEFORE UPDATE OF player_id,concept_id,entity_kind,entity_id ON concept_associations BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM concepts WHERE id=NEW.concept_id AND player_id=NEW.player_id) THEN RAISE(ABORT,'association Concept must belong to Player') END;
 SELECT CASE WHEN NOT (
 (NEW.entity_kind='quest' AND EXISTS(SELECT 1 FROM quests WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
 (NEW.entity_kind='quest_stage' AND EXISTS(SELECT 1 FROM quest_stages WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
 (NEW.entity_kind='quest_branch' AND EXISTS(SELECT 1 FROM quest_branches WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
 (NEW.entity_kind='quest_session' AND EXISTS(SELECT 1 FROM quest_sessions WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
 (NEW.entity_kind='skill' AND EXISTS(SELECT 1 FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE s.id=NEW.entity_id AND t.player_id=NEW.player_id)) OR
 (NEW.entity_kind='skill_tree' AND EXISTS(SELECT 1 FROM skill_trees WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
 (NEW.entity_kind='effect' AND EXISTS(SELECT 1 FROM effects WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
 (NEW.entity_kind='transaction' AND EXISTS(SELECT 1 FROM transactions WHERE CAST(id AS TEXT)=NEW.entity_id AND player_id=NEW.player_id)) OR
 (NEW.entity_kind='comment' AND EXISTS(SELECT 1 FROM comments WHERE CAST(id AS TEXT)=NEW.entity_id AND author_player_id=NEW.player_id)) OR
 (NEW.entity_kind='narrative_entry' AND EXISTS(SELECT 1 FROM narrative_entries WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
 (NEW.entity_kind='concept_progress' AND EXISTS(SELECT 1 FROM concept_progress_history h JOIN concepts c ON c.id=h.concept_id WHERE h.id=NEW.entity_id AND c.player_id=NEW.player_id AND c.id=NEW.concept_id))
 ) THEN RAISE(ABORT,'Concept association target missing or outside Player world') END;
END;

CREATE TABLE content_attachment_roles(code TEXT PRIMARY KEY CHECK(length(code) BETWEEN 1 AND 160 AND code NOT GLOB '*[^a-z0-9_]*'),label TEXT NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL);
INSERT INTO content_attachment_roles VALUES
 ('intro','Introduction',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('guidance','Guidance',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('reading','Reading',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('instructions','Instructions',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('todo','To do',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('reminder','Reminder',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('notes','Notes',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('reflection','Reflection',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('summary','Summary',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('content','Content',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'));
CREATE TABLE content_attachments(content_id TEXT NOT NULL REFERENCES narrative_entries(id) ON DELETE CASCADE,player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,target_kind TEXT NOT NULL CHECK(target_kind IN ('quest','quest_stage','quest_branch','quest_session','skill','skill_tree','concept')),target_id TEXT NOT NULL,role_code TEXT NOT NULL REFERENCES content_attachment_roles(code) ON DELETE RESTRICT,is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0,1)),created_at TEXT NOT NULL,updated_at TEXT NOT NULL,PRIMARY KEY(content_id,target_kind,target_id,role_code));
CREATE INDEX idx_content_attachments_target ON content_attachments(target_kind,target_id,is_active,role_code);
CREATE TRIGGER content_attachment_owner_insert BEFORE INSERT ON content_attachments BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM narrative_entries n WHERE n.id=NEW.content_id AND n.player_id=NEW.player_id) THEN RAISE(ABORT,'Content must belong to Player') END;
 SELECT CASE WHEN NOT ((NEW.target_kind='quest' AND EXISTS(SELECT 1 FROM quests WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR (NEW.target_kind='quest_stage' AND EXISTS(SELECT 1 FROM quest_stages WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR (NEW.target_kind='quest_branch' AND EXISTS(SELECT 1 FROM quest_branches WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR (NEW.target_kind='quest_session' AND EXISTS(SELECT 1 FROM quest_sessions WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR (NEW.target_kind='skill' AND EXISTS(SELECT 1 FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE s.id=NEW.target_id AND t.player_id=NEW.player_id)) OR (NEW.target_kind='skill_tree' AND EXISTS(SELECT 1 FROM skill_trees WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR (NEW.target_kind='concept' AND EXISTS(SELECT 1 FROM concepts WHERE id=NEW.target_id AND player_id=NEW.player_id))) THEN RAISE(ABORT,'Content target missing or outside Player world') END;
END;

-- Proposals are not state. Acceptance must go through the explicit progress use case.
CREATE TABLE progress_suggestions(id TEXT PRIMARY KEY,player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,concept_id TEXT NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,track_code TEXT NOT NULL REFERENCES progress_track_definitions(code) ON DELETE RESTRICT,proposed_value REAL NOT NULL,proposed_level INTEGER,reason TEXT,source TEXT NOT NULL,status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','accepted','rejected')),created_at TEXT NOT NULL,resolved_at TEXT,metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),CHECK(proposed_value BETWEEN -1.7976931348623157e308 AND 1.7976931348623157e308),CHECK(proposed_level IS NULL OR proposed_level>=1));
CREATE INDEX idx_progress_suggestions_pending ON progress_suggestions(player_id,status,created_at DESC);
CREATE TRIGGER progress_suggestion_owner BEFORE INSERT ON progress_suggestions BEGIN SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM concepts WHERE id=NEW.concept_id AND player_id=NEW.player_id) THEN RAISE(ABORT,'suggestion Concept must belong to Player') END; END;
CREATE TRIGGER progress_suggestion_immutable_update BEFORE UPDATE ON progress_suggestions WHEN OLD.status<>'pending' BEGIN SELECT RAISE(ABORT,'resolved suggestions are immutable'); END;
CREATE TRIGGER progress_suggestion_no_delete BEFORE DELETE ON progress_suggestions BEGIN SELECT RAISE(ABORT,'suggestions are retained'); END;

-- Recoverable, typed revisions. Snapshot payloads are JSON, but target kinds are closed.
CREATE TABLE entity_revisions(id TEXT PRIMARY KEY,player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,target_kind TEXT NOT NULL CHECK(target_kind IN ('player','quest','skill_tree','skill','concept','concept_progress','quest_stage','quest_branch','quest_session','narrative_entry')),target_id TEXT NOT NULL,revision_number INTEGER NOT NULL CHECK(revision_number>=1),recorded_at TEXT NOT NULL,author_player_id TEXT REFERENCES players(id) ON DELETE SET NULL,reason TEXT,snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json)),metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),UNIQUE(target_kind,target_id,revision_number));
CREATE INDEX idx_entity_revisions_target ON entity_revisions(target_kind,target_id,revision_number DESC);
CREATE TRIGGER entity_revisions_immutable_update BEFORE UPDATE ON entity_revisions BEGIN SELECT RAISE(ABORT,'entity revisions are append-only'); END;
CREATE TRIGGER entity_revisions_immutable_delete BEFORE DELETE ON entity_revisions BEGIN SELECT RAISE(ABORT,'entity revisions are append-only'); END;

-- Lifecycle transitions are separate from active state and UI visibility.
CREATE TABLE entity_lifecycle(target_kind TEXT NOT NULL CHECK(target_kind IN ('player','quest','skill_tree','skill','concept','concept_progress','quest_stage','quest_branch','quest_session','narrative_entry')),target_id TEXT NOT NULL,player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,state TEXT NOT NULL CHECK(state IN ('active','archived','trashed')),updated_at TEXT NOT NULL,PRIMARY KEY(target_kind,target_id));
CREATE TABLE entity_lifecycle_history(id TEXT PRIMARY KEY,target_kind TEXT NOT NULL,target_id TEXT NOT NULL,player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,previous_state TEXT CHECK(previous_state IS NULL OR previous_state IN ('active','archived','trashed')),current_state TEXT NOT NULL CHECK(current_state IN ('active','archived','trashed')),occurred_at TEXT NOT NULL,captured_at TEXT NOT NULL,reason TEXT,CHECK(occurred_at<=captured_at));
CREATE INDEX idx_lifecycle_history_target ON entity_lifecycle_history(target_kind,target_id,occurred_at,id);
CREATE TRIGGER lifecycle_history_immutable_update BEFORE UPDATE ON entity_lifecycle_history BEGIN SELECT RAISE(ABORT,'lifecycle history is append-only'); END;
CREATE TRIGGER lifecycle_history_immutable_delete BEFORE DELETE ON entity_lifecycle_history BEGIN SELECT RAISE(ABORT,'lifecycle history is append-only'); END;

-- UI curation is contextual, never a property of existence/activity.
CREATE TABLE presentation_preferences(player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,entity_kind TEXT NOT NULL CHECK(entity_kind IN ('player','quest','skill_tree','skill','concept','quest_stage','quest_branch','quest_session','narrative_entry','effect')),entity_id TEXT NOT NULL,context TEXT NOT NULL CHECK(length(context) BETWEEN 1 AND 160 AND context NOT GLOB '*[^a-z0-9_]*'),is_visible INTEGER NOT NULL DEFAULT 1 CHECK(is_visible IN (0,1)),sort_order INTEGER NOT NULL DEFAULT 0,is_pinned INTEGER NOT NULL DEFAULT 0 CHECK(is_pinned IN (0,1)),is_collapsed INTEGER CHECK(is_collapsed IS NULL OR is_collapsed IN (0,1)),variant TEXT,density TEXT,metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),created_at TEXT NOT NULL,updated_at TEXT NOT NULL,PRIMARY KEY(player_id,entity_kind,entity_id,context));
CREATE INDEX idx_presentation_order ON presentation_preferences(player_id,context,is_visible,is_pinned DESC,sort_order,entity_id);
CREATE TRIGGER presentation_owner_insert BEFORE INSERT ON presentation_preferences BEGIN
 SELECT CASE WHEN NOT ((NEW.entity_kind='player' AND EXISTS(SELECT 1 FROM players WHERE id=NEW.entity_id AND id=NEW.player_id)) OR (NEW.entity_kind='quest' AND EXISTS(SELECT 1 FROM quests WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR (NEW.entity_kind='skill_tree' AND EXISTS(SELECT 1 FROM skill_trees WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR (NEW.entity_kind='skill' AND EXISTS(SELECT 1 FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE s.id=NEW.entity_id AND t.player_id=NEW.player_id)) OR (NEW.entity_kind='concept' AND EXISTS(SELECT 1 FROM concepts WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR (NEW.entity_kind='quest_stage' AND EXISTS(SELECT 1 FROM quest_stages WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR (NEW.entity_kind='quest_branch' AND EXISTS(SELECT 1 FROM quest_branches WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR (NEW.entity_kind='quest_session' AND EXISTS(SELECT 1 FROM quest_sessions WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR (NEW.entity_kind='narrative_entry' AND EXISTS(SELECT 1 FROM narrative_entries WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR (NEW.entity_kind='effect' AND EXISTS(SELECT 1 FROM effects WHERE id=NEW.entity_id AND player_id=NEW.player_id))) THEN RAISE(ABORT,'presentation target missing or outside Player world') END;
END;

-- Add data-defined intentional Content types without replacing existing Notes, Story, etc.
INSERT INTO type_definitions(namespace,code,label,description,sort_order,is_system) VALUES
 ('narrative_entry','guide','Guide','Reusable authored guidance or reference material.',80,1),
 ('narrative_entry','guidance','Guidance','A player-authored recommendation for an activity.',90,1),
 ('narrative_entry','lore','Lore','World-building content distinct from comments.',100,1),
 ('narrative_entry','instruction','Instruction','Reusable task instructions.',110,1),
 ('narrative_entry','reading','Reading','Material to read during a Quest or Session.',120,1),
 ('narrative_entry','todo','To Do','Authored checklist or next-action content.',130,1),
 ('narrative_entry','reference','Reference','A reusable reference entry.',140,1),
 ('narrative_entry','summary','Summary','A concise authored summary.',150,1)
ON CONFLICT(namespace,code) DO NOTHING;

-- New activity records join the same bounded FTS index. Lifecycle and visibility
-- are projected at query time so hidden/archived items are still recoverable.
CREATE TRIGGER search_quest_stages_insert AFTER INSERT ON quest_stages BEGIN
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('quest_stage',NEW.id,NEW.player_id,'stage',NEW.status,NEW.is_active,NEW.updated_at,NEW.title,trim(COALESCE(NEW.description,'')||' '||COALESCE(NEW.story,'')||' '||COALESCE(NEW.instructions,'')));
END;
CREATE TRIGGER search_quest_stages_update AFTER UPDATE ON quest_stages BEGIN
 DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='quest_stage' AND entity_id=OLD.id);
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('quest_stage',NEW.id,NEW.player_id,'stage',NEW.status,NEW.is_active,NEW.updated_at,NEW.title,trim(COALESCE(NEW.description,'')||' '||COALESCE(NEW.story,'')||' '||COALESCE(NEW.instructions,'')));
END;
CREATE TRIGGER search_quest_stages_delete AFTER DELETE ON quest_stages BEGIN DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='quest_stage' AND entity_id=OLD.id); END;
CREATE TRIGGER search_quest_branches_insert AFTER INSERT ON quest_branches BEGIN
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('quest_branch',NEW.id,NEW.player_id,'branch',NEW.status,NEW.is_active,NEW.updated_at,NEW.title,COALESCE(NEW.description,''));
END;
CREATE TRIGGER search_quest_branches_update AFTER UPDATE ON quest_branches BEGIN
 DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='quest_branch' AND entity_id=OLD.id);
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('quest_branch',NEW.id,NEW.player_id,'branch',NEW.status,NEW.is_active,NEW.updated_at,NEW.title,COALESCE(NEW.description,''));
END;
CREATE TRIGGER search_quest_branches_delete AFTER DELETE ON quest_branches BEGIN DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='quest_branch' AND entity_id=OLD.id); END;
CREATE TRIGGER search_quest_sessions_insert AFTER INSERT ON quest_sessions BEGIN
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('quest_session',NEW.id,NEW.player_id,'session',NEW.status,NEW.is_active,NEW.started_at,'Quest Session',trim(COALESCE(NEW.notes,'')||' '||COALESCE(NEW.result,'')));
END;
CREATE TRIGGER search_quest_sessions_update AFTER UPDATE ON quest_sessions BEGIN
 DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='quest_session' AND entity_id=OLD.id);
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('quest_session',NEW.id,NEW.player_id,'session',NEW.status,NEW.is_active,NEW.started_at,'Quest Session',trim(COALESCE(NEW.notes,'')||' '||COALESCE(NEW.result,'')));
END;
CREATE TRIGGER search_quest_sessions_delete AFTER DELETE ON quest_sessions BEGIN DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='quest_session' AND entity_id=OLD.id); END;
CREATE TRIGGER search_narratives_update_phase36 AFTER UPDATE ON narrative_entries BEGIN
 DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='narrative_entry' AND entity_id=OLD.id);
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('narrative_entry',NEW.id,NEW.player_id,NEW.kind_code,NULL,NEW.is_active,NEW.updated_at,NEW.title,NEW.content);
END;
CREATE TRIGGER search_narratives_insert_phase36 AFTER INSERT ON narrative_entries BEGIN
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('narrative_entry',NEW.id,NEW.player_id,NEW.kind_code,NULL,NEW.is_active,NEW.updated_at,NEW.title,NEW.content);
END;
-- Replace old NarrativeEntry FTS triggers to avoid duplicate FTS rows.
DROP TRIGGER search_narratives_insert;
DROP TRIGGER search_narratives_update;
DROP TRIGGER search_narratives_delete;
CREATE TRIGGER search_narratives_delete_phase36 AFTER DELETE ON narrative_entries BEGIN DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='narrative_entry' AND entity_id=OLD.id); END;
UPDATE world_search_fts SET active=(SELECT is_active FROM narrative_entries n WHERE n.id=world_search_fts.entity_id),occurred_at=(SELECT updated_at FROM narrative_entries n WHERE n.id=world_search_fts.entity_id) WHERE kind='narrative_entry';

-- Capture the previous authored representation before every mutable-row update.
-- IDs are opaque random tokens; the entity/revision number remains indexed data.
CREATE TRIGGER revision_players_before_update BEFORE UPDATE ON players BEGIN
 INSERT INTO entity_revisions(id,player_id,target_kind,target_id,revision_number,recorded_at,author_player_id,snapshot_json)
 VALUES(lower(hex(randomblob(16))),OLD.id,'player',OLD.id,(SELECT COALESCE(MAX(revision_number),0)+1 FROM entity_revisions WHERE target_kind='player' AND target_id=OLD.id),strftime('%Y-%m-%dT%H:%M:%fZ','now'),OLD.id,json_object('id',OLD.id,'name',OLD.name,'description',OLD.description,'level',OLD.level,'level_name',OLD.level_name,'progression_label',OLD.progression_label,'current_xp',OLD.current_xp,'is_active',OLD.is_active,'metadata_json',OLD.metadata_json,'created_at',OLD.created_at,'updated_at',OLD.updated_at));
END;
CREATE TRIGGER revision_quests_before_update BEFORE UPDATE ON quests BEGIN
 INSERT INTO entity_revisions(id,player_id,target_kind,target_id,revision_number,recorded_at,author_player_id,snapshot_json)
 VALUES(lower(hex(randomblob(16))),OLD.player_id,'quest',OLD.id,(SELECT COALESCE(MAX(revision_number),0)+1 FROM entity_revisions WHERE target_kind='quest' AND target_id=OLD.id),strftime('%Y-%m-%dT%H:%M:%fZ','now'),OLD.player_id,json_object('id',OLD.id,'player_id',OLD.player_id,'parent_quest_id',OLD.parent_quest_id,'skill_id',OLD.skill_id,'quest_type_code',OLD.quest_type_code,'title',OLD.title,'description',OLD.description,'story',OLD.story,'instructions',OLD.instructions,'status',OLD.status,'difficulty',OLD.difficulty,'progress',OLD.progress,'xp_reward',OLD.xp_reward,'due_at',OLD.due_at,'started_at',OLD.started_at,'completed_at',OLD.completed_at,'metadata_json',OLD.metadata_json,'created_at',OLD.created_at,'updated_at',OLD.updated_at));
END;
CREATE TRIGGER revision_skills_before_update BEFORE UPDATE ON skills BEGIN
 INSERT INTO entity_revisions(id,player_id,target_kind,target_id,revision_number,recorded_at,author_player_id,snapshot_json)
 SELECT lower(hex(randomblob(16))),t.player_id,'skill',OLD.id,(SELECT COALESCE(MAX(revision_number),0)+1 FROM entity_revisions WHERE target_kind='skill' AND target_id=OLD.id),strftime('%Y-%m-%dT%H:%M:%fZ','now'),t.player_id,json_object('id',OLD.id,'skill_tree_id',OLD.skill_tree_id,'parent_skill_id',OLD.parent_skill_id,'skill_type_code',OLD.skill_type_code,'name',OLD.name,'description',OLD.description,'story',OLD.story,'instructions',OLD.instructions,'level',OLD.level,'level_name',OLD.level_name,'progression_label',OLD.progression_label,'current_xp',OLD.current_xp,'invested_minutes',OLD.invested_minutes,'status',OLD.status,'started_at',OLD.started_at,'completed_at',OLD.completed_at,'metadata_json',OLD.metadata_json,'created_at',OLD.created_at,'updated_at',OLD.updated_at) FROM skill_trees t WHERE t.id=OLD.skill_tree_id;
END;
CREATE TRIGGER revision_trees_before_update BEFORE UPDATE ON skill_trees BEGIN
 INSERT INTO entity_revisions(id,player_id,target_kind,target_id,revision_number,recorded_at,author_player_id,snapshot_json)
 VALUES(lower(hex(randomblob(16))),OLD.player_id,'skill_tree',OLD.id,(SELECT COALESCE(MAX(revision_number),0)+1 FROM entity_revisions WHERE target_kind='skill_tree' AND target_id=OLD.id),strftime('%Y-%m-%dT%H:%M:%fZ','now'),OLD.player_id,json_object('id',OLD.id,'player_id',OLD.player_id,'name',OLD.name,'description',OLD.description,'story',OLD.story,'instructions',OLD.instructions,'is_active',OLD.is_active,'metadata_json',OLD.metadata_json,'created_at',OLD.created_at,'updated_at',OLD.updated_at));
END;
CREATE TRIGGER revision_concepts_before_update BEFORE UPDATE ON concepts BEGIN
 INSERT INTO entity_revisions(id,player_id,target_kind,target_id,revision_number,recorded_at,author_player_id,snapshot_json)
 VALUES(lower(hex(randomblob(16))),OLD.player_id,'concept',OLD.id,(SELECT COALESCE(MAX(revision_number),0)+1 FROM entity_revisions WHERE target_kind='concept' AND target_id=OLD.id),strftime('%Y-%m-%dT%H:%M:%fZ','now'),OLD.player_id,json_object('id',OLD.id,'player_id',OLD.player_id,'concept_type_code',OLD.concept_type_code,'name',OLD.name,'description',OLD.description,'is_active',OLD.is_active,'metadata_json',OLD.metadata_json,'created_at',OLD.created_at,'updated_at',OLD.updated_at));
END;
CREATE TRIGGER revision_stages_before_update BEFORE UPDATE ON quest_stages BEGIN
 INSERT INTO entity_revisions(id,player_id,target_kind,target_id,revision_number,recorded_at,author_player_id,snapshot_json)
 VALUES(lower(hex(randomblob(16))),OLD.player_id,'quest_stage',OLD.id,(SELECT COALESCE(MAX(revision_number),0)+1 FROM entity_revisions WHERE target_kind='quest_stage' AND target_id=OLD.id),strftime('%Y-%m-%dT%H:%M:%fZ','now'),OLD.player_id,json_object('id',OLD.id,'player_id',OLD.player_id,'quest_id',OLD.quest_id,'title',OLD.title,'description',OLD.description,'story',OLD.story,'instructions',OLD.instructions,'status',OLD.status,'sort_order',OLD.sort_order,'is_active',OLD.is_active,'metadata_json',OLD.metadata_json,'created_at',OLD.created_at,'updated_at',OLD.updated_at));
END;
CREATE TRIGGER revision_branches_before_update BEFORE UPDATE ON quest_branches BEGIN
 INSERT INTO entity_revisions(id,player_id,target_kind,target_id,revision_number,recorded_at,author_player_id,snapshot_json)
 VALUES(lower(hex(randomblob(16))),OLD.player_id,'quest_branch',OLD.id,(SELECT COALESCE(MAX(revision_number),0)+1 FROM entity_revisions WHERE target_kind='quest_branch' AND target_id=OLD.id),strftime('%Y-%m-%dT%H:%M:%fZ','now'),OLD.player_id,json_object('id',OLD.id,'player_id',OLD.player_id,'quest_id',OLD.quest_id,'stage_id',OLD.stage_id,'title',OLD.title,'description',OLD.description,'status',OLD.status,'sort_order',OLD.sort_order,'is_active',OLD.is_active,'metadata_json',OLD.metadata_json,'created_at',OLD.created_at,'updated_at',OLD.updated_at));
END;
CREATE TRIGGER revision_sessions_before_update BEFORE UPDATE ON quest_sessions BEGIN
 INSERT INTO entity_revisions(id,player_id,target_kind,target_id,revision_number,recorded_at,author_player_id,snapshot_json)
 VALUES(lower(hex(randomblob(16))),OLD.player_id,'quest_session',OLD.id,(SELECT COALESCE(MAX(revision_number),0)+1 FROM entity_revisions WHERE target_kind='quest_session' AND target_id=OLD.id),strftime('%Y-%m-%dT%H:%M:%fZ','now'),OLD.player_id,json_object('id',OLD.id,'player_id',OLD.player_id,'quest_id',OLD.quest_id,'stage_id',OLD.stage_id,'branch_id',OLD.branch_id,'skill_id',OLD.skill_id,'concept_id',OLD.concept_id,'started_at',OLD.started_at,'ended_at',OLD.ended_at,'status',OLD.status,'progress_before',OLD.progress_before,'progress_after',OLD.progress_after,'result',OLD.result,'notes',OLD.notes,'is_active',OLD.is_active,'metadata_json',OLD.metadata_json,'created_at',OLD.created_at,'updated_at',OLD.updated_at));
END;
CREATE TRIGGER revision_narrative_before_update BEFORE UPDATE ON narrative_entries BEGIN
 INSERT INTO entity_revisions(id,player_id,target_kind,target_id,revision_number,recorded_at,author_player_id,snapshot_json)
 VALUES(lower(hex(randomblob(16))),OLD.player_id,'narrative_entry',OLD.id,(SELECT COALESCE(MAX(revision_number),0)+1 FROM entity_revisions WHERE target_kind='narrative_entry' AND target_id=OLD.id),strftime('%Y-%m-%dT%H:%M:%fZ','now'),OLD.player_id,json_object('id',OLD.id,'player_id',OLD.player_id,'kind_code',OLD.kind_code,'title',OLD.title,'content',OLD.content,'author',OLD.author,'source_kind',OLD.source_kind,'source_id',OLD.source_id,'is_active',OLD.is_active,'metadata_json',OLD.metadata_json,'created_at',OLD.created_at,'updated_at',OLD.updated_at));
END;

CREATE TRIGGER revision_concept_progress_before_update BEFORE UPDATE ON concept_progress_tracks BEGIN
 INSERT INTO entity_revisions(id,player_id,target_kind,target_id,revision_number,recorded_at,author_player_id,snapshot_json)
 SELECT lower(hex(randomblob(16))),c.player_id,'concept_progress',OLD.id,(SELECT COALESCE(MAX(revision_number),0)+1 FROM entity_revisions WHERE target_kind='concept_progress' AND target_id=OLD.id),strftime('%Y-%m-%dT%H:%M:%fZ','now'),c.player_id,json_object('id',OLD.id,'concept_id',OLD.concept_id,'track_code',OLD.track_code,'current_value',OLD.current_value,'level',OLD.level,'level_name',OLD.level_name,'progression_label',OLD.progression_label,'control',OLD.control,'is_active',OLD.is_active,'metadata_json',OLD.metadata_json,'created_at',OLD.created_at,'updated_at',OLD.updated_at) FROM concepts c WHERE c.id=OLD.concept_id;
END;

CREATE TABLE entity_revision_restores(id TEXT PRIMARY KEY,revision_id TEXT NOT NULL REFERENCES entity_revisions(id) ON DELETE RESTRICT,player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,restored_at TEXT NOT NULL,reason TEXT);
CREATE INDEX idx_revision_restores_revision ON entity_revision_restores(revision_id,restored_at);
CREATE TRIGGER entity_revision_restores_immutable_update BEFORE UPDATE ON entity_revision_restores BEGIN SELECT RAISE(ABORT,'revision restore audit is append-only'); END;
CREATE TRIGGER entity_revision_restores_immutable_delete BEFORE DELETE ON entity_revision_restores BEGIN SELECT RAISE(ABORT,'revision restore audit is append-only'); END;
