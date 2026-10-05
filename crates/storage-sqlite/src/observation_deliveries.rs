//! Typed descriptive delivery audit, isolated from normative injection items.

use application::injection::InjectionMode;
use application::observations::refresh::{decode, encode};
use application::observations::{
    CheckStatus, DeliveredObservation, ObservationCorrection, ObservationDeliveryStore,
    ObservationError, ObservationSnapshot, ObservationStore, RecordStatus,
};
use rusqlite::params;

use crate::store::SqliteStore;

fn error(value: impl std::fmt::Display) -> ObservationError {
    ObservationError(value.to_string())
}

fn include_original_records(
    store: &SqliteStore,
    project: &str,
    originals: &[DeliveredObservation],
    snapshot: &mut ObservationSnapshot,
) -> Result<(), ObservationError> {
    let connection = store.lock();
    for original in originals {
        if snapshot
            .observations
            .iter()
            .any(|record| record.observation_id == original.observation_id)
        {
            continue;
        }
        use rusqlite::OptionalExtension;
        let json: Option<String> = connection.query_row(
            "SELECT record_json FROM observation_records WHERE project_id=?1 AND observation_id=?2 ORDER BY version DESC LIMIT 1",
            params![project, original.observation_id], |row| row.get(0),
        ).optional().map_err(error)?;
        if let Some(json) = json {
            snapshot.observations.push(decode(&json)?);
        }
    }
    Ok(())
}

fn pending(
    original: &DeliveredObservation,
    snapshot: &ObservationSnapshot,
) -> Option<ObservationCorrection> {
    let source = snapshot.sources.iter().find(|source| {
        source.project_id == original.project_id && source.source_id == original.source_id
    });
    if let Some(source) = source.filter(|source| {
        matches!(
            source.last_check_status,
            CheckStatus::Unreadable | CheckStatus::Unsupported | CheckStatus::QuotaExceeded
        )
    }) {
        return Some(ObservationCorrection {
            original: original.clone(),
            invalidated_at: source.last_checked_at.clone()?,
            reason: format!("não foi possível revalidar; não tratar como atual; status={:?}; hash={:?}; policy={}",
                source.last_check_status, source.sha256, source.parser_policy_version),
            replacement: None,
        });
    }
    let record = snapshot.observations.iter().find(|record| {
        record.project_id == original.project_id && record.observation_id == original.observation_id
    });
    if let Some(record) = record {
        if record.status == RecordStatus::Invalidated || record.version != original.version {
            return Some(ObservationCorrection {
                original: original.clone(),
                invalidated_at: record
                    .invalidated_at
                    .clone()
                    .unwrap_or_else(|| record.observed_at.clone()),
                reason: record
                    .invalidation_reason
                    .clone()
                    .unwrap_or_else(|| "declaração substituída por nova versão".into()),
                replacement: (record.status == RecordStatus::Current).then(|| {
                    DeliveredObservation {
                        project_id: record.project_id.clone(),
                        observation_id: record.observation_id.clone(),
                        version: record.version,
                        source_id: record.provenance.source_id.clone(),
                    }
                }),
            });
        }
    }
    // A failed read or disappearance from a bounded snapshot is not evidence of removal.
    let source = snapshot.sources.iter().find(|source| {
        source.project_id == original.project_id
            && source.source_id == original.source_id
            && source.last_check_status == CheckStatus::Missing
    })?;
    Some(ObservationCorrection {
        original: original.clone(),
        invalidated_at: source.last_checked_at.clone()?,
        reason: "fonte confirmada ausente".into(),
        replacement: None,
    })
}

impl ObservationDeliveryStore for SqliteStore {
    fn delivered_observations(
        &self,
        session: &str,
        project: &str,
        mode: InjectionMode,
    ) -> Result<Vec<DeliveredObservation>, ObservationError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT delivered_json FROM observation_deliveries \
             WHERE project_id=?1 AND session_id=?2 AND mode=?3 \
             ORDER BY observation_id, version",
            )
            .map_err(error)?;
        let rows = statement
            .query_map(params![project, session, mode.as_str()], |row| {
                row.get::<_, String>(0)
            })
            .map_err(error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(error)?;
        rows.into_iter().map(|json| decode(&json)).collect()
    }

    fn observation_corrections(
        &self,
        session: &str,
        project: &str,
        mode: InjectionMode,
    ) -> Result<Vec<ObservationCorrection>, ObservationError> {
        let originals = {
            let connection = self.lock();
            let mut statement = connection
                .prepare(
                    "SELECT delivered_json FROM observation_deliveries \
                 WHERE project_id=?1 AND session_id=?2 AND mode=?3 \
                 ORDER BY observation_id, version",
                )
                .map_err(error)?;
            let rows = statement
                .query_map(params![project, session, mode.as_str()], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(error)?;
            rows.into_iter()
                .map(|json| decode::<DeliveredObservation>(&json))
                .collect::<Result<Vec<_>, _>>()?
        };
        let mut snapshot = self.snapshot(project)?;
        include_original_records(self, project, &originals, &mut snapshot)?;
        let mut candidates: Vec<_> = originals
            .iter()
            .filter_map(|original| pending(original, &snapshot))
            .collect();
        let connection = self.lock();
        for original in &originals {
            if candidates.iter().any(|item| item.original == *original) {
                continue;
            }
            let acknowledged: Option<String> = connection.query_row(
                "SELECT correction_json FROM observation_deliveries WHERE project_id=?1 AND session_id=?2 AND mode=?3 AND observation_id=?4 AND version=?5",
                params![project, session, mode.as_str(), original.observation_id, original.version],
                |row| row.get(0),
            ).map_err(error)?;
            if acknowledged
                .as_deref()
                .is_some_and(|json| json.contains("não foi possível revalidar"))
            {
                if let Some(source) = snapshot.sources.iter().find(|source| {
                    source.source_id == original.source_id
                        && source.last_check_status == CheckStatus::Verified
                }) {
                    candidates.push(ObservationCorrection {
                        original: original.clone(),
                        invalidated_at: source.last_checked_at.clone().unwrap_or_default(),
                        reason: "revalidada; mesma versão descritiva; não é regra".into(),
                        replacement: Some(original.clone()),
                    });
                }
            }
        }
        let mut result = Vec::new();
        for correction in candidates {
            let acknowledged: Option<String> = connection.query_row(
                "SELECT correction_json FROM observation_deliveries WHERE project_id=?1 AND session_id=?2 AND mode=?3 AND observation_id=?4 AND version=?5",
                params![project, session, mode.as_str(), correction.original.observation_id, correction.original.version],
                |row| row.get(0),
            ).map_err(error)?;
            if acknowledged.as_deref() != Some(encode(&correction)?.as_str()) {
                result.push(correction);
            }
        }
        Ok(result)
    }

    fn record_observation_delivery(
        &self,
        session: &str,
        project: &str,
        mode: InjectionMode,
        observations: &[DeliveredObservation],
        corrections: &[ObservationCorrection],
    ) -> Result<bool, ObservationError> {
        // Optimistic CAS over this store's shared connection: any intervening mutation
        // rejects the entire audit rather than marking a stale snapshot as fresh.
        let (changes, data_version) = {
            let connection = self.lock();
            let version: i64 = connection
                .query_row("PRAGMA data_version", [], |row| row.get(0))
                .map_err(error)?;
            (connection.total_changes(), version)
        };
        let mut snapshot = self.snapshot(project)?;
        let originals: Vec<_> = corrections
            .iter()
            .map(|item| item.original.clone())
            .collect();
        include_original_records(self, project, &originals, &mut snapshot)?;
        for delivered in observations {
            let valid = snapshot.observations.iter().any(|record| {
                record.project_id == project
                    && delivered.project_id == project
                    && record.observation_id == delivered.observation_id
                    && record.version == delivered.version
                    && record.status == RecordStatus::Current
                    && record.provenance.source_id == delivered.source_id
                    && record.provenance.supporting_sources.iter().all(|support| {
                        snapshot.sources.iter().any(|source| {
                            source.project_id == project
                                && source.source_id == support.source_id
                                && source.last_check_status == CheckStatus::Verified
                                && source.sha256.as_deref() == Some(support.source_sha256.as_str())
                        })
                    })
                    && snapshot.sources.iter().any(|source| {
                        source.project_id == project
                            && source.source_id == delivered.source_id
                            && source.last_check_status == CheckStatus::Verified
                            && source.sha256.as_deref()
                                == Some(record.provenance.source_sha256.as_str())
                            && source.parser_policy_version
                                == record.provenance.parser_policy_version
                    })
            });
            if !valid {
                return Ok(false);
            }
        }
        for correction in corrections {
            let recovery = correction.reason == "revalidada; mesma versão descritiva; não é regra"
                && correction.replacement.as_ref() == Some(&correction.original)
                && snapshot.observations.iter().any(|record| {
                    record.observation_id == correction.original.observation_id
                        && record.version == correction.original.version
                        && record.status == RecordStatus::Current
                })
                && snapshot.sources.iter().any(|source| {
                    source.source_id == correction.original.source_id
                        && source.last_check_status == CheckStatus::Verified
                        && source.last_checked_at.as_deref()
                            == Some(correction.invalidated_at.as_str())
                });
            if correction.original.project_id != project
                || (!recovery
                    && pending(&correction.original, &snapshot).as_ref() != Some(correction))
            {
                return Ok(false);
            }
        }
        let mut connection = self.lock();
        if connection.total_changes() != changes {
            return Ok(false);
        }
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(error)?;
        let latest: i64 = transaction
            .query_row("PRAGMA data_version", [], |row| row.get(0))
            .map_err(error)?;
        if latest != data_version {
            return Ok(false);
        }
        let reaffirmed: Vec<_> = corrections
            .iter()
            .filter(|correction| {
                correction.reason == "revalidada; mesma versão descritiva; não é regra"
            })
            .filter_map(|correction| correction.replacement.as_ref())
            .collect();
        if !observations.is_empty() || !reaffirmed.is_empty() {
            let dirty: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM observation_refresh WHERE project_id=?1 AND dirty=1)",
                [project], |row| row.get(0),
            ).map_err(error)?;
            if dirty {
                return Ok(false);
            }
        }
        // Validate raw SQL identity/status as well as JSON, under the write lock.
        for delivered in observations.iter().chain(reaffirmed.iter().copied()) {
            let record = snapshot
                .observations
                .iter()
                .find(|record| {
                    record.observation_id == delivered.observation_id
                        && record.version == delivered.version
                })
                .ok_or_else(|| ObservationError("versão ausente no snapshot".into()))?;
            for support in &record.provenance.supporting_sources {
                let valid: bool = transaction
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM observation_sources WHERE project_id=?1 \
                     AND source_id=?2 AND sha256=?3 AND last_check_status='\"verified\"')",
                        params![project, support.source_id, support.source_sha256],
                        |row| row.get(0),
                    )
                    .map_err(error)?;
                if !valid {
                    return Ok(false);
                }
            }
            let own_hash: bool = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM observation_sources WHERE project_id=?1 \
                 AND source_id=?2 AND sha256=?3 AND parser_policy_version=?4 \
                 AND last_check_status='\"verified\"')",
                    params![
                        project,
                        delivered.source_id,
                        record.provenance.source_sha256,
                        record.provenance.parser_policy_version
                    ],
                    |row| row.get(0),
                )
                .map_err(error)?;
            if !own_hash {
                return Ok(false);
            }
            let matches: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM observation_records r \
                 JOIN observation_sources s ON s.source_id=r.source_id AND s.project_id=r.project_id \
                 WHERE r.project_id=?1 AND r.observation_id=?2 AND r.version=?3 \
                 AND r.source_id=?4 AND r.status='current' AND s.last_check_status='\"verified\"')",
                params![project, delivered.observation_id, delivered.version, delivered.source_id],
                |row| row.get(0),
            ).map_err(error)?;
            if !matches {
                return Ok(false);
            }
        }
        for correction in corrections {
            let count = transaction
                .execute(
                    "UPDATE observation_deliveries SET correction_json=?1 \
                 WHERE project_id=?2 AND session_id=?3 AND mode=?4 \
                  AND observation_id=?5 AND version=?6 \
                  AND (correction_json IS NULL OR correction_json != ?1)",
                    params![
                        encode(correction)?,
                        project,
                        session,
                        mode.as_str(),
                        correction.original.observation_id,
                        correction.original.version
                    ],
                )
                .map_err(error)?;
            if count != 1 {
                return Ok(false);
            }
        }
        for delivered in observations {
            let count = transaction
                .execute(
                    "INSERT OR IGNORE INTO observation_deliveries \
                 (project_id,session_id,mode,observation_id,version,delivered_json) \
                 VALUES (?1,?2,?3,?4,?5,?6)",
                    params![
                        project,
                        session,
                        mode.as_str(),
                        delivered.observation_id,
                        delivered.version,
                        encode(delivered)?
                    ],
                )
                .map_err(error)?;
            if count != 1 {
                return Ok(false);
            }
        }
        transaction.commit().map_err(error)?;
        Ok(true)
    }
}
