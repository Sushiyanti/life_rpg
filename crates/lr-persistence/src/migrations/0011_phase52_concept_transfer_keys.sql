-- Phase 5.2: portable workspace references use Concept-only semantic keys, never local IDs.
ALTER TABLE concepts ADD COLUMN transfer_key TEXT;
UPDATE concepts
   SET transfer_key = 'concept-ref-v1-' || lower(hex(randomblob(16)))
 WHERE transfer_key IS NULL;
CREATE UNIQUE INDEX idx_concepts_transfer_key ON concepts(transfer_key);

-- Keep legacy/test/direct SQL inserts safe while application-created Concepts supply UUID keys.
CREATE TRIGGER concepts_transfer_key_insert
AFTER INSERT ON concepts
WHEN NEW.transfer_key IS NULL OR length(trim(NEW.transfer_key)) = 0
BEGIN
    UPDATE concepts
       SET transfer_key = 'concept-ref-v1-' || lower(hex(randomblob(16)))
     WHERE id = NEW.id;
END;

-- A semantic reference is stable for the lifetime of a Concept.
CREATE TRIGGER concepts_transfer_key_immutable
BEFORE UPDATE OF transfer_key ON concepts
WHEN OLD.transfer_key IS NOT NULL AND NEW.transfer_key IS NOT OLD.transfer_key
BEGIN
    SELECT RAISE(ABORT, 'Concept transfer key is immutable');
END;
