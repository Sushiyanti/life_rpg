-- Phase 11: explicit Player-owned organizational Tags.
-- Tags are canonical labels; target kinds are a closed list, not a generic entity framework.

-- Reuse the existing lifecycle ledger while adding Tag as one supported lifecycle target.
ALTER TABLE entity_lifecycle RENAME TO entity_lifecycle_v16;
CREATE TABLE entity_lifecycle(
 target_kind TEXT NOT NULL CHECK(target_kind IN ('player','quest','skill_tree','skill','concept','concept_progress','quest_stage','quest_branch','quest_session','narrative_entry','tag')),
 target_id TEXT NOT NULL,
 player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
 state TEXT NOT NULL CHECK(state IN ('active','archived','trashed')),
 updated_at TEXT NOT NULL,
 PRIMARY KEY(target_kind,target_id)
);
INSERT INTO entity_lifecycle(target_kind,target_id,player_id,state,updated_at)
 SELECT target_kind,target_id,player_id,state,updated_at FROM entity_lifecycle_v16;
DROP TABLE entity_lifecycle_v16;

CREATE TABLE tags(
 id TEXT PRIMARY KEY,
 player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
 transfer_key TEXT NOT NULL UNIQUE CHECK(length(trim(transfer_key)) BETWEEN 1 AND 128),
 name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 80),
 normalized_name TEXT NOT NULL CHECK(length(trim(normalized_name)) BETWEEN 1 AND 160),
 description TEXT CHECK(description IS NULL OR length(description)<=4000),
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL,
 UNIQUE(id,player_id),
 -- Names remain unambiguous within a Player even while a Tag is archived/trashed.
 UNIQUE(player_id,normalized_name)
);
CREATE INDEX idx_tags_player_name ON tags(player_id,normalized_name,name);
CREATE INDEX idx_tags_player_transfer ON tags(player_id,transfer_key);

CREATE TABLE tag_relationships(
 id TEXT PRIMARY KEY,
 player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
 tag_id TEXT NOT NULL,
 target_kind TEXT NOT NULL CHECK(target_kind IN ('quest','quest_stage','quest_branch','quest_session','skill_tree','skill','concept','effect','narrative_entry','comment')),
 target_id TEXT NOT NULL CHECK(length(target_id) BETWEEN 1 AND 160),
 added_at TEXT NOT NULL,
 removed_at TEXT,
 CHECK(removed_at IS NULL OR removed_at>=added_at),
 FOREIGN KEY(tag_id,player_id) REFERENCES tags(id,player_id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX idx_tag_relationships_active_unique ON tag_relationships(player_id,tag_id,target_kind,target_id) WHERE removed_at IS NULL;
CREATE INDEX idx_tag_relationships_player_tag_removed ON tag_relationships(player_id,tag_id,removed_at,added_at,target_kind,target_id);
CREATE INDEX idx_tag_relationships_active_target ON tag_relationships(player_id,target_kind,target_id,tag_id) WHERE removed_at IS NULL;

-- Reinforce same-world ownership at the database boundary for every admitted target.
CREATE TRIGGER tag_relationships_target_owner_insert BEFORE INSERT ON tag_relationships BEGIN
 SELECT CASE WHEN NOT (
  (NEW.target_kind='quest' AND EXISTS(SELECT 1 FROM quests WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
  (NEW.target_kind='quest_stage' AND EXISTS(SELECT 1 FROM quest_stages WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
  (NEW.target_kind='quest_branch' AND EXISTS(SELECT 1 FROM quest_branches WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
  (NEW.target_kind='quest_session' AND EXISTS(SELECT 1 FROM quest_sessions WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
  (NEW.target_kind='skill_tree' AND EXISTS(SELECT 1 FROM skill_trees WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
  (NEW.target_kind='skill' AND EXISTS(SELECT 1 FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE s.id=NEW.target_id AND t.player_id=NEW.player_id)) OR
  (NEW.target_kind='concept' AND EXISTS(SELECT 1 FROM concepts WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
  (NEW.target_kind='effect' AND EXISTS(SELECT 1 FROM effects WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
  (NEW.target_kind='narrative_entry' AND EXISTS(SELECT 1 FROM narrative_entries WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
  (NEW.target_kind='comment' AND EXISTS(SELECT 1 FROM comments WHERE CAST(id AS TEXT)=NEW.target_id AND author_player_id=NEW.player_id))
 ) THEN RAISE(ABORT,'Tag target missing or outside Player world') END;
 SELECT CASE WHEN COALESCE((SELECT state FROM entity_lifecycle WHERE target_kind='tag' AND target_id=NEW.tag_id),'active')<>'active'
  THEN RAISE(ABORT,'only an active Tag may be assigned') END;
END;
CREATE TRIGGER tag_relationships_immutable_update BEFORE UPDATE ON tag_relationships
WHEN OLD.removed_at IS NOT NULL OR NEW.removed_at IS NULL OR
 NEW.id<>OLD.id OR NEW.player_id<>OLD.player_id OR NEW.tag_id<>OLD.tag_id OR
 NEW.target_kind<>OLD.target_kind OR NEW.target_id<>OLD.target_id OR NEW.added_at<>OLD.added_at
BEGIN SELECT RAISE(ABORT,'Tag relationship is immutable except for one removal timestamp'); END;
CREATE TRIGGER tag_relationships_immutable_delete BEFORE DELETE ON tag_relationships
BEGIN SELECT RAISE(ABORT,'Tag relationship history is retained'); END;
CREATE TRIGGER tags_transfer_key_immutable BEFORE UPDATE OF transfer_key ON tags
WHEN NEW.transfer_key<>OLD.transfer_key
BEGIN SELECT RAISE(ABORT,'Tag transfer references are immutable'); END;

-- Workspace filters store only local Tag references as a bounded JSON array;
-- portable transfers replace these with opaque stable descriptors.
ALTER TABLE workspace_panels ADD COLUMN filter_tag_ids_json TEXT NOT NULL DEFAULT '[]'
 CHECK(json_valid(filter_tag_ids_json) AND json_type(filter_tag_ids_json)='array' AND json_array_length(filter_tag_ids_json)<=20);
ALTER TABLE workspace_panels ADD COLUMN filter_tag_match TEXT NOT NULL DEFAULT 'any'
 CHECK(filter_tag_match IN ('any','all'));
