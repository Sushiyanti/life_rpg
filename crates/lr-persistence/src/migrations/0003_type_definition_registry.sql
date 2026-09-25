-- 0003_type_definition_registry
-- Database-driven type definitions.
--
-- This is the mechanism the brief asks for: "do not create a new source-code
-- class whenever a new conceptual subtype is invented". Adding `quest` type
-- `raid`, or a new `effect` type, is an INSERT here — no migration, no recompile.
--
-- What this is NOT:
--   * It is not an entity table. Entities (quests, skills, …) arrive in later
--     phases and reference these rows by `(namespace, code)`.
--   * It is not "everything is JSON". The structural columns that every type
--     shares — namespace, code, label, ordering, active flag — are real,
--     typed, indexed columns. Only the genuinely open-ended extras go in
--     `metadata_json`. Types have a shape; their metadata does not.
--
-- `namespace` groups the kinds of things that can have types, and is itself
-- just data: a later phase can add `narrative_entry`, `transaction`, etc.

CREATE TABLE IF NOT EXISTS type_definitions (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    namespace     TEXT    NOT NULL,
    code          TEXT    NOT NULL,
    label         TEXT    NOT NULL,
    description   TEXT,
    sort_order    INTEGER NOT NULL DEFAULT 0,
    is_active     INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    is_system     INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
    metadata_json TEXT    NOT NULL DEFAULT '{}',
    created_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (namespace, code)
);

CREATE INDEX IF NOT EXISTS idx_type_definitions_namespace
    ON type_definitions (namespace, sort_order);

-- Seed data: the type vocabulary named in the requirements. These are the
-- starting set, not a closed set.
INSERT INTO type_definitions (namespace, code, label, description, sort_order, is_system)
VALUES
    ('quest',     'main',         'Main Quest',      'Drives the primary storyline.',            10, 1),
    ('quest',     'side',         'Side Quest',      'Optional objective with its own reward.',  20, 1),
    ('quest',     'daily',        'Daily Quest',     'Repeats on a daily cadence.',              30, 1),
    ('quest',     'long_term',    'Long Term Quest','Runs across weeks or months.',             40, 1),
    ('quest',     'challenge',    'Challenge',       'High difficulty, high reward.',            50, 1),

    ('skill_tree','programming',  'Programming',     'Software craft progression tree.',         10, 1),
    ('skill_tree','fitness',      'Fitness',         'Physical training progression tree.',      20, 1),
    ('skill_tree','education',    'Education',       'Formal and self-directed learning.',       30, 1),
    ('skill_tree','life',         'Life',            'Everyday life skills and habits.',         40, 1),

    ('effect',    'buff',         'Buff',            'Positive temporary modifier.',             10, 1),
    ('effect',    'debuff',       'Debuff',          'Negative temporary modifier.',             20, 1),
    ('effect',    'condition',    'Condition',       'A state the player is currently in.',      30, 1),
    ('effect',    'temporary_modifier', 'Temporary Modifier', 'Scoped numeric modifier.',        40, 1),

    ('transaction','xp',          'Experience',      'Experience point movement.',               10, 1),
    ('transaction','money',       'Money',           'Currency movement.',                       20, 1),
    ('transaction','time',        'Time',            'Time invested in an activity.',            30, 1),

    ('narrative_entry', 'note',       'Note',        'Player-authored record.',                  10, 1),
    ('narrative_entry', 'briefing',   'Briefing',    'Context shown before a quest.',            20, 1),
    ('narrative_entry', 'story',      'Story',       'Narrative content advancing an arc.',      30, 1),
    ('narrative_entry', 'reflection', 'Reflection',  'End-of-period review.',                    40, 1),
    ('narrative_entry', 'reminder',   'Reminder',    'Something to surface later.',              50, 1),
    ('narrative_entry', 'journal',    'Journal',     'Freeform journaling entry.',               60, 1),
    ('narrative_entry', 'system',     'System',      'System-generated message.',                70, 1),

    ('comment_target',  'quest',      'Quest',       'Comments attachable to a quest.',          10, 1),
    ('comment_target',  'skill',      'Skill',       'Comments attachable to a skill.',          20, 1),
    ('comment_target',  'skill_tree', 'Skill Tree',  'Comments attachable to a skill tree.',     30, 1),
    ('comment_target',  'effect',     'Effect',      'Comments attachable to an effect.',        40, 1),
    ('comment_target',  'player',     'Player',      'Comments attachable to the player.',       50, 1),
    ('comment_target',  'transaction','Transaction', 'Comments attachable to a transaction.',    60, 1),
    ('comment_target',  'narrative_entry', 'Narrative Entry', 'Comments attachable to a narrative entry.', 70, 1)
ON CONFLICT (namespace, code) DO NOTHING;
