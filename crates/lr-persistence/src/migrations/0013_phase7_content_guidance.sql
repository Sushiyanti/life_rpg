-- 0013_phase7_content_guidance
-- NarrativeEntry remains the single intentional Content record. This migration
-- evolves only its bounded relationship facts; it creates no parallel content
-- system and never derives content from any other world record.

-- The pre-Phase-7 registry already covers the other required kinds. `introduction`
-- is additive; existing `intro` content-role rows remain valid compatibility data.
INSERT INTO type_definitions(namespace,code,label,description,sort_order,is_system) VALUES
 ('narrative_entry','introduction','Introduction','A player-authored introduction to reusable content.',75,1)
ON CONFLICT(namespace,code) DO NOTHING;

-- Roles are data, not a compiled enum. Existing rows retain their code and label.
INSERT INTO content_attachment_roles(code,label,created_at,updated_at) VALUES
 ('introduction','Introduction',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('about','About',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('context','Context',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('instruction','Instruction',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('result','Result',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('reference','Reference',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))
ON CONFLICT(code) DO NOTHING;

-- The old composite key could only represent a current attachment. Preserve every
-- legacy row exactly, add a relationship id, and map legacy inactive rows to the
-- only prior removal observation (`updated_at`) rather than inventing a new time.
DROP TRIGGER content_attachment_owner_insert;
DROP INDEX idx_content_attachments_target;
ALTER TABLE content_attachments RENAME TO content_attachments_legacy;

CREATE TABLE content_attachments (
 id TEXT PRIMARY KEY,
 content_id TEXT NOT NULL REFERENCES narrative_entries(id) ON DELETE RESTRICT,
 player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
 target_kind TEXT NOT NULL CHECK(target_kind IN ('player','quest','quest_stage','quest_branch','quest_session','skill','skill_tree','concept','effect')),
 target_id TEXT NOT NULL,
 role_code TEXT NOT NULL REFERENCES content_attachment_roles(code) ON DELETE RESTRICT,
 sort_order INTEGER NOT NULL DEFAULT 0,
 created_at TEXT NOT NULL,
 removed_at TEXT,
 updated_at TEXT NOT NULL,
 CHECK(removed_at IS NULL OR removed_at >= created_at)
);

INSERT INTO content_attachments(id,content_id,player_id,target_kind,target_id,role_code,sort_order,created_at,removed_at,updated_at)
 SELECT 'content-rel-' || lower(hex(randomblob(16))),content_id,player_id,target_kind,target_id,role_code,0,created_at,
        CASE WHEN is_active=0 THEN updated_at ELSE NULL END,updated_at
 FROM content_attachments_legacy;
DROP TABLE content_attachments_legacy;

-- Only one active fact with the same content/target/role is allowed. A later
-- reattachment creates a new fact instead of erasing an earlier removal.
CREATE UNIQUE INDEX idx_content_attachments_active_unique
 ON content_attachments(content_id,target_kind,target_id,role_code) WHERE removed_at IS NULL;
CREATE INDEX idx_content_attachments_target
 ON content_attachments(target_kind,target_id,removed_at,sort_order,created_at,id);
CREATE INDEX idx_content_attachments_content
 ON content_attachments(content_id,removed_at,target_kind,target_id,role_code);

CREATE TRIGGER content_attachment_owner_insert BEFORE INSERT ON content_attachments BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM narrative_entries n WHERE n.id=NEW.content_id AND n.player_id=NEW.player_id) THEN RAISE(ABORT,'Content must belong to Player') END;
 SELECT CASE WHEN NOT (
   (NEW.target_kind='player' AND EXISTS(SELECT 1 FROM players WHERE id=NEW.target_id AND id=NEW.player_id)) OR
   (NEW.target_kind='quest' AND EXISTS(SELECT 1 FROM quests WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
   (NEW.target_kind='quest_stage' AND EXISTS(SELECT 1 FROM quest_stages WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
   (NEW.target_kind='quest_branch' AND EXISTS(SELECT 1 FROM quest_branches WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
   (NEW.target_kind='quest_session' AND EXISTS(SELECT 1 FROM quest_sessions WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
   (NEW.target_kind='skill' AND EXISTS(SELECT 1 FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE s.id=NEW.target_id AND t.player_id=NEW.player_id)) OR
   (NEW.target_kind='skill_tree' AND EXISTS(SELECT 1 FROM skill_trees WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
   (NEW.target_kind='concept' AND EXISTS(SELECT 1 FROM concepts WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
   (NEW.target_kind='effect' AND EXISTS(SELECT 1 FROM effects WHERE id=NEW.target_id AND player_id=NEW.player_id))
 ) THEN RAISE(ABORT,'Content target missing or outside Player world') END;
END;
CREATE TRIGGER content_attachment_owner_update BEFORE UPDATE OF content_id,player_id,target_kind,target_id ON content_attachments BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM narrative_entries n WHERE n.id=NEW.content_id AND n.player_id=NEW.player_id) THEN RAISE(ABORT,'Content must belong to Player') END;
 SELECT CASE WHEN NOT (
   (NEW.target_kind='player' AND EXISTS(SELECT 1 FROM players WHERE id=NEW.target_id AND id=NEW.player_id)) OR
   (NEW.target_kind='quest' AND EXISTS(SELECT 1 FROM quests WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
   (NEW.target_kind='quest_stage' AND EXISTS(SELECT 1 FROM quest_stages WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
   (NEW.target_kind='quest_branch' AND EXISTS(SELECT 1 FROM quest_branches WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
   (NEW.target_kind='quest_session' AND EXISTS(SELECT 1 FROM quest_sessions WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
   (NEW.target_kind='skill' AND EXISTS(SELECT 1 FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE s.id=NEW.target_id AND t.player_id=NEW.player_id)) OR
   (NEW.target_kind='skill_tree' AND EXISTS(SELECT 1 FROM skill_trees WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
   (NEW.target_kind='concept' AND EXISTS(SELECT 1 FROM concepts WHERE id=NEW.target_id AND player_id=NEW.player_id)) OR
   (NEW.target_kind='effect' AND EXISTS(SELECT 1 FROM effects WHERE id=NEW.target_id AND player_id=NEW.player_id))
 ) THEN RAISE(ABORT,'Content target missing or outside Player world') END;
END;

-- Polymorphic targets are not foreign keys. Physical deletion is not a normal
-- content workflow (lifecycle trash is); if it occurs, retain the relationship
-- fact but mark an active link removed instead of deleting historical evidence.
CREATE TRIGGER content_attachment_cleanup_quest AFTER DELETE ON quests BEGIN UPDATE content_attachments SET removed_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'),updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE target_kind='quest' AND target_id=OLD.id AND removed_at IS NULL; END;
CREATE TRIGGER content_attachment_cleanup_stage AFTER DELETE ON quest_stages BEGIN UPDATE content_attachments SET removed_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'),updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE target_kind='quest_stage' AND target_id=OLD.id AND removed_at IS NULL; END;
CREATE TRIGGER content_attachment_cleanup_branch AFTER DELETE ON quest_branches BEGIN UPDATE content_attachments SET removed_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'),updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE target_kind='quest_branch' AND target_id=OLD.id AND removed_at IS NULL; END;
CREATE TRIGGER content_attachment_cleanup_session AFTER DELETE ON quest_sessions BEGIN UPDATE content_attachments SET removed_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'),updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE target_kind='quest_session' AND target_id=OLD.id AND removed_at IS NULL; END;
CREATE TRIGGER content_attachment_cleanup_skill AFTER DELETE ON skills BEGIN UPDATE content_attachments SET removed_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'),updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE target_kind='skill' AND target_id=OLD.id AND removed_at IS NULL; END;
CREATE TRIGGER content_attachment_cleanup_tree AFTER DELETE ON skill_trees BEGIN UPDATE content_attachments SET removed_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'),updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE target_kind='skill_tree' AND target_id=OLD.id AND removed_at IS NULL; END;
CREATE TRIGGER content_attachment_cleanup_concept AFTER DELETE ON concepts BEGIN UPDATE content_attachments SET removed_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'),updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE target_kind='concept' AND target_id=OLD.id AND removed_at IS NULL; END;
CREATE TRIGGER content_attachment_cleanup_effect AFTER DELETE ON effects BEGIN UPDATE content_attachments SET removed_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'),updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE target_kind='effect' AND target_id=OLD.id AND removed_at IS NULL; END;
