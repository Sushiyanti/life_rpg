-- 0007_phase35_concepts
-- Typed world subjects and progression; no universal entity/property layer.
-- Preserve event occurrence separately from when this installation observed it.
ALTER TABLE transactions ADD COLUMN captured_at TEXT CHECK(captured_at IS NULL OR captured_at>=occurred_at);
CREATE INDEX idx_transactions_occurred_captured ON transactions(occurred_at,captured_at);
CREATE TABLE concepts (
    id TEXT PRIMARY KEY,
    player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    concept_type_namespace TEXT NOT NULL DEFAULT 'concept' CHECK (concept_type_namespace='concept'),
    concept_type_code TEXT NOT NULL,
    name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 512),
    description TEXT,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0,1)),
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(concept_type_namespace,concept_type_code) REFERENCES type_definitions(namespace,code)
);
CREATE INDEX idx_concepts_player_active ON concepts(player_id,is_active,created_at DESC);
CREATE INDEX idx_concepts_type ON concepts(concept_type_code,player_id);

CREATE TABLE concept_relationship_types (
    code TEXT PRIMARY KEY CHECK(length(code) BETWEEN 1 AND 160 AND code NOT GLOB '*[^a-z0-9_]*'),
    label TEXT NOT NULL CHECK(length(trim(label)) BETWEEN 1 AND 160),
    description TEXT,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0,1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
INSERT INTO concept_relationship_types(code,label,description,created_at,updated_at) VALUES
 ('related_to','Related to','A non-hierarchical semantic association.',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('parent_of','Parent of','Source is a conceptual parent of target.',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('child_of','Child of','Source is a conceptual child of target.',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('depends_on','Depends on','Source depends on target.',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('part_of','Part of','Source is a constituent of target.',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('derived_from','Derived from','Source derives from target.',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 ('prerequisite_of','Prerequisite of','Source is a prerequisite for target.',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'));
CREATE TABLE concept_relationships (
    id TEXT PRIMARY KEY,
    player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    source_concept_id TEXT NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    target_concept_id TEXT NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    relationship_code TEXT NOT NULL REFERENCES concept_relationship_types(code) ON DELETE RESTRICT,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0,1)),
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK(source_concept_id <> target_concept_id),
    UNIQUE(source_concept_id,target_concept_id,relationship_code)
);
CREATE INDEX idx_concept_relationship_source ON concept_relationships(source_concept_id,is_active,relationship_code);
CREATE INDEX idx_concept_relationship_target ON concept_relationships(target_concept_id,is_active,relationship_code);
CREATE TRIGGER concept_relationship_owner_insert BEFORE INSERT ON concept_relationships BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM concepts WHERE id=NEW.source_concept_id AND player_id=NEW.player_id) OR NOT EXISTS(SELECT 1 FROM concepts WHERE id=NEW.target_concept_id AND player_id=NEW.player_id) THEN RAISE(ABORT,'relationship concepts must belong to the same player') END;
END;
CREATE TRIGGER concept_relationship_owner_update BEFORE UPDATE OF player_id,source_concept_id,target_concept_id ON concept_relationships BEGIN
 SELECT CASE WHEN NEW.source_concept_id=NEW.target_concept_id THEN RAISE(ABORT,'concept cannot relate to itself') END;
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM concepts WHERE id=NEW.source_concept_id AND player_id=NEW.player_id) OR NOT EXISTS(SELECT 1 FROM concepts WHERE id=NEW.target_concept_id AND player_id=NEW.player_id) THEN RAISE(ABORT,'relationship concepts must belong to the same player') END;
END;

CREATE TABLE progress_track_definitions (
    code TEXT PRIMARY KEY CHECK(length(code) BETWEEN 1 AND 160 AND code NOT GLOB '*[^a-z0-9_]*'),
    name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 512),
    description TEXT,
    semantics TEXT NOT NULL CHECK(semantics IN ('numeric','percentage','experience','level','mastery')),
    minimum REAL,
    maximum REAL,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0,1)),
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    CHECK(minimum IS NULL OR minimum BETWEEN -1.7976931348623157e308 AND 1.7976931348623157e308),
    CHECK(maximum IS NULL OR maximum BETWEEN -1.7976931348623157e308 AND 1.7976931348623157e308),
    CHECK(minimum IS NULL OR maximum IS NULL OR minimum<=maximum),
    CHECK(semantics<>'percentage' OR ((minimum IS NULL OR minimum>=0) AND (maximum IS NULL OR maximum<=100)))
);
CREATE TABLE concept_progress_tracks (
    id TEXT PRIMARY KEY,
    concept_id TEXT NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    track_code TEXT NOT NULL REFERENCES progress_track_definitions(code) ON DELETE RESTRICT,
    current_value REAL NOT NULL,
    level INTEGER CHECK(level IS NULL OR level>=1),
    is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0,1)),
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(concept_id,track_code),
    CHECK(current_value BETWEEN -1.7976931348623157e308 AND 1.7976931348623157e308)
);
CREATE INDEX idx_concept_tracks_concept ON concept_progress_tracks(concept_id,is_active,track_code);
CREATE TRIGGER concept_track_bounds_insert BEFORE INSERT ON concept_progress_tracks BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM progress_track_definitions d WHERE d.code=NEW.track_code AND d.is_active=1 AND (d.minimum IS NULL OR NEW.current_value>=d.minimum) AND (d.maximum IS NULL OR NEW.current_value<=d.maximum) AND (d.semantics<>'percentage' OR NEW.current_value BETWEEN 0 AND 100) AND (d.semantics<>'experience' OR (NEW.current_value>=0 AND NEW.current_value=CAST(NEW.current_value AS INTEGER))) AND (d.semantics<>'level' OR (NEW.current_value>=1 AND NEW.current_value=CAST(NEW.current_value AS INTEGER)))) THEN RAISE(ABORT,'progress value violates active definition semantics or bounds') END;
END;
CREATE TRIGGER concept_track_bounds_update BEFORE UPDATE OF track_code,current_value ON concept_progress_tracks BEGIN
 SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM progress_track_definitions d WHERE d.code=NEW.track_code AND d.is_active=1 AND (d.minimum IS NULL OR NEW.current_value>=d.minimum) AND (d.maximum IS NULL OR NEW.current_value<=d.maximum) AND (d.semantics<>'percentage' OR NEW.current_value BETWEEN 0 AND 100) AND (d.semantics<>'experience' OR (NEW.current_value>=0 AND NEW.current_value=CAST(NEW.current_value AS INTEGER))) AND (d.semantics<>'level' OR (NEW.current_value>=1 AND NEW.current_value=CAST(NEW.current_value AS INTEGER)))) THEN RAISE(ABORT,'progress value violates active definition semantics or bounds') END;
END;
CREATE TRIGGER progress_definition_guard BEFORE UPDATE OF minimum,maximum,semantics,is_active ON progress_track_definitions WHEN EXISTS(
 SELECT 1 FROM concept_progress_tracks t WHERE t.track_code=OLD.code AND (NEW.is_active=0 OR (NEW.minimum IS NOT NULL AND t.current_value<NEW.minimum) OR (NEW.maximum IS NOT NULL AND t.current_value>NEW.maximum) OR (NEW.semantics='percentage' AND t.current_value NOT BETWEEN 0 AND 100) OR (NEW.semantics='experience' AND (t.current_value<0 OR t.current_value<>CAST(t.current_value AS INTEGER))) OR (NEW.semantics='level' AND (t.current_value<1 OR t.current_value<>CAST(t.current_value AS INTEGER))) )
) BEGIN SELECT RAISE(ABORT,'definition change conflicts with existing progress values'); END;

CREATE TABLE concept_progress_history (
    id TEXT PRIMARY KEY,
    concept_id TEXT NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,
    track_code TEXT NOT NULL REFERENCES progress_track_definitions(code) ON DELETE RESTRICT,
    previous_value REAL,
    current_value REAL NOT NULL,
    level INTEGER,
    occurred_at TEXT NOT NULL,
    captured_at TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),
    CHECK(previous_value IS NULL OR previous_value BETWEEN -1.7976931348623157e308 AND 1.7976931348623157e308),
    CHECK(current_value BETWEEN -1.7976931348623157e308 AND 1.7976931348623157e308),
    CHECK(level IS NULL OR level>=1),
    CHECK(occurred_at<=captured_at)
);
CREATE INDEX idx_concept_progress_history_time ON concept_progress_history(concept_id,track_code,occurred_at DESC,id DESC);
CREATE TRIGGER concept_progress_history_immutable_update BEFORE UPDATE ON concept_progress_history BEGIN SELECT RAISE(ABORT,'concept progress history is append-only'); END;
CREATE TRIGGER concept_progress_history_immutable_delete BEFORE DELETE ON concept_progress_history BEGIN SELECT RAISE(ABORT,'concept progress history is append-only'); END;

CREATE TABLE concept_state_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    concept_id TEXT NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,
    snapshot_date TEXT NOT NULL CHECK(length(snapshot_date)=10),
    state_json TEXT NOT NULL CHECK(json_valid(state_json)),
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),
    captured_at TEXT NOT NULL,
    UNIQUE(concept_id,snapshot_date)
);
CREATE INDEX idx_concept_snapshots_concept_date ON concept_state_snapshots(concept_id,snapshot_date);
CREATE TRIGGER concept_snapshots_immutable_update BEFORE UPDATE ON concept_state_snapshots BEGIN SELECT RAISE(ABORT,'concept snapshots are immutable'); END;
CREATE TRIGGER concept_snapshots_immutable_delete BEFORE DELETE ON concept_state_snapshots BEGIN SELECT RAISE(ABORT,'concept snapshots are immutable'); END;
-- Existing Player/Skill snapshots are immutable and already use created_at as
-- their precise capture moment; do not rewrite historical rows to rename it.

-- Explicit, constrained references from only these named world entities to a Concept.
CREATE TABLE concept_entity_links (
    concept_id TEXT NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    entity_kind TEXT NOT NULL CHECK(entity_kind IN ('quest','skill','skill_tree','effect','transaction','comment','narrative_entry')),
    entity_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY(concept_id,entity_kind,entity_id)
);
CREATE INDEX idx_concept_entity_links_entity ON concept_entity_links(entity_kind,entity_id,concept_id);
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
-- Clean up polymorphic references when their target row is deleted.
CREATE TRIGGER concept_links_quest_delete AFTER DELETE ON quests BEGIN DELETE FROM concept_entity_links WHERE entity_kind='quest' AND entity_id=OLD.id; END;
CREATE TRIGGER concept_links_skill_delete AFTER DELETE ON skills BEGIN DELETE FROM concept_entity_links WHERE entity_kind='skill' AND entity_id=OLD.id; END;
CREATE TRIGGER concept_links_tree_delete AFTER DELETE ON skill_trees BEGIN DELETE FROM concept_entity_links WHERE entity_kind='skill_tree' AND entity_id=OLD.id; END;
CREATE TRIGGER concept_links_effect_delete AFTER DELETE ON effects BEGIN DELETE FROM concept_entity_links WHERE entity_kind='effect' AND entity_id=OLD.id; END;
CREATE TRIGGER concept_links_transaction_delete AFTER DELETE ON transactions BEGIN DELETE FROM concept_entity_links WHERE entity_kind='transaction' AND entity_id=CAST(OLD.id AS TEXT); END;
CREATE TRIGGER concept_links_comment_delete AFTER DELETE ON comments BEGIN DELETE FROM concept_entity_links WHERE entity_kind='comment' AND entity_id=CAST(OLD.id AS TEXT); END;
CREATE TRIGGER concept_links_narrative_delete AFTER DELETE ON narrative_entries BEGIN DELETE FROM concept_entity_links WHERE entity_kind='narrative_entry' AND entity_id=OLD.id; END;

-- Player-owned Effect remains the default; a Concept is the only additional supported target.
ALTER TABLE effects ADD COLUMN target_kind TEXT NOT NULL DEFAULT 'player' CHECK(target_kind IN ('player','concept'));
ALTER TABLE effects ADD COLUMN target_concept_id TEXT REFERENCES concepts(id) ON DELETE RESTRICT;
CREATE TRIGGER effect_target_guard_insert BEFORE INSERT ON effects BEGIN
 SELECT CASE WHEN (NEW.target_kind='player' AND NEW.target_concept_id IS NOT NULL) OR (NEW.target_kind='concept' AND (NEW.target_concept_id IS NULL OR NOT EXISTS(SELECT 1 FROM concepts c WHERE c.id=NEW.target_concept_id AND c.player_id=NEW.player_id))) THEN RAISE(ABORT,'effect target must be Player or same-world Concept') END;
END;
CREATE TRIGGER effect_target_guard_update BEFORE UPDATE OF player_id,target_kind,target_concept_id ON effects BEGIN
 SELECT CASE WHEN (NEW.target_kind='player' AND NEW.target_concept_id IS NOT NULL) OR (NEW.target_kind='concept' AND (NEW.target_concept_id IS NULL OR NOT EXISTS(SELECT 1 FROM concepts c WHERE c.id=NEW.target_concept_id AND c.player_id=NEW.player_id))) THEN RAISE(ABORT,'effect target must be Player or same-world Concept') END;
END;
CREATE INDEX idx_effects_concept_target ON effects(target_concept_id,started_at,expires_at,deactivated_at);

-- Seed illustrative data-defined type vocabularies, not Rust enums.
INSERT INTO type_definitions(namespace,code,label,description,sort_order,is_system) VALUES
 ('concept','subject','Subject','A topic or area of knowledge.',10,1),
 ('concept','project','Project','A bounded piece of work.',20,1),
 ('concept','life_area','Life Area','An ongoing area of personal life.',30,1),
 ('concept','person','Person','A meaningful person.',40,1),
 ('concept','place','Place','A meaningful place.',50,1);
INSERT INTO progress_track_definitions(code,name,semantics,minimum,maximum) VALUES
 ('mastery','Mastery','mastery',NULL,NULL),
 ('familiarity','Familiarity','numeric',NULL,NULL),
 ('confidence','Confidence','percentage',0,100),
 ('progress','Progress','percentage',0,100),
 ('experience','Experience','experience',0,NULL);


-- Global text search stores only searchable identity/type/status/text/timestamps,
-- never full rows, metadata blobs, aggregate state, or rule payloads.
CREATE VIRTUAL TABLE world_search_fts USING fts5(
    kind UNINDEXED, entity_id UNINDEXED, player_id UNINDEXED, type_code UNINDEXED,
    status UNINDEXED, active UNINDEXED, occurred_at UNINDEXED, name, body,
    tokenize='unicode61 remove_diacritics 2'
);
CREATE TRIGGER search_players_insert AFTER INSERT ON players BEGIN
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('player',NEW.id,NEW.id,'player',NULL,NEW.is_active,NEW.updated_at,NEW.name,COALESCE(NEW.description,''));
END;
CREATE TRIGGER search_players_update AFTER UPDATE ON players BEGIN
 DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='player' AND entity_id=OLD.id);
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('player',NEW.id,NEW.id,'player',NULL,NEW.is_active,NEW.updated_at,NEW.name,COALESCE(NEW.description,''));
END;
CREATE TRIGGER search_players_delete AFTER DELETE ON players BEGIN DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='player' AND entity_id=OLD.id); END;
CREATE TRIGGER search_concepts_insert AFTER INSERT ON concepts BEGIN
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('concept',NEW.id,NEW.player_id,NEW.concept_type_code,NULL,NEW.is_active,NEW.updated_at,NEW.name,COALESCE(NEW.description,''));
END;
CREATE TRIGGER search_concepts_update AFTER UPDATE ON concepts BEGIN
 DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='concept' AND entity_id=OLD.id);
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('concept',NEW.id,NEW.player_id,NEW.concept_type_code,NULL,NEW.is_active,NEW.updated_at,NEW.name,COALESCE(NEW.description,''));
END;
CREATE TRIGGER search_concepts_delete AFTER DELETE ON concepts BEGIN DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='concept' AND entity_id=OLD.id); END;
CREATE TRIGGER search_quests_insert AFTER INSERT ON quests BEGIN
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('quest',NEW.id,NEW.player_id,NEW.quest_type_code,NEW.status,CASE WHEN NEW.status IN ('open','active') THEN 1 ELSE 0 END,COALESCE(NEW.completed_at,NEW.started_at,NEW.created_at),NEW.title,trim(COALESCE(NEW.description,'')||' '||COALESCE(NEW.story,'')||' '||COALESCE(NEW.instructions,'')));
END;
CREATE TRIGGER search_quests_update AFTER UPDATE ON quests BEGIN
 DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='quest' AND entity_id=OLD.id);
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('quest',NEW.id,NEW.player_id,NEW.quest_type_code,NEW.status,CASE WHEN NEW.status IN ('open','active') THEN 1 ELSE 0 END,COALESCE(NEW.completed_at,NEW.started_at,NEW.created_at),NEW.title,trim(COALESCE(NEW.description,'')||' '||COALESCE(NEW.story,'')||' '||COALESCE(NEW.instructions,'')));
END;
CREATE TRIGGER search_quests_delete AFTER DELETE ON quests BEGIN DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='quest' AND entity_id=OLD.id); END;
CREATE TRIGGER search_skill_trees_insert AFTER INSERT ON skill_trees BEGIN
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('skill_tree',NEW.id,NEW.player_id,NEW.tree_type_code,NULL,NEW.is_active,NEW.updated_at,NEW.name,trim(COALESCE(NEW.description,'')||' '||COALESCE(NEW.story,'')||' '||COALESCE(NEW.instructions,'')));
END;
CREATE TRIGGER search_skill_trees_update AFTER UPDATE ON skill_trees BEGIN
 DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='skill_tree' AND entity_id=OLD.id);
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('skill_tree',NEW.id,NEW.player_id,NEW.tree_type_code,NULL,NEW.is_active,NEW.updated_at,NEW.name,trim(COALESCE(NEW.description,'')||' '||COALESCE(NEW.story,'')||' '||COALESCE(NEW.instructions,'')));
END;
CREATE TRIGGER search_skill_trees_delete AFTER DELETE ON skill_trees BEGIN DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='skill_tree' AND entity_id=OLD.id); END;
CREATE TRIGGER search_skills_insert AFTER INSERT ON skills BEGIN
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) SELECT 'skill',NEW.id,t.player_id,NEW.skill_type_code,NEW.status,CASE WHEN NEW.status='active' THEN 1 ELSE 0 END,COALESCE(NEW.completed_at,NEW.started_at,NEW.created_at),NEW.name,trim(COALESCE(NEW.description,'')||' '||COALESCE(NEW.story,'')||' '||COALESCE(NEW.instructions,'')) FROM skill_trees t WHERE t.id=NEW.skill_tree_id;
END;
CREATE TRIGGER search_skills_update AFTER UPDATE ON skills BEGIN
 DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='skill' AND entity_id=OLD.id);
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) SELECT 'skill',NEW.id,t.player_id,NEW.skill_type_code,NEW.status,CASE WHEN NEW.status='active' THEN 1 ELSE 0 END,COALESCE(NEW.completed_at,NEW.started_at,NEW.created_at),NEW.name,trim(COALESCE(NEW.description,'')||' '||COALESCE(NEW.story,'')||' '||COALESCE(NEW.instructions,'')) FROM skill_trees t WHERE t.id=NEW.skill_tree_id;
END;
CREATE TRIGGER search_skills_delete AFTER DELETE ON skills BEGIN DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='skill' AND entity_id=OLD.id); END;
CREATE TRIGGER search_effects_insert AFTER INSERT ON effects BEGIN
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('effect',NEW.id,NEW.player_id,NEW.effect_type_code,NULL,1,NEW.started_at,NEW.name,COALESCE(NEW.description,''));
END;
CREATE TRIGGER search_effects_update AFTER UPDATE ON effects BEGIN
 DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='effect' AND entity_id=OLD.id);
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('effect',NEW.id,NEW.player_id,NEW.effect_type_code,NULL,1,NEW.started_at,NEW.name,COALESCE(NEW.description,''));
END;
CREATE TRIGGER search_effects_delete AFTER DELETE ON effects BEGIN DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='effect' AND entity_id=OLD.id); END;
CREATE TRIGGER search_narratives_insert AFTER INSERT ON narrative_entries BEGIN
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('narrative_entry',NEW.id,NEW.player_id,NEW.kind_code,NULL,1,NEW.created_at,NEW.title,NEW.content);
END;
CREATE TRIGGER search_narratives_update AFTER UPDATE ON narrative_entries BEGIN
 DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='narrative_entry' AND entity_id=OLD.id);
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('narrative_entry',NEW.id,NEW.player_id,NEW.kind_code,NULL,1,NEW.created_at,NEW.title,NEW.content);
END;
CREATE TRIGGER search_narratives_delete AFTER DELETE ON narrative_entries BEGIN DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='narrative_entry' AND entity_id=OLD.id); END;
CREATE TRIGGER search_comments_insert AFTER INSERT ON comments BEGIN
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('comment',CAST(NEW.id AS TEXT),NEW.author_player_id,NEW.target_kind,NULL,1,NEW.created_at,'Comment',NEW.body);
END;
CREATE TRIGGER search_comments_update AFTER UPDATE ON comments BEGIN
 DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='comment' AND entity_id=CAST(OLD.id AS TEXT));
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('comment',CAST(NEW.id AS TEXT),NEW.author_player_id,NEW.target_kind,NULL,1,NEW.created_at,'Comment',NEW.body);
END;
CREATE TRIGGER search_comments_delete AFTER DELETE ON comments BEGIN DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='comment' AND entity_id=CAST(OLD.id AS TEXT)); END;
CREATE TRIGGER search_transactions_insert AFTER INSERT ON transactions BEGIN
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) VALUES('transaction',CAST(NEW.id AS TEXT),NEW.player_id,NEW.transaction_type_code,NEW.reason,1,NEW.occurred_at,NEW.resource,trim(COALESCE(NEW.description,'')||' '||COALESCE(NEW.reason,'')));
END;
CREATE TRIGGER search_transactions_delete AFTER DELETE ON transactions BEGIN DELETE FROM world_search_fts WHERE rowid=(SELECT rowid FROM world_search_fts WHERE kind='transaction' AND entity_id=CAST(OLD.id AS TEXT)); END;
CREATE TRIGGER search_concept_progress_insert AFTER INSERT ON concept_progress_history BEGIN
 INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body) SELECT 'concept_progress',NEW.id,c.player_id,NEW.track_code,NULL,1,NEW.occurred_at,c.name,'Progress '||CAST(NEW.previous_value AS TEXT)||' → '||CAST(NEW.current_value AS TEXT) FROM concepts c WHERE c.id=NEW.concept_id;
END;

-- Make pre-existing rows searchable without retaining full domain payloads.
INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body)
 SELECT 'player',id,id,'player',NULL,is_active,updated_at,name,COALESCE(description,'') FROM players;
INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body)
 SELECT 'concept',id,player_id,concept_type_code,NULL,is_active,updated_at,name,COALESCE(description,'') FROM concepts;
INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body)
 SELECT 'quest',id,player_id,quest_type_code,status,CASE WHEN status IN ('open','active') THEN 1 ELSE 0 END,COALESCE(completed_at,started_at,created_at),title,trim(COALESCE(description,'')||' '||COALESCE(story,'')||' '||COALESCE(instructions,'')) FROM quests;
INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body)
 SELECT 'skill_tree',id,player_id,tree_type_code,NULL,is_active,updated_at,name,trim(COALESCE(description,'')||' '||COALESCE(story,'')||' '||COALESCE(instructions,'')) FROM skill_trees;
INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body)
 SELECT 'skill',s.id,t.player_id,s.skill_type_code,s.status,CASE WHEN s.status='active' THEN 1 ELSE 0 END,COALESCE(s.completed_at,s.started_at,s.created_at),s.name,trim(COALESCE(s.description,'')||' '||COALESCE(s.story,'')||' '||COALESCE(s.instructions,'')) FROM skills s JOIN skill_trees t ON t.id=s.skill_tree_id;
INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body)
 SELECT 'effect',id,player_id,effect_type_code,NULL,1,started_at,name,COALESCE(description,'') FROM effects;
INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body)
 SELECT 'narrative_entry',id,player_id,kind_code,NULL,1,created_at,title,content FROM narrative_entries;
INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body)
 SELECT 'comment',CAST(id AS TEXT),author_player_id,target_kind,NULL,1,created_at,'Comment',body FROM comments;
INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body)
 SELECT 'transaction',CAST(id AS TEXT),player_id,transaction_type_code,reason,1,occurred_at,resource,trim(COALESCE(description,'')||' '||COALESCE(reason,'')) FROM transactions;
INSERT INTO world_search_fts(kind,entity_id,player_id,type_code,status,active,occurred_at,name,body)
 SELECT 'concept_progress',h.id,c.player_id,h.track_code,NULL,1,h.occurred_at,c.name,'Progress '||CAST(h.previous_value AS TEXT)||' → '||CAST(h.current_value AS TEXT) FROM concept_progress_history h JOIN concepts c ON c.id=h.concept_id;

-- Rule v1 behavior stays intact; the allow-list now includes a typed progress
-- event while retaining the same schema-versioned data-only interpreter.
CREATE TABLE rules_phase35 (
 id TEXT PRIMARY KEY,
 name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 160),
 description TEXT,
 is_enabled INTEGER NOT NULL DEFAULT 1 CHECK(is_enabled IN (0,1)),
 priority INTEGER NOT NULL DEFAULT 0,
 trigger_kind TEXT NOT NULL CHECK(trigger_kind IN ('quest_completed','player_xp_changed','stat_changed','concept_progress_changed')),
 schema_version INTEGER NOT NULL CHECK(schema_version=1),
 definition_json TEXT NOT NULL CHECK(json_valid(definition_json) AND json_extract(definition_json,'$.schemaVersion')=schema_version AND json_extract(definition_json,'$.trigger')=trigger_kind),
 metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
INSERT INTO rules_phase35 SELECT id,name,description,is_enabled,priority,trigger_kind,schema_version,definition_json,metadata_json,created_at,updated_at FROM rules;
CREATE TABLE rule_execution_history_phase35 (
 id TEXT PRIMARY KEY,
 chain_id TEXT NOT NULL,
 rule_id TEXT NOT NULL REFERENCES rules_phase35(id) ON DELETE RESTRICT,
 event_kind TEXT NOT NULL CHECK(event_kind IN ('quest_completed','player_xp_changed','stat_changed','concept_progress_changed')),
 event_json TEXT NOT NULL CHECK(json_valid(event_json)),
 condition_passed INTEGER CHECK(condition_passed IS NULL OR condition_passed IN (0,1)),
 actions_json TEXT NOT NULL CHECK(json_valid(actions_json)),
 status TEXT NOT NULL CHECK(status IN ('condition_failed','succeeded','failed','guard_aborted')),
 error TEXT,
 depth INTEGER NOT NULL CHECK(depth BETWEEN 0 AND 9),
 executed_at TEXT NOT NULL
);
INSERT INTO rule_execution_history_phase35 SELECT id,chain_id,rule_id,event_kind,event_json,condition_passed,actions_json,status,error,depth,executed_at FROM rule_execution_history;
DROP TABLE rule_execution_history;
DROP TABLE rules;
ALTER TABLE rules_phase35 RENAME TO rules;
ALTER TABLE rule_execution_history_phase35 RENAME TO rule_execution_history;
CREATE INDEX idx_rules_dispatch ON rules(is_enabled,trigger_kind,priority DESC,id ASC);
CREATE INDEX idx_rule_history_chain ON rule_execution_history(chain_id,depth,id);
CREATE INDEX idx_rule_history_rule ON rule_execution_history(rule_id,executed_at DESC);
CREATE TRIGGER rule_history_immutable_update BEFORE UPDATE ON rule_execution_history BEGIN SELECT RAISE(ABORT,'rule execution history is append-only'); END;
CREATE TRIGGER rule_history_immutable_delete BEFORE DELETE ON rule_execution_history BEGIN SELECT RAISE(ABORT,'rule execution history is append-only'); END;
