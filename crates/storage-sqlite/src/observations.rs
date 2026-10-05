//! Transactional descriptive observation history and durable refresh scheduling.

use crate::SqliteStore;
use application::observations::refresh::{decode, encode, identity, REFRESH_OBSERVATIONS_KIND};
use application::observations::*;
use rusqlite::{params, OptionalExtension};

fn error(e: rusqlite::Error) -> ObservationError {
    ObservationError(e.to_string())
}

pub(crate) fn schedule(
    connection: &rusqlite::Connection,
    project: &str,
    trigger: &str,
    timestamp: &str,
    enqueue: bool,
) -> rusqlite::Result<i64> {
    connection.execute(
        "INSERT INTO observation_refresh
        (project_id,generation,dirty,capture_trigger,requested_at) VALUES (?1,1,1,?2,?3)
        ON CONFLICT(project_id) DO UPDATE SET generation=generation+1,dirty=1,
        capture_trigger=excluded.capture_trigger,requested_at=excluded.requested_at",
        params![project, trigger, timestamp],
    )?;
    if enqueue {
        connection.execute(
            "INSERT INTO jobs
            (id,kind,payload,state,idempotent,attempts,created_at,updated_at)
            SELECT ?1,?2,?3,'queued',1,0,?4,?4 WHERE NOT EXISTS
            (SELECT 1 FROM jobs WHERE kind=?2 AND payload=?3 AND state='queued')",
            params![
                format!(
                    "observation-{}",
                    identity(
                        project,
                        &format!(
                            "{timestamp}:{}",
                            connection.query_row(
                                "SELECT generation FROM observation_refresh WHERE project_id=?1",
                                [project],
                                |row| row.get::<_, i64>(0)
                            )?
                        )
                    )
                ),
                REFRESH_OBSERVATIONS_KIND,
                project,
                timestamp
            ],
        )?;
    }
    connection.query_row(
        "SELECT generation FROM observation_refresh WHERE project_id=?1",
        [project],
        |row| row.get(0),
    )
}

fn sources(
    connection: &rusqlite::Connection,
    project: &str,
) -> Result<Vec<ObservationSource>, ObservationError> {
    let mut statement = connection
        .prepare(
            "SELECT source_id,project_relative_path,manifest_kind,
         sha256,parser_policy_version,last_checked_at,last_check_status,semantic_cache FROM observation_sources
        WHERE project_id=?1 ORDER BY project_relative_path",
        )
        .map_err(error)?;
    let rows = statement
        .query_map([project], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })
        .map_err(error)?;
    rows.map(|row| {
        let (id, path, kind, hash, policy, checked, status, cache) = row.map_err(error)?;
        Ok(ObservationSource {
            source_id: id,
            project_id: project.into(),
            project_relative_path: path,
            manifest_kind: decode(&kind)?,
            sha256: hash,
            parser_policy_version: policy,
            last_checked_at: checked,
            last_check_status: decode(&status)?,
            semantic_cache: cache.as_deref().map(decode).transpose()?,
        })
    })
    .collect()
}

impl ObservationStore for SqliteStore {
    fn current_sources(&self, project: &str) -> Result<Vec<ObservationSource>, ObservationError> {
        sources(&self.lock(), project)
    }

    fn candidates(
        &self,
        project: &str,
        limit: usize,
    ) -> Result<Vec<ObservationSource>, ObservationError> {
        Ok(self
            .current_sources(project)?
            .into_iter()
            .take(limit.min(MAX_SOURCES))
            .collect())
    }

    fn request_refresh(
        &self,
        request: &RefreshRequest,
    ) -> Result<Option<RefreshGeneration>, ObservationError> {
        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(error)?;
        // Worker claims advance CAS without creating a self-replenishing job.
        let generation = schedule(
            &transaction,
            &request.project_id,
            &request.capture_trigger,
            &request.requested_at,
            request.capture_trigger != "worker",
        )
        .map_err(error)?;
        transaction.commit().map_err(error)?;
        Ok(Some(RefreshGeneration {
            project_id: request.project_id.clone(),
            generation,
            dirty: true,
        }))
    }

    fn apply_refresh(&self, batch: &RefreshBatch) -> Result<ApplyRefreshResult, ObservationError> {
        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(error)?;
        let current: Option<i64> = transaction
            .query_row(
                "SELECT generation FROM observation_refresh WHERE project_id=?1 AND dirty=1",
                [&batch.project_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(error)?;
        if current != Some(batch.expected_generation) {
            return Ok(ApplyRefreshResult::Superseded);
        }
        for source in &batch.sources {
            if source.project_id != batch.project_id {
                return Err(ObservationError("cross-project source".into()));
            }
            let mut statement = transaction
                .prepare(
                    "SELECT record_json FROM observation_records
                WHERE project_id=?1 AND source_id=?2 AND status='current'",
                )
                .map_err(error)?;
            let existing = statement
                .query_map(params![batch.project_id, source.source_id], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(error)?;
            drop(statement);
            transaction.execute("INSERT INTO observation_sources
                (source_id,project_id,project_relative_path,manifest_kind,sha256,
                parser_policy_version,last_checked_at,last_check_status,semantic_cache)
                VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(source_id) DO UPDATE SET
                sha256=excluded.sha256,parser_policy_version=excluded.parser_policy_version,
                last_checked_at=excluded.last_checked_at,last_check_status=excluded.last_check_status,
                semantic_cache=excluded.semantic_cache",
                params![source.source_id, source.project_id, source.project_relative_path,
                    encode(&source.manifest_kind)?, source.sha256, source.parser_policy_version,
                    source.last_checked_at, encode(&source.last_check_status)?,
                    source.semantic_cache.as_ref().map(encode).transpose()?]).map_err(error)?;
            let incoming: Vec<_> = batch
                .observations
                .iter()
                .filter(|record| record.provenance.source_id == source.source_id)
                .collect();
            for json in existing {
                let mut record: ObservationRecord = decode(&json)?;
                let unchanged = source.last_check_status == CheckStatus::Verified
                    && incoming.iter().any(|new| {
                        new.observation_id == record.observation_id
                            && new.subject == record.subject
                            && new.value == record.value
                            && new.provenance.source_sha256 == record.provenance.source_sha256
                            && new.provenance.parser_policy_version
                                == record.provenance.parser_policy_version
                            && new.provenance.supporting_sources
                                == record.provenance.supporting_sources
                    });
                let scope_removed = !matches!(
                    source.project_relative_path.as_str(),
                    "Cargo.toml" | "package.json"
                ) && batch
                    .sources
                    .iter()
                    .filter(|root| {
                        root.manifest_kind == source.manifest_kind
                            && matches!(
                                root.project_relative_path.as_str(),
                                "Cargo.toml" | "package.json"
                            )
                    })
                    .any(|root| {
                        root.last_check_status == CheckStatus::Missing
                            || (root.last_check_status == CheckStatus::Verified
                                && root
                                    .semantic_cache
                                    .as_ref()
                                    .and_then(|c| c.members.as_ref())
                                    .is_some_and(|members| {
                                        !members.contains(&source.project_relative_path)
                                    }))
                    });
                if scope_removed
                    || (!unchanged
                        && matches!(
                            source.last_check_status,
                            CheckStatus::Verified | CheckStatus::Missing
                        ))
                {
                    record.status = RecordStatus::Invalidated;
                    record.invalidated_at = source.last_checked_at.clone();
                    record.invalidation_reason =
                        Some(format!("source {:?}", source.last_check_status));
                    if scope_removed {
                        record.invalidation_reason =
                            Some("declared workspace scope removed".into());
                    }
                    transaction
                        .execute(
                            "UPDATE observation_records SET status='invalidated',record_json=?3
                        WHERE observation_id=?1 AND version=?2",
                            params![record.observation_id, record.version, encode(&record)?],
                        )
                        .map_err(error)?;
                }
            }
            if source.last_check_status != CheckStatus::Verified {
                continue;
            }
            for new in incoming {
                if new.project_id != batch.project_id {
                    return Err(ObservationError("cross-project fact".into()));
                }
                let active: i64 = transaction
                    .query_row(
                        "SELECT COUNT(*) FROM observation_records
                    WHERE observation_id=?1 AND status='current'",
                        [&new.observation_id],
                        |row| row.get(0),
                    )
                    .map_err(error)?;
                if active > 0 {
                    continue;
                }
                let mut record = new.clone();
                record.version = transaction
                    .query_row(
                        "SELECT COALESCE(MAX(version),0)+1
                    FROM observation_records WHERE observation_id=?1",
                        [&record.observation_id],
                        |row| row.get(0),
                    )
                    .map_err(error)?;
                transaction
                    .execute(
                        "INSERT INTO observation_records
                    (observation_id,version,project_id,source_id,status,record_json)
                    VALUES (?1,?2,?3,?4,'current',?5)",
                        params![
                            record.observation_id,
                            record.version,
                            record.project_id,
                            source.source_id,
                            encode(&record)?
                        ],
                    )
                    .map_err(error)?;
            }
        }
        transaction
            .execute(
                "UPDATE observation_refresh SET dirty=0,coverage=?2 WHERE project_id=?1",
                params![batch.project_id, encode(&batch.coverage)?],
            )
            .map_err(error)?;
        transaction.commit().map_err(error)?;
        Ok(ApplyRefreshResult::Applied)
    }

    fn snapshot(&self, project: &str) -> Result<ObservationSnapshot, ObservationError> {
        let connection = self.lock();
        let coverage: Option<(String, bool)> = connection
            .query_row(
                "SELECT coverage,dirty FROM observation_refresh WHERE project_id=?1",
                [project],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(error)?;
        let mut coverage = match coverage {
            Some((json, dirty)) => {
                let mut value: ObservationCoverage = decode(&json)?;
                value.unknown |= dirty;
                value
            }
            None => ObservationCoverage::default(),
        };
        let mut statement = connection
            .prepare(
                "SELECT record_json FROM observation_records
            WHERE project_id=?1 AND status='current' ORDER BY observation_id LIMIT 1025",
            )
            .map_err(error)?;
        let rows = statement
            .query_map([project], |row| row.get::<_, String>(0))
            .map_err(error)?;
        let mut observations = rows
            .map(|row| decode(&row.map_err(error)?))
            .collect::<Result<Vec<ObservationRecord>, ObservationError>>()?;
        if observations.len() > MAX_OBSERVATIONS {
            observations.truncate(MAX_OBSERVATIONS);
            coverage.partial = true;
        }
        let checked_sources = sources(&connection, project)?;
        // Historical records remain stored on unknown checks, but cannot establish
        // current eligibility without every supporting declaration's exact hash.
        observations.retain(|record| {
            record.provenance.supporting_sources.iter().all(|support| {
                checked_sources.iter().any(|source| {
                    source.source_id == support.source_id
                        && source.last_check_status == CheckStatus::Verified
                        && source.sha256.as_deref() == Some(&support.source_sha256)
                })
            })
        });
        Ok(ObservationSnapshot {
            observations,
            sources: checked_sources,
            coverage,
        })
    }
}
