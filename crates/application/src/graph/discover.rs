//! Components the project declares: members of a Cargo workspace, npm
//! `workspaces` and `pnpm-workspace.yaml` packages, read from the project
//! folder. A declared member is structure the team already wrote down, so an
//! empty map is assembled from it at once; folders inferred from captured
//! diffs stay proposals (ADR-0005, revision of 2026-10-01).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use super::cache::FolderCache;
use super::{EntityEdit, GraphError, GraphStore, KnowledgeGraph, NewEntity, MAX_ENTITY_LIST};
use crate::claims::ClaimStore;
use crate::projects::ProjectRepository;
use crate::relations::RelationStore;
use domain::entities::{entity_key, EntityKind, MAX_DESCRIPTION_CHARS};

/// Members expanded per workspace pattern, at most.
const MAX_MEMBERS: usize = 200;

/// Where a component was declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceKind {
    /// `[workspace] members` of the root `Cargo.toml`.
    Cargo,
    /// `workspaces` of the root `package.json`.
    Npm,
    /// `packages` of `pnpm-workspace.yaml`.
    Pnpm,
}

impl WorkspaceKind {
    /// Product label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Cargo => "workspace Cargo",
            Self::Npm => "workspaces do npm",
            Self::Pnpm => "workspace pnpm",
        }
    }
}

/// What a component that is not a workspace member stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfraKind {
    /// The files at the root of a workspace: its manifest, toolchain and
    /// shared configuration.
    Root,
    /// A continuous integration system and its pipelines.
    Ci,
}

/// One member the project declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredComponent {
    /// Package name, or the folder name when the package has none.
    pub name: String,
    /// `<member dir>/**`.
    pub pattern: String,
    /// Package description, when any.
    pub description: String,
    /// The manifest that declares it.
    pub source: WorkspaceKind,
    /// Set for the root files and the CI of the project, which are structure
    /// too but no workspace member.
    pub infra: Option<InfraKind>,
    /// Patterns after `pattern` (only for an infrastructure component).
    pub more_patterns: Vec<String>,
    /// Aliases written out (only for an infrastructure component).
    pub aliases: Vec<String>,
    /// Names of the dependencies its manifest declares (all tables, `target.*`
    /// ones included; the real package name of a renamed dependency).
    pub dependencies: Vec<String>,
}

impl DeclaredComponent {
    /// The manifest file of the member, relative to the project.
    pub fn manifest(&self) -> String {
        let dir = self.pattern.trim_end_matches("/**");
        match self.source {
            WorkspaceKind::Cargo => format!("{dir}/Cargo.toml"),
            WorkspaceKind::Npm | WorkspaceKind::Pnpm => format!("{dir}/package.json"),
        }
    }
}

/// Every member the manifests at `root` declare, Cargo first, by path.
pub fn declared_components(root: &Path) -> Vec<DeclaredComponent> {
    let mut found: Vec<DeclaredComponent> = Vec::new();
    let mut push = |component: DeclaredComponent| {
        if !found.iter().any(|known| known.pattern == component.pattern) {
            found.push(component);
        }
    };
    if let Ok(text) = std::fs::read_to_string(root.join("Cargo.toml")) {
        for member in cargo_members(&text) {
            for dir in expand(root, &member) {
                let manifest = std::fs::read_to_string(root.join(&dir).join("Cargo.toml"));
                let Ok(manifest) = manifest else {
                    continue;
                };
                push(DeclaredComponent {
                    name: toml_package_field(&manifest, "name")
                        .unwrap_or_else(|| folder_name(&dir)),
                    pattern: format!("{dir}/**"),
                    description: toml_package_field(&manifest, "description").unwrap_or_default(),
                    source: WorkspaceKind::Cargo,
                    infra: None,
                    more_patterns: Vec::new(),
                    aliases: Vec::new(),
                    dependencies: cargo_dependencies(&manifest),
                });
            }
        }
    }
    let mut node_patterns: Vec<(String, WorkspaceKind)> = Vec::new();
    if let Ok(text) = std::fs::read_to_string(root.join("package.json")) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
            let list = value
                .get("workspaces")
                .and_then(|workspaces| workspaces.get("packages").or(Some(workspaces)))
                .and_then(serde_json::Value::as_array);
            for pattern in list.into_iter().flatten() {
                if let Some(pattern) = pattern.as_str() {
                    node_patterns.push((pattern.to_string(), WorkspaceKind::Npm));
                }
            }
        }
    }
    if let Ok(text) = std::fs::read_to_string(root.join("pnpm-workspace.yaml")) {
        for pattern in pnpm_packages(&text) {
            node_patterns.push((pattern, WorkspaceKind::Pnpm));
        }
    }
    for (pattern, source) in node_patterns {
        for dir in expand(root, &pattern) {
            let Ok(manifest) = std::fs::read_to_string(root.join(&dir).join("package.json")) else {
                continue;
            };
            let value: serde_json::Value = serde_json::from_str(&manifest).unwrap_or_default();
            let field = |key: &str| {
                value
                    .get(key)
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            };
            push(DeclaredComponent {
                name: field("name").unwrap_or_else(|| folder_name(&dir)),
                pattern: format!("{dir}/**"),
                description: field("description").unwrap_or_default(),
                source,
                infra: None,
                more_patterns: Vec::new(),
                aliases: Vec::new(),
                dependencies: npm_dependencies(&value),
            });
        }
    }
    found
}

/// CI systems: where their pipelines live, how people call them, and whether
/// the path is a folder (`dir/**`) or a file.
const CI_SYSTEMS: &[(&str, &str, bool)] = &[
    (".github/workflows", "GitHub Actions", true),
    (".gitlab-ci.yml", "GitLab CI", false),
    (".circleci", "CircleCI", true),
    ("azure-pipelines.yml", "Azure Pipelines", false),
    ("Jenkinsfile", "Jenkins", false),
    (".buildkite", "Buildkite", true),
];

/// The structure of the project that no workspace member covers: the files
/// at the root of a workspace (its manifest, toolchain, shared configuration)
/// and the continuous integration pipelines. The root only exists when the
/// project declares `members`; the folder of a pipeline must exist.
pub fn infrastructure_components(
    root: &Path,
    members: &[DeclaredComponent],
) -> Vec<DeclaredComponent> {
    let infra = |name: &str,
                 kind: InfraKind,
                 patterns: Vec<String>,
                 description: String,
                 aliases: Vec<&str>| {
        let mut patterns = patterns.into_iter();
        DeclaredComponent {
            name: name.to_string(),
            pattern: patterns.next().unwrap_or_default(),
            description,
            source: members
                .first()
                .map_or(WorkspaceKind::Cargo, |first| first.source),
            infra: Some(kind),
            more_patterns: patterns.collect(),
            aliases: aliases.into_iter().map(str::to_string).collect(),
            dependencies: Vec::new(),
        }
    };
    let mut found = Vec::new();
    if !members.is_empty() {
        let taken = members
            .iter()
            .any(|member| entity_key(&member.name) == entity_key("workspace"));
        let mut patterns = vec!["*".to_string()];
        if root.join(".cargo").is_dir() {
            patterns.push(".cargo/**".to_string());
        }
        found.push(infra(
            if taken { "workspace-root" } else { "workspace" },
            InfraKind::Root,
            patterns,
            "Arquivos da raiz: manifesto do workspace, toolchain e configuração comum".into(),
            vec!["workspace root", "raiz do workspace"],
        ));
    }
    let systems: Vec<&(&str, &str, bool)> = CI_SYSTEMS
        .iter()
        .filter(|(path, _, folder)| {
            let found = root.join(path);
            if *folder {
                found.is_dir()
            } else {
                found.is_file()
            }
        })
        .collect();
    for (path, provider, folder) in &systems {
        let pattern = if *folder {
            format!("{path}/**")
        } else {
            (*path).to_string()
        };
        let name = if systems.len() == 1 {
            "CI".to_string()
        } else {
            format!("CI {provider}")
        };
        found.push(infra(
            &name,
            InfraKind::Ci,
            vec![pattern.clone()],
            format!("Integração contínua e publicação ({pattern})"),
            vec![provider],
        ));
    }
    found
}

/// Tables of a Cargo manifest that list dependencies.
const CARGO_DEPENDENCY_TABLES: &[&str] =
    &["dependencies", "dev-dependencies", "build-dependencies"];

/// Dependency names of a Cargo manifest: the three tables and the same three
/// under each `target.<cfg>`. A renamed dependency (`package = "x"`) counts as
/// `x`, the name people write.
fn cargo_dependencies(text: &str) -> Vec<String> {
    let Ok(manifest) = text.parse::<toml::Table>() else {
        return Vec::new();
    };
    let mut names: Vec<String> = Vec::new();
    let mut collect = |section: Option<&toml::Value>| {
        let Some(table) = section.and_then(toml::Value::as_table) else {
            return;
        };
        for (key, value) in table {
            let real = value
                .as_table()
                .and_then(|entry| entry.get("package"))
                .and_then(toml::Value::as_str)
                .unwrap_or(key);
            if !names.iter().any(|known| known == real) {
                names.push(real.to_string());
            }
        }
    };
    for kind in CARGO_DEPENDENCY_TABLES {
        collect(manifest.get(*kind));
    }
    if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
        for target in targets.values() {
            for kind in CARGO_DEPENDENCY_TABLES {
                collect(target.get(*kind));
            }
        }
    }
    names
}

/// Dependency names of a `package.json`.
fn npm_dependencies(manifest: &serde_json::Value) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for kind in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        let Some(table) = manifest.get(kind).and_then(serde_json::Value::as_object) else {
            continue;
        };
        for key in table.keys() {
            if !names.contains(key) {
                names.push(key.clone());
            }
        }
    }
    names
}

/// The names the project itself answers to: its folder, the `[package].name`
/// of the root `Cargo.toml` and the `name` of the root `package.json` (with
/// its last segment, `@acme/shop` -> `shop`). A part named like one of them
/// cannot be told from the product by its name alone. Names under 3
/// characters are left out; they never match anyway.
pub fn project_names(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut push = |name: String| {
        let name = name.trim().to_string();
        let known = names
            .iter()
            .any(|other| other.to_lowercase() == name.to_lowercase());
        if entity_key(&name).chars().count() >= 3 && !known {
            names.push(name);
        }
    };
    if let Some(folder) = root.file_name() {
        push(folder.to_string_lossy().to_string());
    }
    if let Ok(text) = std::fs::read_to_string(root.join("Cargo.toml")) {
        if let Some(name) = toml_package_field(&text, "name") {
            push(name);
        }
    }
    if let Ok(text) = std::fs::read_to_string(root.join("package.json")) {
        let name = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|value| value.get("name")?.as_str().map(str::to_string));
        if let Some(name) = name {
            push(name.rsplit('/').next().unwrap_or("").to_string());
            push(name);
        }
    }
    names
}

/// The keys of [`project_names`], kept for a few minutes so a context pack or
/// a refresh does not read the manifests again.
pub(crate) fn project_keys(root: &Path) -> Arc<BTreeSet<String>> {
    static KEYS: OnceLock<FolderCache<BTreeSet<String>>> = OnceLock::new();
    KEYS.get_or_init(FolderCache::new).get_or_make(root, || {
        project_names(root)
            .iter()
            .map(|name| entity_key(name))
            .collect()
    })
}

/// How people write the package in prose: the last segment of its name
/// (`@acme/opencode-adapter` → `opencode-adapter`), its spaced form
/// (`opencode adapter`) and the folder of its pattern (`packages/plugin/**` →
/// `plugin`), minus what equals the name. The length rules of the mention
/// matcher apply later; terms that can never match (under 3 characters) are
/// not worth storing.
fn alias_candidates(component: &DeclaredComponent) -> Vec<String> {
    if component.infra.is_some() {
        return component.aliases.clone();
    }
    let last = component.name.rsplit('/').next().unwrap_or("");
    let folder = component
        .pattern
        .trim_end_matches("/**")
        .rsplit('/')
        .next()
        .unwrap_or("");
    let mut found: Vec<String> = Vec::new();
    // Rust code spells a crate `sc_core`, its package `sc-core`.
    let underscored = (component.source == WorkspaceKind::Cargo)
        .then(|| component.name.replace('-', "_"))
        .filter(|form| *form != component.name);
    for term in [last, folder] {
        for form in [term.to_string(), term.replace(['-', '_'], " ")] {
            let usable = !form.starts_with('@') && entity_key(&form).chars().count() >= 3;
            let known = |other: &str| other.to_lowercase() == form.to_lowercase();
            if usable && !known(&component.name) && !found.iter().any(|other| known(other)) {
                found.push(form);
            }
        }
    }
    if let Some(form) = underscored {
        if !found
            .iter()
            .any(|other| other.to_lowercase() == form.to_lowercase())
        {
            found.push(form);
        }
    }
    found
}

/// The aliases each declared component gains: its candidates minus any whose
/// key another declared name, another candidate or `taken` (what other live
/// entities already answer to) also uses, since an ambiguous term must link
/// to nothing. Aligned with `declared`.
fn package_aliases(declared: &[DeclaredComponent], taken: &[BTreeSet<String>]) -> Vec<Vec<String>> {
    let candidates: Vec<Vec<String>> = declared.iter().map(alias_candidates).collect();
    let mut uses: BTreeMap<String, usize> = BTreeMap::new();
    for (component, own) in declared.iter().zip(&candidates) {
        let keys: BTreeSet<String> = own
            .iter()
            .map(|alias| entity_key(alias))
            .chain([entity_key(&component.name)])
            .collect();
        for key in keys {
            *uses.entry(key).or_default() += 1;
        }
    }
    candidates
        .into_iter()
        .zip(taken)
        .zip(declared)
        .map(|((own, taken), component)| {
            let name_key = entity_key(&component.name);
            own.into_iter()
                .filter(|alias| {
                    let key = entity_key(alias);
                    key == name_key || (uses.get(&key) == Some(&1) && !taken.contains(&key))
                })
                .collect()
        })
        .collect()
}

fn folder_name(dir: &str) -> String {
    dir.rsplit('/').next().unwrap_or(dir).to_string()
}

/// Directories a workspace pattern names, relative with `/`: an exact
/// folder, or the subfolders of `dir/*` (and `dir/**`). Exclusions (`!x`)
/// and anything leaving the root are ignored.
fn expand(root: &Path, pattern: &str) -> Vec<String> {
    let pattern = pattern
        .trim()
        .trim_start_matches("./")
        .trim_end_matches('/')
        .replace('\\', "/");
    if pattern.is_empty() || pattern.starts_with('!') || pattern.contains("..") {
        return Vec::new();
    }
    let prefix = pattern
        .strip_suffix("/**")
        .or_else(|| pattern.strip_suffix("/*"));
    match prefix {
        Some(prefix) if !prefix.contains('*') => {
            let Ok(entries) = std::fs::read_dir(root.join(prefix)) else {
                return Vec::new();
            };
            let mut dirs: Vec<String> = entries
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .filter(|name| !name.starts_with('.'))
                .map(|name| format!("{prefix}/{name}"))
                .collect();
            dirs.sort();
            dirs.truncate(MAX_MEMBERS);
            dirs
        }
        None if !pattern.contains('*') && root.join(&pattern).is_dir() => vec![pattern],
        _ => Vec::new(),
    }
}

/// The `members` array of the `[workspace]` table of a Cargo manifest.
fn cargo_members(text: &str) -> Vec<String> {
    let mut in_workspace = false;
    let mut collecting = false;
    let mut members = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if !collecting && line.starts_with('[') {
            in_workspace = line == "[workspace]";
            continue;
        }
        if !in_workspace {
            continue;
        }
        let rest = if collecting {
            line
        } else if let Some((key, value)) = line.split_once('=') {
            if key.trim() != "members" {
                continue;
            }
            collecting = true;
            value.trim().trim_start_matches('[')
        } else {
            continue;
        };
        members.extend(quoted(rest));
        if rest.contains(']') {
            collecting = false;
        }
    }
    members
}

/// A string field of the `[package]` table (`name = "x"`).
fn toml_package_field(text: &str, key: &str) -> Option<String> {
    let mut in_package = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim() == key {
            return quoted(value).into_iter().next();
        }
    }
    None
}

/// The `packages` list of `pnpm-workspace.yaml`.
fn pnpm_packages(text: &str) -> Vec<String> {
    let mut in_packages = false;
    let mut packages = Vec::new();
    for line in text.lines() {
        if !line.starts_with(' ') && !line.starts_with('-') {
            in_packages = line.trim_end() == "packages:";
            continue;
        }
        if let (true, Some(item)) = (in_packages, line.trim().strip_prefix('-')) {
            let item = item.trim().trim_matches(['"', '\'']);
            if !item.is_empty() {
                packages.push(item.to_string());
            }
        }
    }
    packages
}

/// Every `"…"` or `'…'` string on a line.
fn quoted(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(['"', '\'']) {
        let mark = rest[start..].chars().next().unwrap_or('"');
        let tail = &rest[start + 1..];
        let Some(end) = tail.find(mark) else {
            break;
        };
        found.push(tail[..end].to_string());
        rest = &tail[end + 1..];
    }
    found
}

impl<S> KnowledgeGraph<S>
where
    S: GraphStore + RelationStore + ClaimStore + ProjectRepository,
{
    /// Assembles an empty map from what the project declares: when no
    /// component exists yet, every declared workspace member becomes one,
    /// with its package name, description and path pattern. Returns how many
    /// were created; a map that already has components is left alone (new
    /// members appear as proposals).
    ///
    /// # Errors
    ///
    /// `project_not_found` or `storage`.
    pub fn assemble(&self, project_id: &str) -> Result<usize, GraphError> {
        let project =
            ProjectRepository::get(&self.store, project_id)?.ok_or(GraphError::ProjectNotFound)?;
        let has_components = self
            .store
            .project_entities(project_id)?
            .iter()
            .any(|entity| entity.kind == EntityKind::Component);
        if has_components {
            return Ok(0);
        }
        let mut created = 0;
        let root = Path::new(&project.location);
        let mut declared = declared_components(root);
        let infrastructure = infrastructure_components(root, &declared);
        declared.extend(infrastructure);
        let aliases = package_aliases(&declared, &vec![BTreeSet::new(); declared.len()]);
        for (declared, aliases) in declared.iter().cloned().zip(aliases) {
            let description: String = declared
                .description
                .chars()
                .take(MAX_DESCRIPTION_CHARS)
                .collect();
            match self.create_entity(NewEntity {
                project_id: project_id.to_string(),
                kind: Some(EntityKind::Component),
                name: declared.name,
                description,
                patterns: std::iter::once(declared.pattern)
                    .chain(declared.more_patterns)
                    .collect(),
                aliases,
            }) {
                Ok(_) => created += 1,
                Err(GraphError::DuplicateName | GraphError::Invalid(_)) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(created)
    }

    /// Gives the components already on the map the aliases their package
    /// earns (see [`alias_candidates`]), keeping the ones they have. A
    /// component is matched to its package by pattern. Idempotent: returns
    /// how many components gained aliases, 0 once they are all there.
    ///
    /// # Errors
    ///
    /// `project_not_found` or `storage`.
    pub fn merge_package_aliases(&self, project_id: &str) -> Result<usize, GraphError> {
        let project =
            ProjectRepository::get(&self.store, project_id)?.ok_or(GraphError::ProjectNotFound)?;
        let declared = declared_components(Path::new(&project.location));
        self.merge_aliases(project_id, &declared)
    }

    /// [`Self::merge_package_aliases`] over members already read, so a refresh
    /// reads the manifests once.
    pub(super) fn merge_aliases(
        &self,
        project_id: &str,
        declared: &[DeclaredComponent],
    ) -> Result<usize, GraphError> {
        let live: Vec<_> = self
            .store
            .project_entities(project_id)?
            .into_iter()
            .filter(|entity| entity.retired_at.is_none())
            .collect();
        let matched: Vec<Option<&_>> = declared
            .iter()
            .map(|component| {
                live.iter().find(|entity| {
                    entity.kind == EntityKind::Component
                        && entity.patterns.contains(&component.pattern)
                })
            })
            .collect();
        let taken: Vec<BTreeSet<String>> = matched
            .iter()
            .map(|own| {
                live.iter()
                    .filter(|entity| own.is_none_or(|own| own.entity_id != entity.entity_id))
                    .flat_map(|entity| entity.keys())
                    .collect()
            })
            .collect();
        let mut updated = 0;
        for (entity, aliases) in matched.into_iter().zip(package_aliases(declared, &taken)) {
            let Some(entity) = entity else {
                continue;
            };
            let mut merged = entity.aliases.clone();
            for alias in aliases {
                if !merged_has(&merged, &alias) {
                    merged.push(alias);
                }
            }
            if merged.len() == entity.aliases.len() || merged.len() > MAX_ENTITY_LIST {
                continue;
            }
            let edit = EntityEdit {
                name: entity.name.clone(),
                description: entity.description.clone(),
                patterns: entity.patterns.clone(),
                aliases: merged,
            };
            match self.update_entity(&entity.entity_id, edit) {
                Ok(_) => updated += 1,
                Err(GraphError::DuplicateName | GraphError::Invalid(_) | GraphError::Conflict) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(updated)
    }
}

fn merged_has(aliases: &[String], alias: &str) -> bool {
    aliases
        .iter()
        .any(|known| known.to_lowercase() == alias.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_cargo_members_across_lines() {
        let text = "[package]\nname = \"root\"\n\n[workspace]\nresolver = \"2\"\n\
                    members = [\n    \"apps/desktop\", # the app\n    \"crates/*\",\n]\n\
                    [workspace.package]\nversion = \"0.1.0\"\n";
        assert_eq!(cargo_members(text), vec!["apps/desktop", "crates/*"]);
        assert_eq!(toml_package_field(text, "name").as_deref(), Some("root"));
        assert_eq!(
            cargo_members("[workspace]\nmembers = [\"a\", 'b']\n"),
            vec!["a", "b"]
        );
    }

    #[test]
    fn the_project_answers_to_its_folder_and_its_root_manifests() {
        let base = std::env::temp_dir().join(format!("xemnas-names-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let make = |folder: &str, files: &[(&str, &str)]| {
            let root = base.join(folder);
            std::fs::create_dir_all(&root).unwrap();
            for (name, text) in files {
                std::fs::write(root.join(name), text).unwrap();
            }
            root
        };
        // A Cargo workspace root has no package: only the folder.
        let workspace = make(
            "acme",
            &[("Cargo.toml", "[workspace]\nmembers = [\"apps/acme\"]\n")],
        );
        assert_eq!(project_names(&workspace), vec!["acme"]);
        // A package at the root adds its name.
        let package = make(
            "checkout",
            &[("Cargo.toml", "[package]\nname = \"shop-cli\"\n")],
        );
        assert_eq!(project_names(&package), vec!["checkout", "shop-cli"]);
        // A scoped npm name adds the whole name and its last segment.
        let npm = make(
            "mono",
            &[(
                "package.json",
                "{\"name\": \"@acme/shop\", \"private\": true}",
            )],
        );
        assert_eq!(project_names(&npm), vec!["mono", "shop", "@acme/shop"]);
        // A name that repeats the folder, or is under 3 characters, is not kept.
        let pnpm = make(
            "ui",
            &[
                ("package.json", "{\"name\": \"ui\"}"),
                ("pnpm-workspace.yaml", "packages:\n  - 'packages/*'\n"),
            ],
        );
        assert!(project_names(&pnpm).is_empty());
        let keys = project_keys(&npm);
        assert!(keys.contains("mono") && keys.contains("shop") && keys.contains("acmeshop"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn reads_pnpm_packages() {
        let text = "packages:\n  - 'packages/*'\n  - \"apps/web\"\n  - '!**/test/**'\nother: 1\n";
        assert_eq!(
            pnpm_packages(text),
            vec!["packages/*", "apps/web", "!**/test/**"]
        );
    }

    #[test]
    fn expands_declared_members_from_the_folder() {
        let root = std::env::temp_dir().join(format!("xemnas-discover-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in ["crates/core", "crates/api", "apps/web", "apps/api", "docs"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\", \"missing\"]\n",
        )
        .unwrap();
        std::fs::write(
            root.join("crates/core/Cargo.toml"),
            "[package]\nname = \"core\"\ndescription = \"Regras do domínio.\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("crates/api/Cargo.toml"),
            "[package]\nname = \"api-rs\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("package.json"),
            "{\"workspaces\": {\"packages\": [\"apps/*\"]}}",
        )
        .unwrap();
        std::fs::write(
            root.join("apps/web/package.json"),
            "{\"name\": \"@acme/web\", \"description\": \"Site.\"}",
        )
        .unwrap();
        std::fs::write(root.join("apps/api/package.json"), "{}").unwrap();

        let found = declared_components(&root);
        let summary: Vec<(&str, &str, WorkspaceKind)> = found
            .iter()
            .map(|component| {
                (
                    component.name.as_str(),
                    component.pattern.as_str(),
                    component.source,
                )
            })
            .collect();
        assert_eq!(
            summary,
            vec![
                ("api-rs", "crates/api/**", WorkspaceKind::Cargo),
                ("core", "crates/core/**", WorkspaceKind::Cargo),
                ("api", "apps/api/**", WorkspaceKind::Npm),
                ("@acme/web", "apps/web/**", WorkspaceKind::Npm),
            ]
        );
        assert_eq!(found[1].description, "Regras do domínio.");
        assert!(alias_candidates(&found[1]).is_empty(), "equal to the name");
        let _ = std::fs::remove_dir_all(&root);
    }

    fn declared(name: &str, dir: &str) -> DeclaredComponent {
        DeclaredComponent {
            name: name.into(),
            pattern: format!("{dir}/**"),
            description: String::new(),
            source: WorkspaceKind::Npm,
            infra: None,
            more_patterns: Vec::new(),
            aliases: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn packages_earn_the_aliases_people_write() {
        let adapter = declared("@jevguard/opencode-adapter", "packages/opencode-adapter");
        assert_eq!(
            alias_candidates(&adapter),
            vec!["opencode-adapter", "opencode adapter"]
        );
        let plugin = declared("@pablozrrrr/jevguard", "packages/plugin");
        assert_eq!(alias_candidates(&plugin), vec!["jevguard", "plugin"]);
        let crate_ = declared("storage_sqlite", "crates/storage_sqlite");
        assert_eq!(alias_candidates(&crate_), vec!["storage sqlite"]);
        assert!(alias_candidates(&declared("ui", "apps/ui")).is_empty());
    }

    #[test]
    fn a_cargo_package_is_also_written_with_underscores() {
        let mut core = declared("sc-core", "crates/sc-core");
        core.source = WorkspaceKind::Cargo;
        assert_eq!(alias_candidates(&core), vec!["sc core", "sc_core"]);
        // An npm package has no such spelling.
        let web = declared("sc-core", "crates/sc-core");
        assert_eq!(alias_candidates(&web), vec!["sc core"]);
    }

    #[test]
    fn manifests_list_their_dependencies() {
        let cargo = "[package]\nname = \"x\"\n\n[dependencies]\nquinn = \"0.11\"\n\
                     serde = { version = \"1\", features = [\"derive\"] }\n\
                     tls = { package = \"rustls\", version = \"0.23\" }\n\n\
                     [dev-dependencies]\ntokio = \"1\"\n\n[build-dependencies]\ncc = \"1\"\n\n\
                     [target.'cfg(windows)'.dependencies]\nwindows-sys = \"0.59\"\n\n\
                     [dependencies.iroh]\nversion = \"0.9\"\n";
        let mut found = cargo_dependencies(cargo);
        found.sort();
        assert_eq!(
            found,
            vec![
                "cc",
                "iroh",
                "quinn",
                "rustls",
                "serde",
                "tokio",
                "windows-sys"
            ]
        );
        let npm: serde_json::Value = serde_json::from_str(
            "{\"dependencies\": {\"fastify\": \"^5\"}, \"devDependencies\": {\"vitest\": \"1\"}, \
             \"peerDependencies\": {\"react\": \"18\"}, \"optionalDependencies\": {\"fsevents\": \"2\"}, \
             \"scripts\": {\"build\": \"tsc\"}}",
        )
        .unwrap();
        let mut found = npm_dependencies(&npm);
        found.sort();
        assert_eq!(found, vec!["fastify", "fsevents", "react", "vitest"]);
    }

    fn tree(name: &str, files: &[&str]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("xemnas-infra-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for file in files {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "x").unwrap();
        }
        root
    }

    #[test]
    fn a_workspace_has_a_root_component_and_one_per_ci_system() {
        let root = tree(
            "both",
            &[
                ".github/workflows/ci.yml",
                ".cargo/config.toml",
                "Jenkinsfile",
            ],
        );
        let members = vec![declared("core", "crates/core")];
        let found = infrastructure_components(&root, &members);
        let names: Vec<(&str, &str, InfraKind)> = found
            .iter()
            .map(|component| {
                (
                    component.name.as_str(),
                    component.pattern.as_str(),
                    component.infra.unwrap(),
                )
            })
            .collect();
        assert_eq!(
            names,
            vec![
                ("workspace", "*", InfraKind::Root),
                ("CI GitHub Actions", ".github/workflows/**", InfraKind::Ci),
                ("CI Jenkins", "Jenkinsfile", InfraKind::Ci),
            ]
        );
        assert_eq!(found[0].more_patterns, vec![".cargo/**"]);
        assert_eq!(
            found[0].aliases,
            vec!["workspace root", "raiz do workspace"]
        );
        assert_eq!(found[1].aliases, vec!["GitHub Actions"]);
        assert!(found[1].description.contains(".github/workflows/**"));
        // One system is just "CI"; a member called workspace takes the name.
        let single = tree("single", &[".gitlab-ci.yml"]);
        let taken = vec![declared("workspace", "crates/workspace")];
        let found = infrastructure_components(&single, &taken);
        let names: Vec<&str> = found
            .iter()
            .map(|component| component.name.as_str())
            .collect();
        assert_eq!(names, vec!["workspace-root", "CI"]);
        assert_eq!(found[1].pattern, ".gitlab-ci.yml");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&single);
    }

    #[test]
    fn a_project_without_members_has_no_root_component() {
        let root = tree("single-package", &["src/main.rs"]);
        assert!(infrastructure_components(&root, &[]).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ambiguous_aliases_are_dropped() {
        let web = declared("@a/shared", "apps/web");
        let api = declared("@b/shared", "apps/api");
        let named = declared("web", "apps/other");
        let none = BTreeSet::new();
        let taken = BTreeSet::from(["api".to_string()]);
        let aliases = package_aliases(&[web, api, named], &[none.clone(), taken, none]);
        // `shared` is two packages' last segment; `web` is another's name;
        // `api` is already taken by another entity.
        assert_eq!(aliases[..2], [Vec::<String>::new(), Vec::new()]);
        assert_eq!(aliases[2], vec!["other"], "its own folder is unambiguous");
    }
}
