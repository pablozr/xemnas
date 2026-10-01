//! Deterministic suggestions from captured work (ADR-0005): which components
//! the decisions touched, which dependencies they added, and the `affects` /
//! `uses` edges those imply. No AI, no reading of the repository.

use std::collections::{BTreeMap, BTreeSet};

use domain::entities::{
    component_prefix, entity_key, pattern_matches, EdgeKind, EdgeOrigin, EntityKind, NodeKind,
};
use domain::time::Timestamp;

use super::{DecisionNode, EdgeRecord, EntityRecord, GraphError, GraphStore, KnowledgeGraph};
use crate::claims::ClaimStore;
use crate::clock::now_rfc3339;
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
    S: GraphStore + RelationStore + ClaimStore + ProjectRepository,
{
    /// Derives suggestions from the decisions in force: writes the new
    /// `affects`/`uses` suggestions and lists components and technologies
    /// worth creating. Rejected suggestions are never proposed again.
    ///
    /// # Errors
    ///
    /// `project_not_found`, or `storage` on failure.
    pub fn refresh_suggestions(&self, project_id: &str) -> Result<SuggestionReport, GraphError> {
        ProjectRepository::get(&self.store, project_id)?.ok_or(GraphError::ProjectNotFound)?;
        let now = now_rfc3339();
        let at = Timestamp::parse(&now).ok_or(GraphError::Storage("relógio inválido".into()))?;
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

        let mut report = SuggestionReport::default();
        let mut prefixes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut technologies: BTreeMap<String, (String, BTreeSet<String>)> = BTreeMap::new();

        for decision in &decisions {
            for file in &decision.files {
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
                    if let Some(prefix) = component_prefix(file) {
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
                        &decision.decision_id,
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
                            &decision.decision_id,
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
        }

        // Any entity, even retired, blocks proposing the same name again.
        let known: BTreeSet<(EntityKind, String)> = entities
            .iter()
            .flat_map(|entity| entity.keys().map(move |key| (entity.kind, key)))
            .collect();
        report.components = prefixes
            .into_iter()
            .filter_map(|(prefix, ids)| {
                let name = prefix.rsplit('/').next().unwrap_or(&prefix).to_string();
                (!known.contains(&(EntityKind::Component, entity_key(&name)))).then(|| {
                    ComponentProposal {
                        pattern: format!("{prefix}/**"),
                        name,
                        decisions: ids.len(),
                    }
                })
            })
            .collect();
        report.components.sort_by(|left, right| {
            right
                .decisions
                .cmp(&left.decisions)
                .then(left.name.cmp(&right.name))
        });
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

    /// Writes one derived suggestion unless any row for the same edge exists
    /// (pending, confirmed or rejected, or written earlier in this run).
    /// Returns 1 when written.
    #[allow(clippy::too_many_arguments)]
    fn suggest(
        &self,
        edges: &mut Vec<EdgeRecord>,
        project_id: &str,
        kind: EdgeKind,
        decision_id: &str,
        entity_id: &str,
        reason: &str,
        now: &str,
    ) -> Result<usize, GraphError> {
        let exists = edges.iter().any(|edge| {
            edge.kind == kind
                && edge.source_kind == NodeKind::Decision
                && edge.source_id == decision_id
                && edge.entity_id == entity_id
        });
        if exists {
            return Ok(0);
        }
        let record = EdgeRecord {
            edge_id: uuid::Uuid::now_v7().to_string(),
            project_id: project_id.to_string(),
            kind,
            source_kind: NodeKind::Decision,
            source_id: decision_id.to_string(),
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
