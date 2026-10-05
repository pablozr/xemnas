//! Parser-derived descriptions and generation-scoped refresh use case.

use super::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

type ParseCache = BTreeMap<String, serde_json::Value>;
static PARSE_CACHE: OnceLock<Mutex<ParseCache>> = OnceLock::new();

/// Actual parsing and hash-cache activity for one refresh.
#[derive(Debug, Default)]
pub struct RefreshMetrics {
    /// Manifest syntax parsers invoked.
    pub parser_calls: usize,
    /// Parsed trees reused by exact hash and policy.
    pub cache_hits: usize,
}

/// Cache policy identity, changed when parser semantics change.
pub const PARSER_POLICY_VERSION: &str = "manifest-declarations-v1";
/// Durable job kind; payload is only a registered project identifier.
pub const REFRESH_OBSERVATIONS_KIND: &str = "refresh_observations";

/// Stable project-scoped identity.
pub fn identity(project: &str, key: &str) -> String {
    format!(
        "{:x}",
        Sha256::digest(format!("{}:{project}{key}", project.len()))
    )
}

/// Serialize observation persistence data without exposing a storage dependency.
pub fn encode<T: serde::Serialize>(value: &T) -> Result<String, ObservationError> {
    serde_json::to_string(value).map_err(|e| ObservationError(e.to_string()))
}
/// Deserialize observation persistence data.
pub fn decode<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, ObservationError> {
    serde_json::from_str(value).map_err(|e| ObservationError(e.to_string()))
}

fn source(project: &str, path: &str) -> ObservationSource {
    ObservationSource {
        source_id: identity(project, path),
        project_id: project.into(),
        project_relative_path: path.into(),
        manifest_kind: if path.ends_with("Cargo.toml") {
            ManifestKind::Cargo
        } else {
            ManifestKind::PackageJson
        },
        sha256: None,
        parser_policy_version: PARSER_POLICY_VERSION.into(),
        last_checked_at: None,
        last_check_status: CheckStatus::Unreadable,
        semantic_cache: None,
    }
}

fn parse(bytes: &[u8], kind: ManifestKind) -> Result<serde_json::Value, ObservationError> {
    match kind {
        ManifestKind::PackageJson => {
            serde_json::from_slice(bytes).map_err(|e| ObservationError(e.to_string()))
        }
        ManifestKind::Cargo => {
            let text = std::str::from_utf8(bytes).map_err(|e| ObservationError(e.to_string()))?;
            let value: toml::Value =
                toml::from_str(text).map_err(|e| ObservationError(e.to_string()))?;
            serde_json::to_value(value).map_err(|e| ObservationError(e.to_string()))
        }
    }
}

fn facts(
    source: &ObservationSource,
    tree: &serde_json::Value,
    workspace: Option<&serde_json::Value>,
    trigger: &str,
    timestamp: &str,
) -> Vec<ObservationRecord> {
    let mut records = Vec::new();
    let package = if source.manifest_kind == ManifestKind::Cargo {
        &tree["package"]
    } else {
        tree
    };
    let mut emit = |pointer: String, subject, value| {
        if records.len() > MAX_OBSERVATIONS {
            return;
        }
        records.push(ObservationRecord {
            observation_id: identity(
                &source.project_id,
                &format!("{}:{pointer}", source.project_relative_path),
            ),
            project_id: source.project_id.clone(),
            version: 1,
            subject,
            value,
            path_scope: vec![source.project_relative_path.clone()],
            provenance: ObservationProvenance {
                source_id: source.source_id.clone(),
                source_sha256: source.sha256.clone().unwrap(),
                supporting_sources: Vec::new(),
                parser_policy_version: PARSER_POLICY_VERSION.into(),
                field_pointer: pointer,
                capture_trigger: trigger.into(),
                commit: None,
            },
            observed_at: timestamp.into(),
            status: RecordStatus::Current,
            invalidated_at: None,
            invalidation_reason: None,
        });
    };
    if let Some(name) = package["name"].as_str() {
        let mut version = package["version"].as_str().map(str::to_owned);
        let mut pointer = "/package".to_string();
        if package["version"]["workspace"].as_bool() == Some(true) {
            version = workspace
                .and_then(|w| w["workspace"]["package"]["version"].as_str())
                .map(str::to_owned);
            pointer.push_str(";Cargo.toml#/workspace/package/version");
        }
        emit(
            pointer,
            ObservationSubject::Package { name: name.into() },
            ObservationValue {
                declared_version: version,
                dependency_category: None,
                version_requirement: None,
                target: None,
            },
        );
    }
    let categories = [
        ("dependencies", DependencyCategory::Runtime),
        ("dev-dependencies", DependencyCategory::Development),
        ("build-dependencies", DependencyCategory::Build),
        ("devDependencies", DependencyCategory::Development),
        ("optionalDependencies", DependencyCategory::Optional),
        ("peerDependencies", DependencyCategory::Peer),
    ];
    let mut tables = vec![(String::new(), tree)];
    if let Some(targets) = tree["target"].as_object() {
        tables.extend(
            targets
                .iter()
                .take(MAX_OBSERVATIONS + 1)
                .map(|(name, value)| (name.clone(), value)),
        );
    }
    for (target, table) in tables {
        for (key, category) in categories {
            if let Some(dependencies) = table[key].as_object() {
                for (name, declaration) in dependencies.iter().take(MAX_OBSERVATIONS + 1) {
                    let inherited = declaration["workspace"].as_bool() == Some(true);
                    let resolved = if inherited {
                        workspace
                            .map(|w| &w["workspace"]["dependencies"][name])
                            .unwrap_or(declaration)
                    } else {
                        declaration
                    };
                    let requirement = resolved.as_str().or_else(|| resolved["version"].as_str());
                    let mut pointer = format!("/{target}/{key}/{name}");
                    if inherited {
                        pointer.push_str(&format!(";Cargo.toml#/workspace/dependencies/{name}"));
                    }
                    emit(
                        pointer,
                        ObservationSubject::Dependency { name: name.clone() },
                        ObservationValue {
                            declared_version: None,
                            dependency_category: Some(
                                if declaration["optional"].as_bool() == Some(true) {
                                    DependencyCategory::Optional
                                } else {
                                    category
                                },
                            ),
                            version_requirement: requirement.map(str::to_owned),
                            target: if target.is_empty() {
                                None
                            } else {
                                Some(target.clone())
                            },
                        },
                    );
                }
            }
        }
    }
    records
}

/// Refresh one registered project; filesystem work occurs before the atomic store CAS.
pub fn refresh_project<S: ObservationStore, R: ObservationReader>(
    store: &S,
    reader: &R,
    project_id: &str,
    root: &str,
    generation: i64,
    trigger: &str,
) -> Result<ApplyRefreshResult, ObservationError> {
    refresh_project_with_metrics(store, reader, project_id, root, generation, trigger)
        .map(|(result, _)| result)
}

/// Refresh with honest local parser counters; the bounded process cache is not durable.
pub fn refresh_project_with_metrics<S: ObservationStore, R: ObservationReader>(
    store: &S,
    reader: &R,
    project_id: &str,
    root: &str,
    generation: i64,
    trigger: &str,
) -> Result<(ApplyRefreshResult, RefreshMetrics), ObservationError> {
    let mut metrics = RefreshMetrics::default();
    let snapshot = store.snapshot(project_id)?;
    let known = snapshot.sources.clone();
    let inherited_values: Vec<_> = snapshot
        .observations
        .iter()
        .filter(|r| r.provenance.field_pointer.contains(";Cargo.toml#"))
        .cloned()
        .collect();
    let mut discovered = std::collections::BTreeSet::new();
    let mut discovery_complete = true;
    let mut root_changed = false;
    let mut failed_roots = std::collections::BTreeSet::new();
    let timestamp = crate::clock::now_rfc3339();
    let mut pending = BTreeMap::new();
    for candidate in store.current_sources(project_id)? {
        pending.insert(candidate.project_relative_path.clone(), candidate);
    }
    for path in ["Cargo.toml", "package.json"] {
        pending
            .entry(path.into())
            .or_insert_with(|| source(project_id, path));
    }
    let mut coverage = ObservationCoverage {
        unknown: false,
        ..Default::default()
    };
    let mut sources = Vec::new();
    let mut observations = Vec::new();
    let mut total = 0;
    let mut workspace = None;
    while !pending.is_empty() {
        let mut candidate = pending
            .remove("Cargo.toml")
            .or_else(|| pending.pop_first().map(|(_, candidate)| candidate))
            .unwrap();
        if sources.len() >= MAX_SOURCES {
            coverage.partial = true;
            coverage.unknown = true;
            break;
        }
        let read = reader.read_source(&SourceReadRequest {
            project_id: project_id.into(),
            project_root: root.into(),
            project_relative_path: candidate.project_relative_path.clone(),
            max_bytes: MAX_SOURCE_BYTES.min(MAX_REFRESH_BYTES - total),
        })?;
        total += read.bytes.len();
        candidate.last_checked_at = Some(timestamp.clone());
        candidate.last_check_status = read.status;
        candidate.parser_policy_version = PARSER_POLICY_VERSION.into();
        if read.status == CheckStatus::Verified {
            let hash = format!("{:x}", Sha256::digest(&read.bytes));
            let previous = known.iter().find(|s| s.source_id == candidate.source_id);
            let unchanged = previous.is_some_and(|s| {
                s.sha256.as_deref() == Some(&hash)
                    && s.parser_policy_version == PARSER_POLICY_VERSION
                    && s.last_check_status == CheckStatus::Verified
            });
            candidate.sha256 = Some(hash);
            let root_manifest = matches!(
                candidate.project_relative_path.as_str(),
                "Cargo.toml" | "package.json"
            );
            if root_manifest && !unchanged {
                root_changed = true;
            }
            let cached_records: Vec<_> = snapshot
                .observations
                .iter()
                .filter(|r| r.provenance.source_id == candidate.source_id)
                .cloned()
                .collect();
            let inheritance = cached_records
                .iter()
                .any(|r| r.provenance.field_pointer.contains(";Cargo.toml#"));
            if unchanged && !root_manifest && !(inheritance && root_changed) {
                metrics.cache_hits += 1;
                observations.extend(cached_records);
                coverage.verified_sources += 1;
                sources.push(candidate);
                continue;
            }
            // Root syntax is needed only to rediscover member scope. Persist its parsed tree
            // as a descriptive package record's pointer-independent cache is not supported;
            // for unchanged roots use known members, while changed roots are parsed below.
            if let Some(cache) = candidate
                .semantic_cache
                .as_ref()
                .filter(|_| unchanged && root_manifest)
            {
                metrics.cache_hits += 1;
                observations.extend(cached_records);
                if let Some(members) = &cache.members {
                    discovered.extend(members.iter().cloned());
                    for path in members {
                        pending
                            .entry(path.clone())
                            .or_insert_with(|| source(project_id, path));
                    }
                } else {
                    discovery_complete = false;
                    coverage.unknown = true;
                }
                if candidate.manifest_kind == ManifestKind::Cargo {
                    let dependencies: serde_json::Map<String, serde_json::Value> = cache
                        .dependencies
                        .iter()
                        .map(|(name, version)| {
                            (
                                name.clone(),
                                version
                                    .clone()
                                    .map(serde_json::Value::String)
                                    .unwrap_or(serde_json::Value::Null),
                            )
                        })
                        .collect();
                    workspace = Some(
                        serde_json::json!({"workspace":{"package":{"version":cache.package_version},
                        "dependencies": dependencies}}),
                    );
                }
                coverage.verified_sources += 1;
                sources.push(candidate);
                continue;
            }
            let cache_key = format!(
                "{project_id}:{}:{:?}:{PARSER_POLICY_VERSION}",
                candidate.sha256.as_deref().unwrap(),
                candidate.manifest_kind
            );
            let cache = PARSE_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
            let cached = cache
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&cache_key)
                .cloned();
            let parsed_tree = if let Some(tree) = cached {
                metrics.cache_hits += 1;
                Ok(tree)
            } else {
                metrics.parser_calls += 1;
                let parsed = parse(&read.bytes, candidate.manifest_kind);
                if let Ok(tree) = &parsed {
                    let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
                    if cache.len() >= MAX_SOURCES {
                        cache.clear();
                    }
                    cache.insert(cache_key, tree.clone());
                }
                parsed
            };
            match parsed_tree {
                Ok(tree) if tree.is_object() => {
                    if candidate.project_relative_path == "Cargo.toml" {
                        workspace = Some(tree.clone());
                        if let Some(members) = tree["workspace"]["members"].as_array() {
                            for member in members.iter().take(MAX_SOURCES + 1) {
                                if let Some(member) = member.as_str() {
                                    if member.contains(['*', '?', '[']) {
                                        coverage.unknown = true;
                                        coverage.partial = true;
                                        discovery_complete = false;
                                    } else {
                                        let path = format!("{member}/Cargo.toml");
                                        discovered.insert(path.clone());
                                        if pending.len() < MAX_SOURCES {
                                            pending
                                                .entry(path.clone())
                                                .or_insert_with(|| source(project_id, &path));
                                        } else {
                                            coverage.partial = true;
                                            coverage.unknown = true;
                                            discovery_complete = false;
                                        }
                                    }
                                }
                            }
                            if members.len() > MAX_SOURCES {
                                coverage.partial = true;
                                coverage.unknown = true;
                                discovery_complete = false;
                            }
                        }
                    }
                    if candidate.project_relative_path == "package.json" {
                        let members = tree["workspaces"]
                            .as_array()
                            .or_else(|| tree["workspaces"]["packages"].as_array());
                        if let Some(members) = members {
                            for member in members.iter().take(MAX_SOURCES) {
                                if let Some(member) = member.as_str() {
                                    if member.contains(['*', '?', '[']) {
                                        discovery_complete = false;
                                        coverage.partial = true;
                                        coverage.unknown = true;
                                    } else {
                                        let path = format!("{member}/package.json");
                                        discovered.insert(path.clone());
                                        if pending.len() < MAX_SOURCES {
                                            pending
                                                .entry(path.clone())
                                                .or_insert_with(|| source(project_id, &path));
                                        } else {
                                            discovery_complete = false;
                                            coverage.partial = true;
                                        }
                                    }
                                }
                            }
                            if members.len() > MAX_SOURCES {
                                discovery_complete = false;
                                coverage.partial = true;
                            }
                        }
                    }
                    if root_manifest {
                        let package_version = tree["workspace"]["package"]["version"]
                            .as_str()
                            .map(str::to_owned);
                        let dependencies = tree["workspace"]["dependencies"]
                            .as_object()
                            .map(|deps| {
                                deps.iter()
                                    .take(MAX_OBSERVATIONS)
                                    .map(|(name, value)| {
                                        (
                                            name.clone(),
                                            value
                                                .as_str()
                                                .or_else(|| value["version"].as_str())
                                                .map(str::to_owned),
                                        )
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        candidate.semantic_cache = Some(ManifestSemanticCache {
                            members: if discovery_complete {
                                Some(
                                    discovered
                                        .iter()
                                        .filter(|p| {
                                            p.ends_with(
                                                if candidate.manifest_kind == ManifestKind::Cargo {
                                                    "Cargo.toml"
                                                } else {
                                                    "package.json"
                                                },
                                            )
                                        })
                                        .cloned()
                                        .collect(),
                                )
                            } else {
                                None
                            },
                            package_version,
                            dependencies,
                        });
                    }
                    let mut parsed =
                        facts(&candidate, &tree, workspace.as_ref(), trigger, &timestamp);
                    if let Some(root_source) = sources.iter().find(|s: &&ObservationSource| {
                        s.project_relative_path == "Cargo.toml"
                            && s.last_check_status == CheckStatus::Verified
                    }) {
                        for record in &mut parsed {
                            if let Some((_, pointer)) =
                                record.provenance.field_pointer.split_once(";Cargo.toml#")
                            {
                                record
                                    .provenance
                                    .supporting_sources
                                    .push(ObservationSupport {
                                        source_id: root_source.source_id.clone(),
                                        source_sha256: root_source.sha256.clone().unwrap(),
                                        field_pointer: pointer.into(),
                                    });
                            }
                        }
                    }
                    if workspace.is_none() && !root_changed {
                        for record in &mut parsed {
                            if let Some(previous) = inherited_values.iter().find(|r| {
                                r.provenance.field_pointer == record.provenance.field_pointer
                            }) {
                                record.value = previous.value.clone();
                            }
                        }
                    }
                    if parsed.len() + observations.len() > MAX_OBSERVATIONS {
                        candidate.last_check_status = CheckStatus::QuotaExceeded;
                    } else {
                        observations.extend(parsed);
                    }
                }
                _ => {
                    candidate.last_check_status = CheckStatus::Unsupported;
                    if root_manifest {
                        discovery_complete = false;
                    }
                }
            }
        }
        match candidate.last_check_status {
            CheckStatus::Verified => coverage.verified_sources += 1,
            CheckStatus::Missing => coverage.missing_sources += 1,
            CheckStatus::QuotaExceeded => {
                coverage.quota_exceeded_sources += 1;
                coverage.partial = true;
                coverage.unknown = true;
            }
            _ => {
                coverage.failed_sources += 1;
                coverage.unknown = true;
            }
        }
        if matches!(
            candidate.project_relative_path.as_str(),
            "Cargo.toml" | "package.json"
        ) && candidate.last_check_status != CheckStatus::Verified
        {
            failed_roots.insert(candidate.project_relative_path.clone());
        }
        sources.push(candidate);
    }
    // A member's bytes cannot independently support inherited declarations.
    if failed_roots.contains("Cargo.toml") {
        let dependent_sources: std::collections::BTreeSet<_> = observations
            .iter()
            .filter(|r| r.provenance.field_pointer.contains(";Cargo.toml#"))
            .map(|r| r.provenance.source_id.clone())
            .collect();
        observations.retain(|r| !dependent_sources.contains(&r.provenance.source_id));
        for checked in &mut sources {
            if dependent_sources.contains(&checked.source_id) {
                checked.last_check_status = CheckStatus::Unsupported;
            }
        }
        coverage.unknown = true;
    }
    if discovery_complete && !coverage.partial {
        for checked in &mut sources {
            if !matches!(
                checked.project_relative_path.as_str(),
                "Cargo.toml" | "package.json"
            ) && !discovered.contains(&checked.project_relative_path)
            {
                checked.last_check_status = CheckStatus::OutOfScope;
                observations.retain(|r| r.provenance.source_id != checked.source_id);
                coverage.unknown = true;
            }
        }
    }
    let applied = store.apply_refresh(&RefreshBatch {
        project_id: project_id.into(),
        expected_generation: generation,
        sources,
        observations,
        coverage,
    })?;
    Ok((applied, metrics))
}

/// Job handler resolving roots exclusively through the registered project repository.
pub struct RefreshObservations<S, R> {
    store: S,
    reader: R,
}

impl<S, R> RefreshObservations<S, R> {
    /// Construct a local refresh handler.
    pub fn new(store: S, reader: R) -> Self {
        Self { store, reader }
    }
}

impl<S: ObservationStore + crate::projects::ProjectRepository, R: ObservationReader>
    RefreshObservations<S, R>
{
    /// Run a project-id-only durable job.
    pub fn run(&self, job: &crate::jobs::JobRecord) -> Result<(), crate::jobs::JobFailure> {
        let work = || -> Result<(), ObservationError> {
            let project = crate::projects::ProjectRepository::get(&self.store, &job.payload)
                .map_err(|e| ObservationError(e.to_string()))?
                .ok_or_else(|| ObservationError("project removed".into()))?;
            let generation = self
                .store
                .request_refresh(&RefreshRequest {
                    project_id: project.id.clone(),
                    capture_trigger: "worker".into(),
                    requested_at: crate::clock::now_rfc3339(),
                })?
                .ok_or_else(|| ObservationError("unsupported refresh".into()))?;
            match refresh_project(
                &self.store,
                &self.reader,
                &project.id,
                &project.location,
                generation.generation,
                "worker",
            )? {
                ApplyRefreshResult::Applied | ApplyRefreshResult::Superseded => Ok(()),
                ApplyRefreshResult::Unsupported => {
                    Err(ObservationError("unsupported refresh".into()))
                }
            }
        };
        work().map_err(|_| crate::jobs::JobFailure::Failed)
    }
}
