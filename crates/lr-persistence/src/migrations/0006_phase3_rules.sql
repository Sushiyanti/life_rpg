-- 0006_phase3_rules
-- Declarative, versioned data interpreted only by trusted application code.
CREATE TABLE rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 160),
    description TEXT,
    is_enabled INTEGER NOT NULL DEFAULT 1 CHECK (is_enabled IN (0,1)),
    priority INTEGER NOT NULL DEFAULT 0,
    trigger_kind TEXT NOT NULL CHECK (trigger_kind IN ('quest_completed','player_xp_changed','stat_changed')),
    schema_version INTEGER NOT NULL CHECK (schema_version = 1),
    definition_json TEXT NOT NULL CHECK (json_valid(definition_json) AND json_extract(definition_json,'$.schemaVersion') = schema_version AND json_extract(definition_json,'$.trigger') = trigger_kind),
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(metadata_json)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_rules_dispatch ON rules(is_enabled,trigger_kind,priority DESC,id ASC);

CREATE TABLE rule_execution_history (
    id TEXT PRIMARY KEY,
    chain_id TEXT NOT NULL,
    rule_id TEXT NOT NULL REFERENCES rules(id) ON DELETE RESTRICT,
    event_kind TEXT NOT NULL CHECK (event_kind IN ('quest_completed','player_xp_changed','stat_changed')),
    event_json TEXT NOT NULL CHECK (json_valid(event_json)),
    condition_passed INTEGER CHECK (condition_passed IS NULL OR condition_passed IN (0,1)),
    actions_json TEXT NOT NULL CHECK (json_valid(actions_json)),
    status TEXT NOT NULL CHECK (status IN ('condition_failed','succeeded','failed','guard_aborted')),
    error TEXT,
    depth INTEGER NOT NULL CHECK (depth BETWEEN 0 AND 9),
    executed_at TEXT NOT NULL
);
CREATE INDEX idx_rule_history_chain ON rule_execution_history(chain_id,depth,id);
CREATE INDEX idx_rule_history_rule ON rule_execution_history(rule_id,executed_at DESC);
CREATE TRIGGER rule_history_immutable_update BEFORE UPDATE ON rule_execution_history
BEGIN SELECT RAISE(ABORT,'rule execution history is append-only'); END;
CREATE TRIGGER rule_history_immutable_delete BEFORE DELETE ON rule_execution_history
BEGIN SELECT RAISE(ABORT,'rule execution history is append-only'); END;
