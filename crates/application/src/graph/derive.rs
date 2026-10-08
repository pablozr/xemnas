//! Deterministic suggestions from captured work (ADR-0005): which components
//! the decisions touched, which dependencies they added, which map entities
//! their text names, and the `affects` / `uses` edges those imply. No AI.
//!
//! Documentation files a decision touched are evidence, not the part it
//! affects: they never yield `affects` nor a component proposal. A decision
//! taken from an ADR reaches the parts it is about through the mentions in
//! its text ([`super::mention`]) and through the files that text cites. A file
//! counts only when it exists in the project folder (the listing of
//! [`super::repo_files`]), so a path an ADR cites relative to a crate, or one
//! that does not exist, never becomes a component.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use domain::entities::{
    component_prefix, entity_key, pattern_matches, EdgeKind, EdgeOrigin, EntityKind, NodeKind,
};
use domain::time::Timestamp;

use super::mention::{dependency_term, entity_terms, mention_reason, Folded, Term};
use super::repo_files::{counted_files, repo_files, RepoFiles};
use super::{DecisionNode, EdgeRecord, EntityRecord, GraphError, GraphStore, KnowledgeGraph};
use crate::claims::ClaimStore;
use crate::clock::now_rfc3339;
use crate::documents::is_documentation_path;
use crate::jobs::JobRepository;
use crate::projects::ProjectRepository;
use crate::relations::RelationStore;

/// A component the captured work suggests, not yet created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentProposal {
    /// Suggested name (the folder name).
    pub name: String,
    /// Suggested path pattern (`<prefix>/**`).
    pub pattern: String,
    /// Decisions that touched it.
    pub decisions: usize,
    /// Optional description (the package's, for a declared member).
    pub description: String,
    /// The manifest that declares it, when it is a workspace member.
    pub declared: Option<super::WorkspaceKind>,
}

/// A technology the captured work added as a dependency, not yet created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TechnologyProposal {
    /// Dependency name.
    pub name: String,
    /// Decisions whose evidence added it.
    pub decisions: usize,
}

/// What one refresh found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SuggestionReport {
    /// New suggested edges written.
    pub new_edges: usize,
    /// Components worth creating, most used first.
    pub components: Vec<ComponentProposal>,
    /// Technologies worth creating, most used first.
    pub technologies: Vec<TechnologyProposal>,
}

/// Prefix of the reason of a link derived from a dependency the decision's text
/// cites, followed by `"name" (manifest)`.
pub const DEPENDENCY_REASON: &str = "dependência citada: ";
/// Appended to that reason when a second component declares the dependency
/// too, so the rules do not accept it alone.
pub const DEPENDENCY_SHARED_MARK: &str = "; também declarada por outro componente";
/// Components that may declare a dependency for it to point at them; a
/// dependency of more is shared plumbing, not a place.
const MAX_OWNERS: usize = 2;

/// A dependency the text may cite and the components that declare it.
struct OwnedDependency {
    term: Term,
    name: String,
    owners: Vec<(String, String)>,
}

/// Manifest keys that are not dependency names.
const CARGO_KEYS: &[&str] = &[
    "version",
    "edition",
    "name",
    "path",
    "features",
    "default-features",
    "optional",
    "workspace",
    "git",
    "rev",
    "branch",
    "tag",
    "package",
    "authors",
    "description",
    "license",
    "members",
    "resolver",
    "publish",
    "rust-version",
    "repository",
    "homepage",
    "readme",
    "keywords",
    "categories",
    "exclude",
    "include",
    "build",
    "links",
    "default",
];

/// Prepares a project's map in the background: what registering a project
/// and indexing its documents run so extraction, link suggestions and scoped
/// rules find the components from the first analysis.
pub type MapPreparer = std::sync::Arc<dyn Fn(&str) + Send + Sync>;

/// A [`MapPreparer`] over `store`. Best effort: a failure is left for the Map
/// screen, which runs the same work when it loads.
pub fn map_preparer<S>(store: S) -> MapPreparer
where
    S: GraphStore + RelationStore + ClaimStore + ProjectRepository + JobRepository,
    S: Send + Sync + 'static,
{
    let graph = KnowledgeGraph::new(store);
    std::sync::Arc::new(move |project_id: &str| {
        let _ = graph.prepare(project_id);
    })
}

/// The project use case as the app composes it: registering prepares the
/// map. The app and the end-to-end runner both build it here so they cannot
/// drift apart.
pub fn prepared_projects<S>(store: S) -> crate::projects::Projects<S>
where
    S: GraphStore + RelationStore + ClaimStore + ProjectRepository + JobRepository,
    S: Clone + Send + Sync + 'static,
{
    crate::projects::Projects::new(store.clone()).with_map_preparer(map_preparer(store))
}

/// The documentation use case as the app composes it: indexing prepares the
/// map. Shared with the end-to-end runner (see [`prepared_projects`]).
pub fn prepared_documents<S>(store: S) -> crate::documents::Documents<S>
where
    S: GraphStore + RelationStore + ClaimStore + ProjectRepository + JobRepository,
    S: crate::documents::DocumentStore + Clone + Send + Sync + 'static,
{
    crate::documents::Documents::new(store.clone()).with_map_preparer(map_preparer(store))
}

/// Dependency names added by manifest hunks (`Cargo.toml`, `package.json`),
/// in order of appearance, without duplicates.
///
/// The section of each line is read from the hunk (`[dependencies]`,
/// `"devDependencies": {`): lines in other sections (`[package.metadata]`,
/// `"engines"`, scripts) are never dependencies. When the hunk does not show
/// its section, only a line whose value looks like a version or a dependency
/// table counts. A name also removed in the same file is a version change,
/// not an addition.
pub fn added_dependencies(diff: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut removed: BTreeSet<String> = BTreeSet::new();
    let mut file_added: Vec<String> = Vec::new();
    let mut manifest = None;
    let mut section = Section::Unknown;
    let flush =
        |names: &mut Vec<String>, added: &mut Vec<String>, removed: &mut BTreeSet<String>| {
            for name in added.drain(..) {
                if !removed.contains(&name) && !names.contains(&name) {
                    names.push(name);
                }
            }
            removed.clear();
        };
    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            flush(&mut names, &mut file_added, &mut removed);
            let file = rest
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            manifest = if file.ends_with("cargo.toml") {
                Some(Manifest::Cargo)
            } else if file.ends_with("package.json") {
                Some(Manifest::Npm)
            } else {
                None
            };
            section = Section::Unknown;
            continue;
        }
        let Some(kind) = manifest else {
            continue;
        };
        if line.starts_with("@@") {
            // A new hunk may start anywhere in the file.
            section = Section::Unknown;
            continue;
        }
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        let (marker, body) = match line.chars().next() {
            Some('+') => ('+', &line[1..]),
            Some('-') => ('-', &line[1..]),
            Some(' ') => (' ', &line[1..]),
            _ => (' ', line),
        };
        let body = body.trim();
        // Section headers, in context or added, move the section.
        if let Some(next) = match kind {
            Manifest::Cargo => cargo_section(body),
            Manifest::Npm => npm_section(body),
        } {
            if marker != '-' {
                section = next;
            }
            // `[dependencies.tokio]` names a dependency by itself.
            if marker == '+' {
                if let Some(name) = cargo_table_dependency(body) {
                    file_added.push(name);
                }
            }
            continue;
        }
        if kind == Manifest::Npm && body.starts_with('}') && marker != '-' {
            section = Section::Unknown;
            continue;
        }
        let name = match kind {
            Manifest::Cargo => cargo_dependency(body, section),
            Manifest::Npm => npm_dependency(body, section),
        };
        let Some(name) = name else {
            continue;
        };
        match marker {
            '+' => file_added.push(name),
            '-' => {
                removed.insert(name);
            }
            _ => {}
        }
    }
    flush(&mut names, &mut file_added, &mut removed);
    names
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Manifest {
    Cargo,
    Npm,
}

/// Where a manifest line sits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Section {
    /// A dependency table or object.
    Dependencies,
    /// Any other table or object.
    Other,
    /// The hunk does not show it.
    Unknown,
}

/// The section a Cargo table header opens.
fn cargo_section(line: &str) -> Option<Section> {
    let header = line.strip_prefix('[')?.trim_start_matches('[');
    let header = header.trim_end_matches(']').trim();
    if header.is_empty() || line.contains('=') {
        return None;
    }
    let dependency_table = header.ends_with("dependencies")
        || header
            .rsplit_once('.')
            .is_some_and(|(table, _)| table.ends_with("dependencies"));
    Some(if dependency_table {
        Section::Dependencies
    } else {
        Section::Other
    })
}

/// `[dependencies.tokio]` → `tokio`.
fn cargo_table_dependency(line: &str) -> Option<String> {
    let header = line.strip_prefix('[')?.strip_suffix(']')?;
    let (table, name) = header.rsplit_once('.')?;
    (table.ends_with("dependencies") && !name.is_empty()).then(|| name.trim().to_string())
}

/// The section an npm key opening an object starts (`"dependencies": {`).
fn npm_section(line: &str) -> Option<Section> {
    let rest = line.strip_prefix('"')?;
    let (key, value) = rest.split_once("\":")?;
    if !value.trim().starts_with('{') {
        return None;
    }
    Some(
        if matches!(
            key,
            "dependencies" | "devDependencies" | "peerDependencies" | "optionalDependencies"
        ) {
            Section::Dependencies
        } else {
            Section::Other
        },
    )
}

fn cargo_dependency(line: &str, section: Section) -> Option<String> {
    if section == Section::Other {
        return None;
    }
    let (name, value) = line.split_once('=')?;
    let name = name.trim();
    let value = value.trim();
    let valid_name = !name.is_empty()
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
        && !CARGO_KEYS.contains(&name);
    let valid_value = match section {
        Section::Dependencies => value.starts_with('"') || value.starts_with('{'),
        // Without the section, only a version or a dependency table.
        _ => looks_like_cargo_requirement(value),
    };
    (valid_name && valid_value).then(|| name.to_string())
}

/// `"1.2"`, `"^0.4"`, `{ version = …}`, `{ path = …}`, `{ git = …}`,
/// `{ workspace = true }`.
fn looks_like_cargo_requirement(value: &str) -> bool {
    if let Some(text) = value.strip_prefix('"') {
        return text
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_digit() || "^~=<>*".contains(first));
    }
    value.starts_with('{')
        && ["version", "path", "git", "workspace"]
            .iter()
            .any(|key| value.contains(&format!("{key} =")) || value.contains(&format!("{key}=")))
}

fn npm_dependency(line: &str, section: Section) -> Option<String> {
    if section == Section::Other {
        return None;
    }
    let rest = line.strip_prefix('"')?;
    let (name, value) = rest.split_once("\":")?;
    let value = value.trim().trim_start_matches('"');
    let looks_like_version = value
        .chars()
        .next()
        .is_some_and(|first| "^~><=*".contains(first) || first.is_ascii_digit())
        || ["workspace:", "npm:", "file:", "link:", "git"]
            .iter()
            .any(|prefix| value.starts_with(prefix))
        || (section == Section::Dependencies
            && value
                .chars()
                .next()
                .is_some_and(|first| first.is_ascii_alphabetic()));
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "@/._-".contains(character))
        && looks_like_version;
    valid.then(|| name.to_string())
}

impl<S> KnowledgeGraph<S>
where
    S: GraphStore + RelationStore + ClaimStore + ProjectRepository + JobRepository,
{
    /// Gives a project its map: assembles the declared components when the
    /// map is empty and derives the suggestions, the same work the Map screen
    /// does on load. Idempotent: with components in place only the
    /// suggestions are refreshed, and those never repeat.
    ///
    /// # Errors
    ///
    /// `project_not_found` or `storage`.
    pub fn prepare(&self, project_id: &str) -> Result<(), GraphError> {
        self.assemble(project_id)?;
        self.refresh_suggestions(project_id)?;
        Ok(())
    }

    /// Derives suggestions from the decisions in force: writes the new
    /// `affects`/`uses` suggestions (from touched files, added dependencies
    /// and mentions in the text) and lists components and technologies worth
    /// creating. Rejected suggestions are never proposed again.
    ///
    /// # Errors
    ///
    /// `project_not_found`, or `storage` on failure.
    pub fn refresh_suggestions(&self, project_id: &str) -> Result<SuggestionReport, GraphError> {
        let project =
            ProjectRepository::get(&self.store, project_id)?.ok_or(GraphError::ProjectNotFound)?;
        // Aliases first, so mentions below already use them.
        let declared = super::declared_components(std::path::Path::new(&project.location));
        self.merge_aliases(project_id, &declared)?;
        let now = now_rfc3339();
        let at = Timestamp::parse(&now).ok_or(GraphError::Storage("relógio inválido".into()))?;
        let project_keys = super::discover::project_keys(std::path::Path::new(&project.location));
        let entities = self.store.project_entities(project_id)?;
        let live: Vec<&EntityRecord> = entities
            .iter()
            .filter(|entity| entity.retired_at.is_none())
            .collect();
        let mut edges = self.store.project_edges(project_id)?;
        let relations = self.store.project_relations(project_id)?;
        let decisions: Vec<DecisionNode> = self
            .store
            .project_decisions(project_id)?
            .into_iter()
            .filter(|decision| super::query::decision_in_force(decision, &relations, &at))
            .collect();

        let root = std::path::Path::new(&project.location);
        // The listing of the project folder, read when a decision first has a
        // file to check.
        let mut listing: Option<Arc<Option<RepoFiles>>> = None;
        let mut report = SuggestionReport::default();
        let mut prefixes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut technologies: BTreeMap<String, (String, BTreeSet<String>)> = BTreeMap::new();
        // How the text may name each live entity, worked out once.
        let named: Vec<(&EntityRecord, EdgeKind, Vec<Term>)> = live
            .iter()
            .filter_map(|entity| {
                let kind = match entity.kind {
                    EntityKind::Component => EdgeKind::Affects,
                    EntityKind::Technology => EdgeKind::Uses,
                };
                let terms = entity_terms(entity, &project_keys);
                (!terms.is_empty()).then_some((*entity, kind, terms))
            })
            .collect();

        let owned = self.dependency_owners(&live, &declared);
        for decision in &decisions {
            let texts = decision_texts(decision);
            let has_files = decision
                .files
                .iter()
                .any(|file| !is_documentation_path(file));
            let repo =
                has_files.then(|| Arc::clone(listing.get_or_insert_with(|| repo_files(root))));
            let files = counted_files(
                &decision.files,
                decision.from_document,
                &texts,
                repo.as_deref().and_then(Option::as_ref),
            );
            for file in &files {
                let components: Vec<&&EntityRecord> = live
                    .iter()
                    .filter(|entity| entity.kind == EntityKind::Component)
                    .filter(|entity| {
                        entity
                            .patterns
                            .iter()
                            .any(|pattern| pattern_matches(pattern, file))
                    })
                    .collect();
                if components.is_empty() {
                    // A folder that leaves the project is never a component.
                    let prefix = component_prefix(file)
                        .filter(|prefix| !prefix.split('/').any(|part| part == ".."));
                    if let Some(prefix) = prefix {
                        prefixes
                            .entry(prefix)
                            .or_default()
                            .insert(decision.decision_id.clone());
                    }
                }
                for component in components {
                    report.new_edges += self.suggest(
                        &mut edges,
                        project_id,
                        EdgeKind::Affects,
                        (NodeKind::Decision, &decision.decision_id),
                        &component.entity_id,
                        file,
                        &now,
                    )?;
                }
            }
            for dependency in decision
                .diffs
                .iter()
                .flat_map(|diff| added_dependencies(diff))
            {
                let key = entity_key(&dependency);
                let technology = live.iter().find(|entity| {
                    entity.kind == EntityKind::Technology && entity.keys().any(|known| known == key)
                });
                match technology {
                    Some(technology) => {
                        report.new_edges += self.suggest(
                            &mut edges,
                            project_id,
                            EdgeKind::Uses,
                            (NodeKind::Decision, &decision.decision_id),
                            &technology.entity_id,
                            &dependency,
                            &now,
                        )?;
                    }
                    None => {
                        technologies
                            .entry(key)
                            .or_insert_with(|| (dependency.clone(), BTreeSet::new()))
                            .1
                            .insert(decision.decision_id.clone());
                    }
                }
            }
            report.new_edges +=
                self.suggest_dependencies(&mut edges, project_id, decision, &texts, &owned, &now)?;
            report.new_edges +=
                self.suggest_mentions(&mut edges, project_id, decision, &texts, &named, &now)?;
        }

        // Decisions no file tied to the map ask the AI which components they
        // govern (best effort, bounded; see `link_suggestions`).
        let ids: Vec<&str> = decisions
            .iter()
            .map(|decision| decision.decision_id.as_str())
            .collect();
        let components = live
            .iter()
            .filter(|entity| entity.kind == EntityKind::Component)
            .count();
        crate::link_suggestions::queue_untied(&self.store, &ids, &edges, components);

        // Rules adopted from Revisão apply to the components their evidence
        // touched: suggested, like any derived edge.
        let claims = self
            .store
            .project_claims(project_id)
            .map_err(|error| GraphError::Storage(error.to_string()))?;
        // Claims derived from a decision follow its confirmed component ties
        // (backfill for ties made before the claim or before this rule).
        let ties = Self::decision_ties(&edges);
        self.inherit_claim_scope(&mut edges, &claims, &ties, &now)?;
        // Standing rules still without a component tie, and whose source
        // decision gives none, ask the AI which components they govern.
        crate::link_suggestions::queue_untied_claims(&self.store, &claims, &edges, components, &at);
        let statements: BTreeMap<String, String> = claims
            .iter()
            .map(|claim| (claim.claim_id.clone(), claim.statement.clone()))
            .collect();
        let valid_claims: BTreeSet<String> = claims
            .into_iter()
            .filter(|claim| claim.is_valid_at(&at))
            .map(|claim| claim.claim_id)
            .collect();
        for rule in self.store.rule_sources(project_id)? {
            if !valid_claims.contains(&rule.claim_id) {
                continue;
            }
            let has_files = rule.files.iter().any(|file| !is_documentation_path(file));
            let repo =
                has_files.then(|| Arc::clone(listing.get_or_insert_with(|| repo_files(root))));
            let statement = [Folded::new(
                statements.get(&rule.claim_id).map_or("", String::as_str),
            )];
            let files = counted_files(
                &rule.files,
                rule.from_document,
                &statement,
                repo.as_deref().and_then(Option::as_ref),
            );
            for file in &files {
                for component in live.iter().filter(|entity| {
                    entity.kind == EntityKind::Component
                        && entity
                            .patterns
                            .iter()
                            .any(|pattern| pattern_matches(pattern, file))
                }) {
                    report.new_edges += self.suggest(
                        &mut edges,
                        project_id,
                        EdgeKind::AppliesTo,
                        (NodeKind::Claim, &rule.claim_id),
                        &component.entity_id,
                        file,
                        &now,
                    )?;
                }
            }
        }

        // Any entity, even retired, blocks proposing the same name again.
        let known: BTreeSet<(EntityKind, String)> = entities
            .iter()
            .flat_map(|entity| entity.keys().map(move |key| (entity.kind, key)))
            .collect();
        // A component is identified by its path. The folder name is a fine
        // display name until two folders share it (`apps/api`, `services/api`)
        // or a known entity already has it: then the path names it.
        let folder = |prefix: &str| prefix.rsplit('/').next().unwrap_or(prefix).to_string();
        let mut shared: BTreeMap<String, usize> = BTreeMap::new();
        for prefix in prefixes.keys() {
            *shared.entry(entity_key(&folder(prefix))).or_default() += 1;
        }
        let claimed: BTreeSet<String> = entities
            .iter()
            .filter(|entity| entity.kind == EntityKind::Component)
            .flat_map(|entity| entity.patterns.clone())
            .collect();
        report.components = prefixes
            .into_iter()
            .filter(|(prefix, _)| !claimed.contains(&format!("{prefix}/**")))
            .filter_map(|(prefix, ids)| {
                let short = folder(&prefix);
                let ambiguous = shared.get(&entity_key(&short)).copied().unwrap_or(0) > 1
                    || known.contains(&(EntityKind::Component, entity_key(&short)));
                let name = if ambiguous { prefix.clone() } else { short };
                (!known.contains(&(EntityKind::Component, entity_key(&name)))).then(|| {
                    ComponentProposal {
                        pattern: format!("{prefix}/**"),
                        name,
                        decisions: ids.len(),
                        description: String::new(),
                        declared: None,
                    }
                })
            })
            .collect();
        // Members the project declares come first: they are structure the
        // team wrote down, not a guess from a diff.
        let declared: Vec<ComponentProposal> = declared
            .iter()
            .filter(|member| !claimed.contains(&member.pattern))
            .filter(|member| !known.contains(&(EntityKind::Component, entity_key(&member.name))))
            .map(|member| ComponentProposal {
                decisions: report
                    .components
                    .iter()
                    .find(|inferred| inferred.pattern == member.pattern)
                    .map_or(0, |inferred| inferred.decisions),
                name: member.name.clone(),
                pattern: member.pattern.clone(),
                description: member.description.clone(),
                declared: Some(member.source),
            })
            .collect();
        report.components.retain(|inferred| {
            !declared
                .iter()
                .any(|member| member.pattern == inferred.pattern)
        });
        report.components.sort_by(|left, right| {
            right
                .decisions
                .cmp(&left.decisions)
                .then(left.name.cmp(&right.name))
        });
        report.components.splice(0..0, declared);
        report.technologies = technologies
            .into_iter()
            .filter(|(key, _)| !known.contains(&(EntityKind::Technology, key.clone())))
            .map(|(_, (name, ids))| TechnologyProposal {
                name,
                decisions: ids.len(),
            })
            .collect();
        report.technologies.sort_by(|left, right| {
            right
                .decisions
                .cmp(&left.decisions)
                .then(left.name.cmp(&right.name))
        });
        Ok(report)
    }

    /// The dependencies the members declare, each with the live components
    /// that declare it. Left out: a dependency that is a member of the
    /// workspace, one a live component is already named after, and one that
    /// more than [`MAX_OWNERS`] components declare.
    fn dependency_owners(
        &self,
        live: &[&EntityRecord],
        declared: &[super::DeclaredComponent],
    ) -> Vec<OwnedDependency> {
        let taken: BTreeSet<String> = declared
            .iter()
            .map(|member| entity_key(&member.name))
            .chain(
                live.iter()
                    .filter(|entity| entity.kind == EntityKind::Component)
                    .flat_map(|entity| entity.keys()),
            )
            .collect();
        let mut by_key: BTreeMap<String, OwnedDependency> = BTreeMap::new();
        for member in declared {
            let Some(component) = live.iter().find(|entity| {
                entity.kind == EntityKind::Component && entity.patterns.contains(&member.pattern)
            }) else {
                continue;
            };
            for dependency in &member.dependencies {
                let key = entity_key(dependency);
                if taken.contains(&key) {
                    continue;
                }
                let Some(term) = dependency_term(dependency) else {
                    continue;
                };
                let entry = by_key.entry(key).or_insert_with(|| OwnedDependency {
                    term,
                    name: dependency.clone(),
                    owners: Vec::new(),
                });
                if !entry
                    .owners
                    .iter()
                    .any(|(id, _)| *id == component.entity_id)
                {
                    entry
                        .owners
                        .push((component.entity_id.clone(), member.manifest()));
                }
            }
        }
        by_key
            .into_values()
            .filter(|dependency| dependency.owners.len() <= MAX_OWNERS)
            .collect()
    }

    /// Ties the decision to the component that declares a dependency its text
    /// cites (not to exclude it). With a second declaring component the
    /// reason says so.
    fn suggest_dependencies(
        &self,
        edges: &mut Vec<EdgeRecord>,
        project_id: &str,
        decision: &DecisionNode,
        texts: &[Folded],
        owned: &[OwnedDependency],
        now: &str,
    ) -> Result<usize, GraphError> {
        let source = (NodeKind::Decision, decision.decision_id.as_str());
        let mut written = 0;
        for dependency in owned {
            if !texts
                .iter()
                .any(|text| text.mention(&dependency.term).is_some())
            {
                continue;
            }
            let shared = if dependency.owners.len() > 1 {
                DEPENDENCY_SHARED_MARK
            } else {
                ""
            };
            for (entity_id, manifest) in &dependency.owners {
                let reason = format!(
                    "{DEPENDENCY_REASON}\"{}\" ({manifest}{shared})",
                    dependency.name
                );
                written += self.suggest(
                    edges,
                    project_id,
                    EdgeKind::Affects,
                    source,
                    entity_id,
                    &reason,
                    now,
                )?;
            }
        }
        Ok(written)
    }

    /// Suggests the entities the decision's text names (question, choice,
    /// rationale, assumptions, scope, consequences), with the quote as the
    /// reason. An entity already tied to the decision is not searched for.
    fn suggest_mentions(
        &self,
        edges: &mut Vec<EdgeRecord>,
        project_id: &str,
        decision: &DecisionNode,
        texts: &[Folded],
        named: &[(&EntityRecord, EdgeKind, Vec<Term>)],
        now: &str,
    ) -> Result<usize, GraphError> {
        if named.is_empty() {
            return Ok(0);
        }
        let source = (NodeKind::Decision, decision.decision_id.as_str());
        let mut written = 0;
        for (entity, kind, terms) in named {
            if edge_exists(edges, *kind, source, &entity.entity_id) {
                continue;
            }
            let quote = terms
                .iter()
                .find_map(|term| texts.iter().find_map(|text| text.mention(term)));
            if let Some(quote) = quote {
                written += self.suggest(
                    edges,
                    project_id,
                    *kind,
                    source,
                    &entity.entity_id,
                    &mention_reason(&quote),
                    now,
                )?;
            }
        }
        Ok(written)
    }

    /// Writes one derived suggestion unless any row for the same edge exists
    /// (pending, confirmed or rejected, or written earlier in this run).
    /// Returns 1 when written.
    #[allow(clippy::too_many_arguments)]
    fn suggest(
        &self,
        edges: &mut Vec<EdgeRecord>,
        project_id: &str,
        kind: EdgeKind,
        source: (NodeKind, &str),
        entity_id: &str,
        reason: &str,
        now: &str,
    ) -> Result<usize, GraphError> {
        let (source_kind, source_id) = source;
        if edge_exists(edges, kind, source, entity_id) {
            return Ok(0);
        }
        let record = EdgeRecord {
            edge_id: uuid::Uuid::now_v7().to_string(),
            project_id: project_id.to_string(),
            kind,
            source_kind,
            source_id: source_id.to_string(),
            entity_id: entity_id.to_string(),
            origin: EdgeOrigin::Derived,
            reason: reason.to_string(),
            created_at: now.to_string(),
            confirmed_at: None,
            invalidated_at: None,
        };
        self.store.insert_edge(&record)?;
        // Later files of the same decision see it and do not repeat it.
        edges.push(record);
        Ok(1)
    }
}

/// The text of a decision, ready to be searched: question, choice, rationale,
/// assumptions, scope and consequences.
fn decision_texts(decision: &DecisionNode) -> Vec<Folded> {
    [&decision.question, &decision.choice, &decision.rationale]
        .into_iter()
        .chain(&decision.context)
        .map(|text| Folded::new(text))
        .collect()
}

/// Whether any row (pending, confirmed or rejected) ties `source` to the
/// entity with `kind`.
fn edge_exists(
    edges: &[EdgeRecord],
    kind: EdgeKind,
    source: (NodeKind, &str),
    entity_id: &str,
) -> bool {
    edges.iter().any(|edge| {
        edge.kind == kind
            && edge.source_kind == source.0
            && edge.source_id == source.1
            && edge.entity_id == entity_id
    })
}

#[cfg(test)]
mod tests {
    use super::added_dependencies;

    #[test]
    fn reads_added_dependencies_from_manifest_hunks() {
        let diff = "diff --git a/crates/app/Cargo.toml b/crates/app/Cargo.toml\n\
                    @@ -1,3 +1,6 @@\n \
                    [dependencies]\n\
                    +rusqlite = { version = \"0.31\", features = [\"bundled\"] }\n\
                    +serde = \"1\"\n\
                    +version = \"0.2.0\"\n\
                    +[dependencies.tokio]\n\
                    diff --git a/web/package.json b/web/package.json\n\
                    @@ -1,2 +1,4 @@\n \
                    \"dependencies\": {\n\
                    +    \"@tanstack/query\": \"^5.0.0\",\n\
                    +    \"zod\": \"workspace:*\"\n\
                    diff --git a/src/main.rs b/src/main.rs\n\
                    +let x = \"1\";\n";
        assert_eq!(
            added_dependencies(diff),
            vec!["rusqlite", "serde", "tokio", "@tanstack/query", "zod"]
        );
    }

    #[test]
    fn other_sections_are_not_dependencies() {
        // The two false positives found in the audit.
        let cargo = "diff --git a/Cargo.toml b/Cargo.toml\n\
                     @@ -1 +1,3 @@\n \
                     [package.metadata]\n\
                     +channel = \"stable\"\n";
        assert!(added_dependencies(cargo).is_empty());
        let npm = "diff --git a/package.json b/package.json\n\
                   @@ -1 +1,3 @@\n \
                   \"engines\": {\n\
                   +  \"node\": \">=22\"\n";
        assert!(added_dependencies(npm).is_empty());
        let scripts = "diff --git a/package.json b/package.json\n\
                       @@ -1 +1,3 @@\n \
                       \"scripts\": {\n\
                       +  \"build\": \"tsc -p .\"\n";
        assert!(added_dependencies(scripts).is_empty());
    }

    #[test]
    fn a_version_change_is_not_an_addition() {
        let diff = "diff --git a/Cargo.toml b/Cargo.toml\n\
                    @@ -3,3 +3,3 @@\n \
                    [dependencies]\n\
                    -serde = \"1.0.100\"\n\
                    +serde = \"1.0.200\"\n\
                    +anyhow = \"1\"\n";
        assert_eq!(added_dependencies(diff), vec!["anyhow"]);
    }

    #[test]
    fn without_a_visible_section_only_requirements_count() {
        let diff = "diff --git a/Cargo.toml b/Cargo.toml\n\
                    @@ -40,3 +40,5 @@\n \
                    tokio = \"1\"\n\
                    +regex = \"1.10\"\n\
                    +channel = \"stable\"\n\
                    +sqlx = { version = \"0.8\", features = [\"sqlite\"] }\n";
        assert_eq!(added_dependencies(diff), vec!["regex", "sqlx"]);
    }
}
