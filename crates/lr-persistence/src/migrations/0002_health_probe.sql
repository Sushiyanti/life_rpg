-- 0002_health_probe
-- Storage for the write/read round-trip proof used by the status screen.
--
-- Why a real table instead of a `SELECT 1`?
-- Because `SELECT 1` proves the driver loads, not that *persistence* works. A
-- row that survives a COMMIT and is then read back is evidence that the file is
-- writable, the schema is present, and the query path is wired up. The status
-- screen is only allowed to claim "SQLite is reachable and writing" because
-- this table said so.
--
-- This table is infrastructure, not game domain data, so it is never read by a
-- repository and is excluded from any future export/backup of the world.

CREATE TABLE IF NOT EXISTS health_probe (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    token      TEXT    NOT NULL,
    written_at TEXT    NOT NULL,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_health_probe_written_at
    ON health_probe (written_at);
