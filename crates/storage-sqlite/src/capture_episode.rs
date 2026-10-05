//! Bounded project-scoped capture detail; never reconstruct source from checkpoints.
use application::capture_episode::{artifact_fact, CaptureEpisode, CaptureEpisodeStore};
use application::captures::{CaptureArtifactRecord, CaptureError};
use rusqlite::params;

use crate::store::SqliteStore;

impl CaptureEpisodeStore for SqliteStore {
    fn capture_episodes(
        &self,
        project_id: &str,
        capture_id: Option<&str>,
        offset: usize,
    ) -> Result<Vec<CaptureEpisode>, CaptureError> {
        let read = || -> rusqlite::Result<Vec<CaptureEpisode>> {
            let mut connection = self.lock();
            let tx = connection.transaction()?;
            let mut query = tx.prepare(
                "SELECT r.capture_id,r.received_at,s.provenance FROM capture_receipts r
                 JOIN projects p ON p.location=r.canonical_path
                 LEFT JOIN capture_episode_sources s ON s.capture_id=r.capture_id
                 WHERE p.id=?1 AND (?2 IS NULL OR r.capture_id=?2)
                 ORDER BY r.received_at DESC,r.capture_id DESC LIMIT 20 OFFSET ?3",
            )?;
            let receipts = query
                .query_map(
                    params![
                        project_id,
                        capture_id,
                        i64::try_from(offset).unwrap_or(i64::MAX)
                    ],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, Option<String>>(2)?,
                        ))
                    },
                )?
                .collect::<Result<Vec<_>, _>>()?;
            drop(query);
            let mut episodes = Vec::new();
            for (capture_id, received_at, source) in receipts {
                let mut query = tx.prepare(
                    "SELECT artifact_id,kind,substr(CAST(content AS BLOB),1,65536),
                     CASE WHEN length(CAST(metadata AS BLOB))<=16384 THEN metadata ELSE '{}' END,
                      fingerprint,length(CAST(content AS BLOB))
                      FROM capture_artifacts WHERE capture_id=?1
                     ORDER BY artifact_id LIMIT 101",
                )?;
                let rows = query
                    .query_map([&capture_id], |r| {
                        let bytes: Vec<u8> = r.get(2)?;
                        let original_length: i64 = r.get(5)?;
                        let truncated = original_length > 65_536;
                        let valid_length = match std::str::from_utf8(&bytes) {
                            Ok(_) => bytes.len(),
                            Err(error) if truncated && error.error_len().is_none() => {
                                error.valid_up_to()
                            }
                            Err(_) => return Err(rusqlite::Error::InvalidQuery),
                        };
                        let content = std::str::from_utf8(&bytes[..valid_length])
                            .map_err(|_| rusqlite::Error::InvalidQuery)?
                            .to_owned();
                        Ok((
                            CaptureArtifactRecord {
                                capture_id: capture_id.clone(),
                                artifact_id: r.get(0)?,
                                kind: r.get(1)?,
                                content,
                                metadata: r.get(3)?,
                                fingerprint: r.get(4)?,
                            },
                            truncated,
                        ))
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                let facts_truncated =
                    rows.len() > 100 || rows.iter().any(|(_, truncated)| *truncated);
                episodes.push(CaptureEpisode {
                    capture_id,
                    received_at,
                    provenance: source
                        .filter(|s| s.len() <= 8192)
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    artifacts: rows
                        .iter()
                        .take(100)
                        .map(|(record, truncated)| {
                            let mut fact = artifact_fact(record);
                            if *truncated {
                                fact.diff_counts = None;
                            }
                            fact
                        })
                        .collect(),
                    facts_truncated,
                });
            }
            tx.commit()?;
            Ok(episodes)
        };
        read().map_err(|e| CaptureError::Storage(e.to_string()))
    }
}
