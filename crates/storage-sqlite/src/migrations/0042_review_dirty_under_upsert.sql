-- SQLite applies the conflict clause of the outer statement to the statements
-- of a trigger: an upsert (INSERT ... ON CONFLICT DO UPDATE, as observation
-- refresh scheduling does) turns these triggers' INSERT OR IGNORE into an abort
-- once a candidate is already marked. Each trigger now skips what is marked.
DROP TRIGGER review_candidate_insert;
CREATE TRIGGER review_candidate_insert AFTER INSERT ON decision_candidates BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT NEW.id WHERE NOT EXISTS (SELECT 1 FROM review_dirty WHERE candidate_id=NEW.id);
END;
DROP TRIGGER review_candidate_update;
CREATE TRIGGER review_candidate_update AFTER UPDATE ON decision_candidates BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT NEW.id WHERE NOT EXISTS (SELECT 1 FROM review_dirty WHERE candidate_id=NEW.id);
END;
DROP TRIGGER review_nature_insert;
CREATE TRIGGER review_nature_insert AFTER INSERT ON candidate_nature BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT NEW.candidate_id WHERE NOT EXISTS (SELECT 1 FROM review_dirty WHERE candidate_id=NEW.candidate_id);
END;
DROP TRIGGER review_nature_update;
CREATE TRIGGER review_nature_update AFTER UPDATE ON candidate_nature BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT NEW.candidate_id WHERE NOT EXISTS (SELECT 1 FROM review_dirty WHERE candidate_id=NEW.candidate_id);
END;
DROP TRIGGER review_nature_delete;
CREATE TRIGGER review_nature_delete AFTER DELETE ON candidate_nature BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT id FROM decision_candidates WHERE id=OLD.candidate_id AND id NOT IN (SELECT candidate_id FROM review_dirty);
END;
DROP TRIGGER review_refresh_update;
CREATE TRIGGER review_refresh_update AFTER UPDATE ON observation_refresh BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=NEW.project_id AND n.nature='description' AND dc.id NOT IN (SELECT candidate_id FROM review_dirty);
END;
DROP TRIGGER review_source_update;
CREATE TRIGGER review_source_update AFTER UPDATE ON observation_sources BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=NEW.project_id AND n.nature='description' AND dc.id NOT IN (SELECT candidate_id FROM review_dirty);
END;
DROP TRIGGER review_record_update;
CREATE TRIGGER review_record_update AFTER UPDATE ON observation_records BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=NEW.project_id AND n.nature='description' AND dc.id NOT IN (SELECT candidate_id FROM review_dirty);
END;
DROP TRIGGER review_record_insert;
CREATE TRIGGER review_record_insert AFTER INSERT ON observation_records BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=NEW.project_id AND n.nature='description' AND dc.id NOT IN (SELECT candidate_id FROM review_dirty);
END;
DROP TRIGGER review_source_delete;
CREATE TRIGGER review_source_delete AFTER DELETE ON observation_sources BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=OLD.project_id AND n.nature='description' AND dc.id NOT IN (SELECT candidate_id FROM review_dirty);
END;
DROP TRIGGER review_artifact_update;
CREATE TRIGGER review_artifact_update AFTER UPDATE ON capture_artifacts BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT id FROM decision_candidates WHERE capture_id=NEW.capture_id AND id NOT IN (SELECT candidate_id FROM review_dirty);
END;
DROP TRIGGER review_artifact_delete;
CREATE TRIGGER review_artifact_delete AFTER DELETE ON capture_artifacts BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT id FROM decision_candidates WHERE capture_id=OLD.capture_id AND id NOT IN (SELECT candidate_id FROM review_dirty);
END;
DROP TRIGGER review_refresh_insert;
CREATE TRIGGER review_refresh_insert AFTER INSERT ON observation_refresh BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=NEW.project_id AND n.nature='description' AND dc.id NOT IN (SELECT candidate_id FROM review_dirty);
END;
DROP TRIGGER review_refresh_delete;
CREATE TRIGGER review_refresh_delete AFTER DELETE ON observation_refresh BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=OLD.project_id AND n.nature='description' AND dc.id NOT IN (SELECT candidate_id FROM review_dirty);
END;
DROP TRIGGER review_record_delete;
CREATE TRIGGER review_record_delete AFTER DELETE ON observation_records BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=OLD.project_id AND n.nature='description' AND dc.id NOT IN (SELECT candidate_id FROM review_dirty);
END;
DROP TRIGGER review_source_insert;
CREATE TRIGGER review_source_insert AFTER INSERT ON observation_sources BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=NEW.project_id AND n.nature='description' AND dc.id NOT IN (SELECT candidate_id FROM review_dirty);
END;
