-- 0012_phase61_effect_history_sessions
-- Effects remain independent world records; links and history are explicit.
CREATE TABLE effect_history (
    id TEXT PRIMARY KEY,
    player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    effect_id TEXT NOT NULL REFERENCES effects(id) ON DELETE CASCADE,
    session_id TEXT REFERENCES quest_sessions(id) ON DELETE SET NULL,
    event_kind TEXT NOT NULL CHECK(event_kind IN ('created','details_changed','expiry_changed','manually_deactivated','session_linked','session_unlinked')),
    recorded_at TEXT NOT NULL,
    previous_state_json TEXT CHECK(previous_state_json IS NULL OR json_valid(previous_state_json)),
    current_state_json TEXT NOT NULL CHECK(json_valid(current_state_json))
);
CREATE INDEX idx_effect_history_effect_time ON effect_history(player_id,effect_id,recorded_at,id);
CREATE INDEX idx_effect_history_session_time ON effect_history(player_id,session_id,recorded_at,id);
CREATE TRIGGER effect_history_immutable_update BEFORE UPDATE ON effect_history BEGIN
    SELECT RAISE(ABORT,'effect history is append-only');
END;
CREATE TRIGGER effect_history_immutable_delete BEFORE DELETE ON effect_history BEGIN
    SELECT RAISE(ABORT,'effect history is append-only');
END;
CREATE TRIGGER effect_history_owner_insert BEFORE INSERT ON effect_history BEGIN
    SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM effects e WHERE e.id=NEW.effect_id AND e.player_id=NEW.player_id)
        THEN RAISE(ABORT,'Effect history must belong to the Effect Player') END;
    SELECT CASE WHEN NEW.session_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM quest_sessions s WHERE s.id=NEW.session_id AND s.player_id=NEW.player_id)
        THEN RAISE(ABORT,'Effect history Session must belong to the same Player') END;
END;

CREATE TABLE session_effects (
    id TEXT PRIMARY KEY,
    player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL REFERENCES quest_sessions(id) ON DELETE CASCADE,
    effect_id TEXT NOT NULL REFERENCES effects(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK(role IN ('relevant','applied','removed','observed')),
    added_at TEXT NOT NULL,
    removed_at TEXT,
    UNIQUE(session_id,effect_id,role),
    CHECK(removed_at IS NULL OR removed_at>=added_at)
);
CREATE INDEX idx_session_effects_session ON session_effects(player_id,session_id,removed_at,added_at);
CREATE INDEX idx_session_effects_effect ON session_effects(player_id,effect_id,removed_at,added_at);
CREATE TRIGGER session_effects_owner_insert BEFORE INSERT ON session_effects BEGIN
    SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM effects e WHERE e.id=NEW.effect_id AND e.player_id=NEW.player_id)
        THEN RAISE(ABORT,'Session Effect must belong to the same Player') END;
    SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM quest_sessions s WHERE s.id=NEW.session_id AND s.player_id=NEW.player_id)
        THEN RAISE(ABORT,'Session Effect must belong to the same Player') END;
END;
CREATE TRIGGER session_effects_owner_update BEFORE UPDATE OF player_id,session_id,effect_id,role ON session_effects BEGIN
    SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM effects e WHERE e.id=NEW.effect_id AND e.player_id=NEW.player_id)
        THEN RAISE(ABORT,'Session Effect must belong to the same Player') END;
    SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM quest_sessions s WHERE s.id=NEW.session_id AND s.player_id=NEW.player_id)
        THEN RAISE(ABORT,'Session Effect must belong to the same Player') END;
END;
