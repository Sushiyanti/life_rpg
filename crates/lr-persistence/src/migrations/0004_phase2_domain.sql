-- 0004_phase2_domain
-- Persistent Phase 2 world domain. Earlier migrations are intentionally untouched.
-- Current aggregate state, immutable history, and daily snapshots are distinct.

CREATE TABLE players (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    description TEXT,
    level INTEGER NOT NULL CHECK (level >= 1),
    current_xp INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_players_active ON players (is_active, name);

CREATE TABLE skill_trees (
    id TEXT PRIMARY KEY,
    player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    tree_type_namespace TEXT NOT NULL DEFAULT 'skill_tree' CHECK (tree_type_namespace = 'skill_tree'),
    tree_type_code TEXT NOT NULL,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    description TEXT,
    story TEXT,
    instructions TEXT,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (tree_type_namespace, tree_type_code) REFERENCES type_definitions(namespace, code)
);
CREATE INDEX idx_skill_trees_player ON skill_trees (player_id, is_active, created_at);

CREATE TABLE skills (
    id TEXT PRIMARY KEY,
    skill_tree_id TEXT NOT NULL REFERENCES skill_trees(id) ON DELETE CASCADE,
    parent_skill_id TEXT REFERENCES skills(id) ON DELETE SET NULL,
    skill_type_namespace TEXT NOT NULL DEFAULT 'skill' CHECK (skill_type_namespace = 'skill'),
    skill_type_code TEXT NOT NULL,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    description TEXT,
    story TEXT,
    instructions TEXT,
    level INTEGER NOT NULL DEFAULT 1 CHECK (level >= 1),
    current_xp INTEGER NOT NULL DEFAULT 0,
    invested_minutes INTEGER NOT NULL DEFAULT 0 CHECK (invested_minutes >= 0),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'completed', 'archived')),
    started_at TEXT,
    completed_at TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (skill_type_namespace, skill_type_code) REFERENCES type_definitions(namespace, code),
    CHECK (parent_skill_id IS NULL OR parent_skill_id <> id)
);
CREATE INDEX idx_skills_tree ON skills (skill_tree_id, created_at);
CREATE INDEX idx_skills_parent ON skills (parent_skill_id);

CREATE TABLE quests (
    id TEXT PRIMARY KEY,
    player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    parent_quest_id TEXT REFERENCES quests(id) ON DELETE SET NULL,
    skill_id TEXT REFERENCES skills(id) ON DELETE SET NULL,
    quest_type_namespace TEXT NOT NULL DEFAULT 'quest' CHECK (quest_type_namespace = 'quest'),
    quest_type_code TEXT NOT NULL,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    description TEXT,
    story TEXT,
    instructions TEXT,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'active', 'completed', 'abandoned')),
    difficulty INTEGER CHECK (difficulty IS NULL OR difficulty >= 0),
    progress INTEGER NOT NULL DEFAULT 0 CHECK (progress BETWEEN 0 AND 100),
    xp_reward INTEGER NOT NULL DEFAULT 0 CHECK (xp_reward >= 0),
    due_at TEXT,
    started_at TEXT,
    completed_at TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (quest_type_namespace, quest_type_code) REFERENCES type_definitions(namespace, code),
    CHECK (parent_quest_id IS NULL OR parent_quest_id <> id)
);
CREATE INDEX idx_quests_player ON quests (player_id, status, created_at DESC);
CREATE INDEX idx_quests_parent ON quests (parent_quest_id);
CREATE INDEX idx_quests_skill ON quests (skill_id);

CREATE TABLE effects (
    id TEXT PRIMARY KEY,
    player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    effect_type_namespace TEXT NOT NULL DEFAULT 'effect' CHECK (effect_type_namespace = 'effect'),
    effect_type_code TEXT NOT NULL,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    description TEXT,
    started_at TEXT NOT NULL,
    expires_at TEXT,
    intensity INTEGER NOT NULL DEFAULT 1,
    source_kind TEXT,
    source_id TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (effect_type_namespace, effect_type_code) REFERENCES type_definitions(namespace, code),
    CHECK (expires_at IS NULL OR expires_at >= started_at)
);
CREATE INDEX idx_effects_player_active ON effects (player_id, expires_at);

CREATE TABLE transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    transaction_type_namespace TEXT NOT NULL DEFAULT 'transaction' CHECK (transaction_type_namespace = 'transaction'),
    transaction_type_code TEXT NOT NULL,
    resource TEXT NOT NULL CHECK (length(trim(resource)) > 0),
    amount INTEGER NOT NULL CHECK (amount <> 0),
    occurred_at TEXT NOT NULL,
    reason TEXT,
    description TEXT,
    source_kind TEXT,
    source_id TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    FOREIGN KEY (transaction_type_namespace, transaction_type_code) REFERENCES type_definitions(namespace, code)
);
CREATE INDEX idx_transactions_player_occurred ON transactions (player_id, occurred_at DESC, id DESC);
CREATE INDEX idx_transactions_player_resource ON transactions (player_id, resource);

CREATE TABLE player_state_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    snapshot_date TEXT NOT NULL CHECK (length(snapshot_date) = 10),
    level INTEGER NOT NULL CHECK (level >= 1),
    current_xp INTEGER NOT NULL,
    state_json TEXT NOT NULL DEFAULT '{}',
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    UNIQUE (player_id, snapshot_date)
);
CREATE INDEX idx_player_snapshots_player_date ON player_state_snapshots (player_id, snapshot_date);

CREATE TABLE skill_state_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    snapshot_date TEXT NOT NULL CHECK (length(snapshot_date) = 10),
    level INTEGER NOT NULL CHECK (level >= 1),
    current_xp INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'paused', 'completed', 'archived')),
    invested_minutes INTEGER NOT NULL CHECK (invested_minutes >= 0),
    state_json TEXT NOT NULL DEFAULT '{}',
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    UNIQUE (skill_id, snapshot_date)
);
CREATE INDEX idx_skill_snapshots_skill_date ON skill_state_snapshots (skill_id, snapshot_date);

CREATE TABLE narrative_entries (
    id TEXT PRIMARY KEY,
    player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    kind_namespace TEXT NOT NULL DEFAULT 'narrative_entry' CHECK (kind_namespace = 'narrative_entry'),
    kind_code TEXT NOT NULL,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    content TEXT NOT NULL CHECK (length(trim(content)) > 0),
    author TEXT,
    source_kind TEXT,
    source_id TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (kind_namespace, kind_code) REFERENCES type_definitions(namespace, code)
);
CREATE INDEX idx_narrative_entries_player ON narrative_entries (player_id, created_at DESC);

-- A polymorphic target cannot have one SQL foreign key. The store verifies target
-- existence within the same transaction as insertion; this check blocks unknown kinds.
CREATE TABLE comments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    author_player_id TEXT REFERENCES players(id) ON DELETE SET NULL,
    target_kind TEXT NOT NULL CHECK (target_kind IN ('player','quest','skill','skill_tree','effect','transaction','narrative_entry')),
    target_id TEXT NOT NULL,
    body TEXT NOT NULL CHECK (length(trim(body)) > 0),
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_comments_target ON comments (target_kind, target_id, created_at);

-- Phase 1 seeded all requested namespaces except skill, which Phase 2 now consumes.
INSERT INTO type_definitions (namespace, code, label, description, sort_order, is_system)
VALUES
    ('skill', 'core', 'Core', 'A foundational skill of its tree.', 10, 1),
    ('skill', 'advanced', 'Advanced', 'A skill building on core skills.', 20, 1),
    ('skill', 'specialization', 'Specialization', 'A narrow mastery branch.', 30, 1)
ON CONFLICT (namespace, code) DO NOTHING;
