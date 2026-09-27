-- 0005_phase21_integrity
-- Additive hardening: structured player stats, richer XP history, explicit
-- effect deactivation, database-enforced hierarchy ownership, and XP floors.

CREATE TABLE player_stat_definitions (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE CHECK (length(code) BETWEEN 1 AND 160 AND code GLOB '[a-z]*' AND code NOT GLOB '*[^a-z0-9_]*'),
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 160),
    description TEXT,
    unit TEXT,
    minimum REAL,
    maximum REAL,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (minimum IS NULL OR minimum BETWEEN -1.7976931348623157e308 AND 1.7976931348623157e308),
    CHECK (maximum IS NULL OR maximum BETWEEN -1.7976931348623157e308 AND 1.7976931348623157e308),
    CHECK (minimum IS NULL OR maximum IS NULL OR minimum <= maximum)
);

CREATE TABLE player_stats (
    player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    stat_code TEXT NOT NULL REFERENCES player_stat_definitions(code) ON DELETE RESTRICT,
    current_value REAL NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    updated_at TEXT NOT NULL,
    CHECK (current_value BETWEEN -1.7976931348623157e308 AND 1.7976931348623157e308),
    PRIMARY KEY (player_id, stat_code)
);
CREATE INDEX idx_player_stats_stat ON player_stats (stat_code, player_id);

-- `amount` remains the requested historical event; applied_amount records the
-- actual XP delta after applying the zero floor. NULL for non-XP resources.
ALTER TABLE transactions ADD COLUMN applied_amount INTEGER
    CHECK (applied_amount IS NULL OR resource = 'xp');

UPDATE transactions SET applied_amount = amount WHERE resource = 'xp';
INSERT INTO transactions (player_id,transaction_type_namespace,transaction_type_code,resource,amount,applied_amount,occurred_at,reason,description)
SELECT id,'transaction','xp','xp',-current_xp,-current_xp,updated_at,'phase21_xp_floor_migration','Correction that floors a legacy negative XP cache at zero.'
FROM players WHERE current_xp < 0;
UPDATE players SET current_xp=0, level=1 WHERE current_xp < 0;

ALTER TABLE effects ADD COLUMN deactivated_at TEXT;
CREATE INDEX idx_effects_player_lifecycle ON effects (player_id, started_at, expires_at, deactivated_at);

-- SQLite foreign keys ensure that referenced rows exist, but cannot express
-- that two independently referenced owners must match or that a parent chain
-- must be acyclic. These triggers provide those hierarchy semantics.
CREATE TRIGGER quests_owner_guard_insert
BEFORE INSERT ON quests
BEGIN
    SELECT CASE WHEN NEW.parent_quest_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM quests WHERE id = NEW.parent_quest_id AND player_id = NEW.player_id
    ) THEN RAISE(ABORT, 'quest parent must belong to the same player') END;
    SELECT CASE WHEN NEW.skill_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM skills s JOIN skill_trees t ON t.id = s.skill_tree_id
        WHERE s.id = NEW.skill_id AND t.player_id = NEW.player_id
    ) THEN RAISE(ABORT, 'quest skill must belong to the same player') END;
END;

CREATE TRIGGER quests_owner_guard_update
BEFORE UPDATE OF player_id, parent_quest_id, skill_id ON quests
BEGIN
    SELECT CASE WHEN NEW.parent_quest_id = NEW.id THEN RAISE(ABORT, 'quest cannot parent itself') END;
    SELECT CASE WHEN NEW.parent_quest_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM quests WHERE id = NEW.parent_quest_id AND player_id = NEW.player_id
    ) THEN RAISE(ABORT, 'quest parent must belong to the same player') END;
    SELECT CASE WHEN NEW.skill_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM skills s JOIN skill_trees t ON t.id = s.skill_tree_id
        WHERE s.id = NEW.skill_id AND t.player_id = NEW.player_id
    ) THEN RAISE(ABORT, 'quest skill must belong to the same player') END;
    SELECT CASE WHEN EXISTS (
        WITH RECURSIVE descendants(id, player_id) AS (
            SELECT id, player_id FROM quests WHERE parent_quest_id = NEW.id
            UNION
            SELECT q.id, q.player_id FROM quests q JOIN descendants d ON q.parent_quest_id = d.id
        ) SELECT 1 FROM descendants WHERE player_id <> NEW.player_id
    ) THEN RAISE(ABORT, 'quest owner change conflicts with child ownership') END;
    SELECT CASE WHEN EXISTS (
        WITH RECURSIVE ancestors(id, parent_quest_id) AS (
            SELECT id, parent_quest_id FROM quests WHERE id = NEW.parent_quest_id
            UNION
            SELECT q.id, q.parent_quest_id FROM quests q JOIN ancestors a ON q.id = a.parent_quest_id
        ) SELECT 1 FROM ancestors WHERE id = NEW.id
    ) THEN RAISE(ABORT, 'quest hierarchy cycle') END;
END;

CREATE TRIGGER skills_hierarchy_guard_insert
BEFORE INSERT ON skills
BEGIN
    SELECT CASE WHEN NEW.parent_skill_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM skills WHERE id = NEW.parent_skill_id AND skill_tree_id = NEW.skill_tree_id
    ) THEN RAISE(ABORT, 'skill parent must belong to the same tree') END;
END;

CREATE TRIGGER skills_hierarchy_guard_update
BEFORE UPDATE OF skill_tree_id, parent_skill_id ON skills
BEGIN
    SELECT CASE WHEN NEW.parent_skill_id = NEW.id THEN RAISE(ABORT, 'skill cannot parent itself') END;
    SELECT CASE WHEN NEW.parent_skill_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM skills WHERE id = NEW.parent_skill_id AND skill_tree_id = NEW.skill_tree_id
    ) THEN RAISE(ABORT, 'skill parent must belong to the same tree') END;
    SELECT CASE WHEN EXISTS (
        WITH RECURSIVE descendants(id, skill_tree_id) AS (
            SELECT id, skill_tree_id FROM skills WHERE parent_skill_id = NEW.id
            UNION
            SELECT s.id, s.skill_tree_id FROM skills s JOIN descendants d ON s.parent_skill_id = d.id
        ) SELECT 1 FROM descendants WHERE skill_tree_id <> NEW.skill_tree_id
    ) THEN RAISE(ABORT, 'skill tree change conflicts with child ownership') END;
    SELECT CASE WHEN EXISTS (
        WITH RECURSIVE ancestors(id, parent_skill_id) AS (
            SELECT id, parent_skill_id FROM skills WHERE id = NEW.parent_skill_id
            UNION
            SELECT s.id, s.parent_skill_id FROM skills s JOIN ancestors a ON s.id = a.parent_skill_id
        ) SELECT 1 FROM ancestors WHERE id = NEW.id
    ) THEN RAISE(ABORT, 'skill hierarchy cycle') END;
END;

-- Enforce nonnegative cached XP regardless of which adapter or SQL path writes it.
CREATE TRIGGER players_xp_floor_insert BEFORE INSERT ON players
WHEN NEW.current_xp < 0 BEGIN SELECT RAISE(ABORT, 'player XP cannot be negative'); END;
CREATE TRIGGER players_xp_floor_update BEFORE UPDATE OF current_xp ON players
WHEN NEW.current_xp < 0 BEGIN SELECT RAISE(ABORT, 'player XP cannot be negative'); END;
CREATE TRIGGER skills_xp_floor_insert BEFORE INSERT ON skills
WHEN NEW.current_xp < 0 BEGIN SELECT RAISE(ABORT, 'skill XP cannot be negative'); END;
CREATE TRIGGER skills_xp_floor_update BEFORE UPDATE OF current_xp ON skills
WHEN NEW.current_xp < 0 BEGIN SELECT RAISE(ABORT, 'skill XP cannot be negative'); END;
CREATE TRIGGER player_snapshots_xp_floor_insert BEFORE INSERT ON player_state_snapshots
WHEN NEW.current_xp < 0 BEGIN SELECT RAISE(ABORT, 'snapshot XP cannot be negative'); END;
CREATE TRIGGER skill_snapshots_xp_floor_insert BEFORE INSERT ON skill_state_snapshots
WHEN NEW.current_xp < 0 BEGIN SELECT RAISE(ABORT, 'snapshot XP cannot be negative'); END;

CREATE TRIGGER xp_transactions_require_applied_amount
BEFORE INSERT ON transactions
WHEN NEW.resource = 'xp' AND (
  NEW.applied_amount IS NULL
  OR (NEW.amount > 0 AND NEW.applied_amount <> NEW.amount)
  OR (NEW.amount < 0 AND (NEW.applied_amount > 0 OR NEW.applied_amount < NEW.amount))
)
BEGIN SELECT RAISE(ABORT, 'XP transaction must record a valid applied amount'); END;

-- Transactions and snapshots are historical. Correct them with new records;
-- never rewrite or delete past events/states.
CREATE TRIGGER transactions_immutable_update BEFORE UPDATE ON transactions
BEGIN SELECT RAISE(ABORT, 'transactions are append-only'); END;
CREATE TRIGGER transactions_immutable_delete BEFORE DELETE ON transactions
BEGIN SELECT RAISE(ABORT, 'transactions are append-only'); END;
CREATE TRIGGER player_snapshots_json_insert BEFORE INSERT ON player_state_snapshots
WHEN json_valid(NEW.state_json) = 0
BEGIN SELECT RAISE(ABORT, 'player snapshot state must be valid JSON'); END;
CREATE TRIGGER skill_snapshots_json_insert BEFORE INSERT ON skill_state_snapshots
WHEN json_valid(NEW.state_json) = 0
BEGIN SELECT RAISE(ABORT, 'skill snapshot state must be valid JSON'); END;
CREATE TRIGGER player_snapshots_immutable_update BEFORE UPDATE ON player_state_snapshots
BEGIN SELECT RAISE(ABORT, 'player snapshots are immutable'); END;
CREATE TRIGGER player_snapshots_immutable_delete BEFORE DELETE ON player_state_snapshots
BEGIN SELECT RAISE(ABORT, 'player snapshots are immutable'); END;
CREATE TRIGGER skill_snapshots_immutable_update BEFORE UPDATE ON skill_state_snapshots
BEGIN SELECT RAISE(ABORT, 'skill snapshots are immutable'); END;
CREATE TRIGGER skill_snapshots_immutable_delete BEFORE DELETE ON skill_state_snapshots
BEGIN SELECT RAISE(ABORT, 'skill snapshots are immutable'); END;

CREATE TRIGGER player_stats_bounds_insert
BEFORE INSERT ON player_stats
BEGIN
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1 FROM player_stat_definitions d WHERE d.code = NEW.stat_code AND d.is_active = 1
          AND (d.minimum IS NULL OR NEW.current_value >= d.minimum)
          AND (d.maximum IS NULL OR NEW.current_value <= d.maximum)
    ) THEN RAISE(ABORT, 'player stat is inactive or outside definition bounds') END;
END;
CREATE TRIGGER player_stats_bounds_update
BEFORE UPDATE OF stat_code, current_value ON player_stats
BEGIN
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1 FROM player_stat_definitions d WHERE d.code = NEW.stat_code AND d.is_active = 1
          AND (d.minimum IS NULL OR NEW.current_value >= d.minimum)
          AND (d.maximum IS NULL OR NEW.current_value <= d.maximum)
    ) THEN RAISE(ABORT, 'player stat is inactive or outside definition bounds') END;
END;
CREATE TRIGGER stat_definition_bounds_update
BEFORE UPDATE OF minimum, maximum, is_active ON player_stat_definitions
WHEN EXISTS (
    SELECT 1 FROM player_stats s WHERE s.stat_code = OLD.code AND (
      NEW.is_active = 0 OR (NEW.minimum IS NOT NULL AND s.current_value < NEW.minimum)
      OR (NEW.maximum IS NOT NULL AND s.current_value > NEW.maximum)
    )
)
BEGIN SELECT RAISE(ABORT, 'stat definition change conflicts with existing player values'); END;

-- Effects are current-state records with explicit, non-destructive manual
-- deactivation. Expiration is computed from `expires_at` at query time.
CREATE TRIGGER effects_lifecycle_guard_insert BEFORE INSERT ON effects
WHEN NEW.deactivated_at IS NOT NULL AND NEW.deactivated_at < NEW.started_at
BEGIN SELECT RAISE(ABORT, 'effect deactivation cannot precede start'); END;
CREATE TRIGGER effects_lifecycle_guard_update BEFORE UPDATE OF started_at, expires_at, deactivated_at ON effects
WHEN (NEW.expires_at IS NOT NULL AND NEW.expires_at < NEW.started_at)
  OR (NEW.deactivated_at IS NOT NULL AND NEW.deactivated_at < NEW.started_at)
BEGIN SELECT RAISE(ABORT, 'effect lifecycle timestamps are inconsistent'); END;
