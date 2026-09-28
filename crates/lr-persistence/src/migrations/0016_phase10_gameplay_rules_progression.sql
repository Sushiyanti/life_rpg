-- 0016_phase10_gameplay_rules_progression
-- Add availability without overloading Skill lifecycle. Existing Skills remain available and Player-controlled.
ALTER TABLE skills ADD COLUMN availability TEXT NOT NULL DEFAULT 'available'
 CHECK(availability IN ('locked','available'));
ALTER TABLE skills ADD COLUMN availability_control TEXT NOT NULL DEFAULT 'manual'
 CHECK(availability_control IN ('manual','rule_controlled'));

-- Rule-caused deactivation is distinct from Player deactivation and derived expiry.
ALTER TABLE effects ADD COLUMN deactivation_source TEXT
 CHECK(deactivation_source IS NULL OR deactivation_source IN ('manual','rule'));
UPDATE effects SET deactivation_source='manual' WHERE deactivated_at IS NOT NULL;
CREATE TRIGGER effects_deactivation_source_insert BEFORE INSERT ON effects
WHEN (NEW.deactivated_at IS NULL AND NEW.deactivation_source IS NOT NULL)
  OR (NEW.deactivated_at IS NOT NULL AND NEW.deactivation_source IS NULL)
BEGIN SELECT RAISE(ABORT,'Effect deactivation timestamp and source must agree'); END;
CREATE TRIGGER effects_deactivation_source_update BEFORE UPDATE OF deactivated_at,deactivation_source ON effects
WHEN (NEW.deactivated_at IS NULL AND NEW.deactivation_source IS NOT NULL)
  OR (NEW.deactivated_at IS NOT NULL AND NEW.deactivation_source IS NULL)
BEGIN SELECT RAISE(ABORT,'Effect deactivation timestamp and source must agree'); END;

-- This history is narrowly scoped to Skill availability/policy. Skill XP remains
-- represented by the existing canonical Transaction ledger.
CREATE TABLE skill_history (
 id TEXT PRIMARY KEY,
 player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
 skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
 event_kind TEXT NOT NULL CHECK(event_kind IN ('availability_changed','control_changed')),
 source TEXT NOT NULL CHECK(source IN ('manual','rule')),
 recorded_at TEXT NOT NULL,
 previous_state_json TEXT CHECK(previous_state_json IS NULL OR json_valid(previous_state_json)),
 current_state_json TEXT NOT NULL CHECK(json_valid(current_state_json))
);
CREATE INDEX idx_skill_history_skill_time ON skill_history(player_id,skill_id,recorded_at,id);
CREATE TRIGGER skill_history_owner_insert BEFORE INSERT ON skill_history BEGIN
 SELECT CASE WHEN NOT EXISTS(
  SELECT 1 FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id
  WHERE s.id=NEW.skill_id AND t.player_id=NEW.player_id
 ) THEN RAISE(ABORT,'Skill history must belong to the Skill Player') END;
END;
CREATE TRIGGER skill_history_immutable_update BEFORE UPDATE ON skill_history BEGIN
 SELECT RAISE(ABORT,'Skill history is append-only'); END;
CREATE TRIGGER skill_history_immutable_delete BEFORE DELETE ON skill_history BEGIN
 SELECT RAISE(ABORT,'Skill history is append-only'); END;

-- Extend the Effect history allow-list without rewriting or dropping old facts.
CREATE TABLE effect_history_phase10 (
 id TEXT PRIMARY KEY,
 player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
 effect_id TEXT NOT NULL REFERENCES effects(id) ON DELETE CASCADE,
 session_id TEXT REFERENCES quest_sessions(id) ON DELETE SET NULL,
 event_kind TEXT NOT NULL CHECK(event_kind IN ('created','details_changed','expiry_changed','manually_deactivated','rule_deactivated','session_linked','session_unlinked')),
 recorded_at TEXT NOT NULL,
 previous_state_json TEXT CHECK(previous_state_json IS NULL OR json_valid(previous_state_json)),
 current_state_json TEXT NOT NULL CHECK(json_valid(current_state_json))
);
INSERT INTO effect_history_phase10
 SELECT id,player_id,effect_id,session_id,event_kind,recorded_at,previous_state_json,current_state_json FROM effect_history;
DROP TABLE effect_history;
ALTER TABLE effect_history_phase10 RENAME TO effect_history;
CREATE INDEX idx_effect_history_effect_time ON effect_history(player_id,effect_id,recorded_at,id);
CREATE INDEX idx_effect_history_session_time ON effect_history(player_id,session_id,recorded_at,id);
CREATE INDEX idx_timeline_effect_history_player_time ON effect_history(player_id,recorded_at DESC,id DESC);
CREATE TRIGGER effect_history_immutable_update BEFORE UPDATE ON effect_history BEGIN
 SELECT RAISE(ABORT,'effect history is append-only'); END;
CREATE TRIGGER effect_history_immutable_delete BEFORE DELETE ON effect_history BEGIN
 SELECT RAISE(ABORT,'effect history is append-only'); END;
CREATE TRIGGER effect_history_owner_insert BEFORE INSERT ON effect_history BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM effects e WHERE e.id=NEW.effect_id AND e.player_id=NEW.player_id)
  THEN RAISE(ABORT,'Effect history must belong to the Effect Player') END;
 SELECT CASE WHEN NEW.session_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM quest_sessions s WHERE s.id=NEW.session_id AND s.player_id=NEW.player_id)
  THEN RAISE(ABORT,'Effect history Session must belong to the same Player') END;
END;

-- Preserve all v1 rule definitions and audit rows while extending only the closed trigger allow-list.
CREATE TABLE rules_phase10 (
 id TEXT PRIMARY KEY,
 name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 160),
 description TEXT,
 is_enabled INTEGER NOT NULL DEFAULT 1 CHECK(is_enabled IN (0,1)),
 priority INTEGER NOT NULL DEFAULT 0,
 trigger_kind TEXT NOT NULL CHECK(trigger_kind IN ('quest_completed','player_xp_changed','stat_changed','concept_progress_changed','skill_xp_changed','skill_unlocked','session_started','session_finished','effect_created','effect_deactivated')),
 schema_version INTEGER NOT NULL CHECK(schema_version=1),
 definition_json TEXT NOT NULL CHECK(json_valid(definition_json) AND json_extract(definition_json,'$.schemaVersion')=schema_version AND json_extract(definition_json,'$.trigger')=trigger_kind),
 metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
INSERT INTO rules_phase10 SELECT id,name,description,is_enabled,priority,trigger_kind,schema_version,definition_json,metadata_json,created_at,updated_at FROM rules;
CREATE TABLE rule_execution_history_phase10 (
 id TEXT PRIMARY KEY,
 chain_id TEXT NOT NULL,
 rule_id TEXT NOT NULL REFERENCES rules_phase10(id) ON DELETE RESTRICT,
 event_kind TEXT NOT NULL CHECK(event_kind IN ('quest_completed','player_xp_changed','stat_changed','concept_progress_changed','skill_xp_changed','skill_unlocked','session_started','session_finished','effect_created','effect_deactivated')),
 event_json TEXT NOT NULL CHECK(json_valid(event_json)),
 condition_passed INTEGER CHECK(condition_passed IS NULL OR condition_passed IN (0,1)),
 actions_json TEXT NOT NULL CHECK(json_valid(actions_json)),
 status TEXT NOT NULL CHECK(status IN ('condition_failed','succeeded','failed','guard_aborted')),
 error TEXT,
 depth INTEGER NOT NULL CHECK(depth BETWEEN 0 AND 9),
 executed_at TEXT NOT NULL
);
INSERT INTO rule_execution_history_phase10
 SELECT id,chain_id,rule_id,event_kind,event_json,condition_passed,actions_json,status,error,depth,executed_at FROM rule_execution_history;
DROP TABLE rule_execution_history;
DROP TABLE rules;
ALTER TABLE rules_phase10 RENAME TO rules;
ALTER TABLE rule_execution_history_phase10 RENAME TO rule_execution_history;
CREATE INDEX idx_rules_dispatch ON rules(is_enabled,trigger_kind,priority DESC,id ASC);
CREATE INDEX idx_rule_history_chain ON rule_execution_history(chain_id,depth,id);
CREATE INDEX idx_rule_history_rule ON rule_execution_history(rule_id,executed_at DESC);
CREATE TRIGGER rule_history_immutable_update BEFORE UPDATE ON rule_execution_history BEGIN
 SELECT RAISE(ABORT,'rule execution history is append-only'); END;
CREATE TRIGGER rule_history_immutable_delete BEFORE DELETE ON rule_execution_history BEGIN
 SELECT RAISE(ABORT,'rule execution history is append-only'); END;

-- The shared immutable Transaction ledger now records Skill XP applied deltas as
-- well as Player XP. Rebuild the legacy XP-only CHECK without losing capture
-- timestamps, sequence IDs, indexes, search rows, concept links, or triggers.
DROP TRIGGER xp_transactions_require_applied_amount;
DROP TRIGGER transactions_immutable_update;
DROP TRIGGER transactions_immutable_delete;
DROP TRIGGER search_transactions_insert;
DROP TRIGGER search_transactions_delete;
DROP TRIGGER concept_links_transaction_delete;
DROP TRIGGER concept_link_owner_insert;
DROP TRIGGER concept_association_owner_insert;
DROP TRIGGER concept_association_owner_update;
CREATE TABLE transactions_phase10 (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
 transaction_type_namespace TEXT NOT NULL DEFAULT 'transaction' CHECK(transaction_type_namespace='transaction'),
 transaction_type_code TEXT NOT NULL,
 resource TEXT NOT NULL CHECK(length(trim(resource))>0),
 amount INTEGER NOT NULL CHECK(amount<>0),
 applied_amount INTEGER CHECK(applied_amount IS NULL OR resource IN ('xp','skill_xp')),
 occurred_at TEXT NOT NULL,
 captured_at TEXT CHECK(captured_at IS NULL OR captured_at>=occurred_at),
 reason TEXT,
 description TEXT,
 source_kind TEXT,
 source_id TEXT,
 metadata_json TEXT NOT NULL DEFAULT '{}',
 FOREIGN KEY(transaction_type_namespace,transaction_type_code) REFERENCES type_definitions(namespace,code)
);
INSERT INTO transactions_phase10(id,player_id,transaction_type_namespace,transaction_type_code,resource,amount,applied_amount,occurred_at,captured_at,reason,description,source_kind,source_id,metadata_json)
 SELECT id,player_id,transaction_type_namespace,transaction_type_code,resource,amount,applied_amount,occurred_at,captured_at,reason,description,source_kind,source_id,metadata_json FROM transactions;
DROP TABLE transactions;
ALTER TABLE transactions_phase10 RENAME TO transactions;
CREATE INDEX idx_transactions_player_occurred ON transactions(player_id,occurred_at DESC,id DESC);
CREATE INDEX idx_transactions_player_resource ON transactions(player_id,resource);
CREATE INDEX idx_transactions_occurred_captured ON transactions(occurred_at,captured_at);
CREATE TRIGGER xp_transactions_require_applied_amount BEFORE INSERT ON transactions
WHEN NEW.resource IN ('xp','skill_xp') AND (
 NEW.applied_amount IS NULL
 OR (NEW.amount>0 AND NEW.applied_amount<>NEW.amount)
 OR (NEW.amount<0 AND (NEW.applied_amount>0 OR NEW.applied_amount<NEW.amount))
)
BEGIN SELECT RAISE(ABORT,'XP transaction must record a valid applied amount'); END;
CREATE TRIGGER transactions_immutable_update BEFORE UPDATE ON transactions
BEGIN SELECT RAISE(ABORT,'transactions are append-only'); END;
CREATE TRIGGER transactions_immutable_delete BEFORE DELETE ON transactions
BEGIN SELECT RAISE(ABORT,'transactions are append-only'); END;
CREATE TRIGGER search_transactions_insert AFTER INSERT ON transactions BEGIN
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body)
 VALUES('transaction',CAST(NEW.id AS TEXT),NEW.player_id,NEW.transaction_type_code,NEW.reason,1,NEW.occurred_at,NEW.resource,trim(COALESCE(NEW.description,'')||' '||COALESCE(NEW.reason,'')));
END;
CREATE TRIGGER search_transactions_delete AFTER DELETE ON transactions BEGIN
 DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='transaction' AND entity_id=CAST(OLD.id AS TEXT));
END;
CREATE TRIGGER concept_links_transaction_delete AFTER DELETE ON transactions BEGIN
 DELETE FROM concept_entity_links WHERE entity_kind='transaction' AND entity_id=CAST(OLD.id AS TEXT);
END;
CREATE TRIGGER concept_link_owner_insert BEFORE INSERT ON concept_entity_links BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM concepts c WHERE c.id=NEW.concept_id AND c.player_id=NEW.player_id) THEN RAISE(ABORT,'linked Concept must belong to link Player') END;
 SELECT CASE WHEN NOT (
  (NEW.entity_kind='quest' AND EXISTS(SELECT 1 FROM quests WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
  (NEW.entity_kind='skill' AND EXISTS(SELECT 1 FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id WHERE s.id=NEW.entity_id AND t.player_id=NEW.player_id)) OR
  (NEW.entity_kind='skill_tree' AND EXISTS(SELECT 1 FROM skill_trees WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
  (NEW.entity_kind='effect' AND EXISTS(SELECT 1 FROM effects WHERE id=NEW.entity_id AND player_id=NEW.player_id)) OR
  (NEW.entity_kind='transaction' AND EXISTS(SELECT 1 FROM transactions WHERE CAST(id AS TEXT)=NEW.entity_id AND player_id=NEW.player_id)) OR
  (NEW.entity_kind='comment' AND EXISTS(SELECT 1 FROM comments WHERE CAST(id AS TEXT)=NEW.entity_id AND author_player_id=NEW.player_id)) OR
  (NEW.entity_kind='narrative_entry' AND EXISTS(SELECT 1 FROM narrative_entries WHERE id=NEW.entity_id AND player_id=NEW.player_id))
 ) THEN RAISE(ABORT,'Concept link target is missing or outside Player world') END;
END;
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
