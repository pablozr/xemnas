CREATE TABLE review_dirty (
 candidate_id TEXT PRIMARY KEY REFERENCES decision_candidates(id) ON DELETE CASCADE
);
INSERT INTO review_dirty SELECT id FROM decision_candidates;
ALTER TABLE review_group_members ADD COLUMN review_required INTEGER NOT NULL DEFAULT 1;
CREATE INDEX review_candidate_order ON decision_candidates(project_id,status,created_at DESC,id DESC);
CREATE INDEX review_target_fingerprint ON review_targets(fingerprint);
CREATE TRIGGER review_candidate_insert AFTER INSERT ON decision_candidates BEGIN
 INSERT OR IGNORE INTO review_dirty VALUES (NEW.id);
END;
CREATE TRIGGER review_candidate_update AFTER UPDATE ON decision_candidates BEGIN
 INSERT OR IGNORE INTO review_dirty VALUES (NEW.id);
END;
CREATE TRIGGER review_nature_insert AFTER INSERT ON candidate_nature BEGIN
 INSERT OR IGNORE INTO review_dirty VALUES (NEW.candidate_id);
END;
CREATE TRIGGER review_nature_update AFTER UPDATE ON candidate_nature BEGIN
 INSERT OR IGNORE INTO review_dirty VALUES (NEW.candidate_id);
END;
CREATE TRIGGER review_nature_delete AFTER DELETE ON candidate_nature BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT id FROM decision_candidates WHERE id=OLD.candidate_id;
END;
CREATE TRIGGER review_refresh_update AFTER UPDATE ON observation_refresh BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc
 JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=NEW.project_id AND n.nature='description';
END;
CREATE TRIGGER review_source_update AFTER UPDATE ON observation_sources BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc
 JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=NEW.project_id AND n.nature='description';
END;
CREATE TRIGGER review_record_update AFTER UPDATE ON observation_records BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc
 JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=NEW.project_id AND n.nature='description';
END;
CREATE TRIGGER review_record_insert AFTER INSERT ON observation_records BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc
 JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=NEW.project_id AND n.nature='description';
END;
CREATE TRIGGER review_source_delete AFTER DELETE ON observation_sources BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc
 JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=OLD.project_id AND n.nature='description';
END;
CREATE TRIGGER review_artifact_update AFTER UPDATE ON capture_artifacts BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT id FROM decision_candidates WHERE capture_id=NEW.capture_id;
END;
CREATE TRIGGER review_artifact_delete AFTER DELETE ON capture_artifacts BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT id FROM decision_candidates WHERE capture_id=OLD.capture_id;
END;
CREATE TRIGGER review_refresh_insert AFTER INSERT ON observation_refresh BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc
 JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=NEW.project_id AND n.nature='description';
END;
CREATE TRIGGER review_refresh_delete AFTER DELETE ON observation_refresh BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc
 JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=OLD.project_id AND n.nature='description';
END;
CREATE TRIGGER review_record_delete AFTER DELETE ON observation_records BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc
 JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=OLD.project_id AND n.nature='description';
END;
CREATE TRIGGER review_source_insert AFTER INSERT ON observation_sources BEGIN
 INSERT OR IGNORE INTO review_dirty SELECT dc.id FROM decision_candidates dc
 JOIN candidate_nature n ON n.candidate_id=dc.id WHERE dc.project_id=NEW.project_id AND n.nature='description';
END;
