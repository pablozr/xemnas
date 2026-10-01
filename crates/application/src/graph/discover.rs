//! Components the project declares: members of a Cargo workspace, npm
//! `workspaces` and `pnpm-workspace.yaml` packages, read from the project
//! folder. A declared member is structure the team already wrote down, so an
//! empty map is assembled from it at once; folders inferred from captured
//! diffs stay proposals (ADR-0005, revision of 2026-10-01).

use std::path::Path;

use super::{GraphError, GraphStore, KnowledgeGraph, NewEntity};
use crate::claims::ClaimStore;
use crate::projects::ProjectRepository;
use crate::relations::RelationStore;
use domain::entities::{EntityKind, MAX_DESCRIPTION_CHARS};

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
            });
        }
    }
    found
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
        for declared in declared_components(Path::new(&project.location)) {
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
                patterns: vec![declared.pattern],
                aliases: Vec::new(),
            }) {
                Ok(_) => created += 1,
                Err(GraphError::DuplicateName | GraphError::Invalid(_)) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(created)
    }
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
        let _ = std::fs::remove_dir_all(&root);
    }
}
