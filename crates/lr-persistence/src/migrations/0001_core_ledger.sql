-- 0001_core_ledger
-- Baseline schema for the Life RPG local store.
--
-- Phase 1 deliberately creates very little *domain* structure: the brief says
-- not to implement the Player/Quest/Skill system yet. What it does create is
-- the foundation every later migration will lean on:
--
--   * `app_meta`       — key/value facts about this particular store (world id,
--                        created-at, format label). Lives in the database, not
--                        in a config file, so the store is self-describing.
--
-- `schema_migrations` is intentionally NOT created here: the migration runner
-- creates it before reading anything, so it must exist before migration 1 runs.

CREATE TABLE IF NOT EXISTS app_meta (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT INTO app_meta (key, value, updated_at)
VALUES
    ('store.format',       'life-rpg/sqlite', '1970-01-01T00:00:00+00:00'),
    ('store.world_label',  'Primary World',   '1970-01-01T00:00:00+00:00')
ON CONFLICT (key) DO NOTHING;
