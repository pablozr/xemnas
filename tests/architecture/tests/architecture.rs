//! Architecture guard tests.
//!
//! These integration tests enforce the dependency rules from ARCH-001. They run
//! as part of `cargo test --workspace` and read the workspace manifests and
//! sources directly, so a violating dependency edge, a `path` dependency that
//! escapes the workspace, or a GPUI/process/FFI reference in the wrong crate
//! fails the build without extra external tooling.
//!
//! Scope decision: the guard does **not** resolve the transitive dependency
//! graph with `cargo metadata`. A cargo-in-cargo invocation is slow,
//! environment sensitive and would hide violations behind a successful build.
//! The direct-dependency allow-list for `domain` plus the source traps below
//! cover the realistic escape hatches (AI SDKs, `std::process`, FFI).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

// The GPUI crate names are assembled at runtime so this guard's own source does
// not contain the substrings it forbids elsewhere.
const GPUI: &str = "gpui";
const GPUI_PLATFORM: &str = "gpui_platform";

/// Crates the domain must never know about (ARCH-001).
const DOMAIN_BANNED: &[&str] = &[
    GPUI,
    GPUI_PLATFORM,
    "rusqlite",
    "axum",
    "tower-http",
    "reqwest",
    "ureq",
    "openai",
    "async-openai",
];

/// HTTP crates only allowed at the `local-api` boundary (ARCH-001).
const HTTP_BANNED: &[&str] = &["axum", "tower-http", "reqwest", "ureq"];

/// Crates the `domain` crate is allowed to depend on directly.
///
/// Starts empty: the domain has no dependencies today. The list only grows when
/// the specification has already approved a crate (for example `serde`, `uuid`
/// or `thiserror`), and every addition must edit this guard on purpose. It is an
/// allow-list because a deny-list of known AI providers (`genai`, `ollama`,
/// `anthropic`, ...) can never be complete. The [`DOMAIN_BANNED`] deny-list
/// stays as a second, more descriptive layer.
const DOMAIN_ALLOWED_CRATES: &[&str] = &[];

/// Direct process/FFI escape hatches forbidden in the domain (ARCH-001).
const DOMAIN_FORBIDDEN_SOURCE_TOKENS: &[&str] =
    &["std::process", "process::Command", "extern \"C\"", "#[link"];

/// Dependency tables scanned at the top level and under every `[target.*]`.
const DEPENDENCY_TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

/// A workspace package together with its manifest dependencies.
struct Member {
    name: String,
    path: PathBuf,
    deps: Dependencies,
}

/// Locates the workspace root two levels above this package.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("tests/architecture lives two levels below the workspace root")
        .to_path_buf()
}

/// Reads and parses a TOML manifest.
fn read_manifest(path: &Path) -> toml::Value {
    let text =
        fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    text.parse::<toml::Value>()
        .unwrap_or_else(|err| panic!("parse {}: {err}", path.display()))
}

/// Manifest dependencies keyed by declared alias.
///
/// The alias maps to **every** declaration of that alias: the same name can be
/// declared under several `[target.'cfg(...)']` tables and each spec must be
/// examined, so a later declaration cannot shadow an earlier forbidden one.
type Dependencies = BTreeMap<String, Vec<toml::Value>>;

/// Merges one dependency table into `deps`, preserving every declaration and
/// `package = "..."` rename.
fn merge_dependency_table(table: Option<&toml::Value>, deps: &mut Dependencies) {
    if let Some(entries) = table.and_then(toml::Value::as_table) {
        for (name, spec) in entries {
            deps.entry(name.clone()).or_default().push(spec.clone());
        }
    }
}

/// Collects every dependency of a manifest, including target-specific tables.
///
/// Reads `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]` and the
/// same three tables under each `[target.'cfg(...)']`, so a dependency hidden
/// behind `[target.'cfg(windows)'.dependencies]` cannot escape the guard.
fn dependencies(manifest: &toml::Value) -> Dependencies {
    let mut deps = BTreeMap::new();
    for table in DEPENDENCY_TABLES {
        merge_dependency_table(manifest.get(table), &mut deps);
    }
    if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
        for target in targets.values() {
            for table in DEPENDENCY_TABLES {
                merge_dependency_table(target.get(table), &mut deps);
            }
        }
    }
    deps
}

/// Loads every workspace member listed by the root manifest.
fn members(root: &Path) -> Vec<Member> {
    let workspace = read_manifest(&root.join("Cargo.toml"));
    let dirs = workspace
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(toml::Value::as_array)
        .expect("workspace.members must be an array");

    dirs.iter()
        .map(|entry| {
            let dir = root.join(entry.as_str().expect("workspace member must be a string"));
            let manifest = read_manifest(&dir.join("Cargo.toml"));
            let name = manifest
                .get("package")
                .and_then(|package| package.get("name"))
                .and_then(toml::Value::as_str)
                .expect("member manifest must define package.name")
                .to_string();
            Member {
                name,
                path: fs::canonicalize(&dir).expect("canonicalize workspace member"),
                deps: dependencies(&manifest),
            }
        })
        .collect()
}

/// Result of resolving a member's `path = ...` dependencies.
#[derive(Debug, Default)]
struct PathDependencies {
    /// Workspace member names reached through resolvable `path` deps.
    targets: BTreeSet<String>,
    /// `(dependency name, declared path)` pairs that do not resolve to a member.
    unresolved: Vec<(String, String)>,
}

/// Resolves `path` dependencies against the registered member paths.
///
/// `canonicalize` is injected so the logic can be unit-tested without touching
/// the filesystem. A `path` dep that cannot be canonicalized, or that resolves
/// outside the registered members, is reported as unresolved instead of being
/// skipped silently.
fn resolve_path_dependencies(
    member_dir: &Path,
    deps: &Dependencies,
    canonicalize: impl Fn(&Path) -> Option<PathBuf>,
    member_paths: &BTreeMap<PathBuf, String>,
) -> PathDependencies {
    let mut result = PathDependencies::default();
    for (name, specs) in deps {
        for spec in specs {
            let Some(relative) = spec.get("path").and_then(toml::Value::as_str) else {
                continue;
            };
            match canonicalize(&member_dir.join(relative)) {
                Some(canonical) => match member_paths.get(&canonical) {
                    Some(target) => {
                        result.targets.insert(target.clone());
                    }
                    None => result.unresolved.push((name.clone(), relative.to_string())),
                },
                None => result.unresolved.push((name.clone(), relative.to_string())),
            }
        }
    }
    result
}

/// Resolves a member's `path` dependencies against the loaded workspace.
fn path_dependencies(member: &Member, all: &[Member]) -> PathDependencies {
    let member_paths: BTreeMap<PathBuf, String> = all
        .iter()
        .map(|candidate| (candidate.path.clone(), candidate.name.clone()))
        .collect();
    resolve_path_dependencies(
        &member.path,
        &member.deps,
        |path| fs::canonicalize(path).ok(),
        &member_paths,
    )
}

/// The dependency targets each workspace member is allowed to have.
fn allowed_dependencies(name: &str) -> BTreeSet<&'static str> {
    let allowed: &[&str] = match name {
        "domain" => &[],
        // The ingest use case consumes the versioned Capture Envelope contract
        // directly: the contract is the language of the ingest boundary, and
        // duplicating its shape into a command struct would drift (ticket 09).
        "application" => &["domain", "integration-contracts"],
        "integration-contracts" => &["domain"],
        "storage-sqlite" => &["application", "domain"],
        // `storage-sqlite` is a dev-dependency only: the integration tests mount
        // the real use case on a real store, exactly like `main.rs` composes
        // them at runtime. The library itself stays storage-agnostic.
        "local-api" => &[
            "application",
            "domain",
            "integration-contracts",
            "storage-sqlite",
        ],
        "telemetry" => &[],
        // The desktop app is the composition root: `main.rs` opens the SQLite
        // store and mounts it into the application use case before handing the
        // port to the view. The UI never sees SQL, only the `Projects` use case;
        // this edge is the documented "storage-sqlite implements the
        // interfaces" wiring from the stack doc (Regra de dependência).
        // `local-api` is started from `main.rs` on its own runtime thread so the
        // API lives and dies with the desktop process (MVP-SPEC §9).
        "desktop-gpui" => &[
            "application",
            "domain",
            "telemetry",
            "storage-sqlite",
            "local-api",
        ],
        "architecture" => &[],
        other => {
            panic!("unknown workspace member `{other}`; register it in the architecture guard")
        }
    };
    allowed.iter().copied().collect()
}

#[test]
fn only_allowed_dependency_edges() {
    let root = workspace_root();
    let all = members(&root);
    let mut violations = Vec::new();
    for member in &all {
        let allowed = allowed_dependencies(&member.name);
        let resolved = path_dependencies(member, &all);
        for (name, path) in &resolved.unresolved {
            violations.push(format!(
                "ARCH-001: `{}` has path dependency `{name}` = \"{path}\" that does not resolve to a workspace member",
                member.name
            ));
        }
        for target in &resolved.targets {
            if !allowed.contains(target.as_str()) {
                violations.push(format!(
                    "ARCH-001: `{}` must not depend on `{}` through a path dependency",
                    member.name, target
                ));
            }
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// Asserts that a member does not depend on any of the banned crates.
fn assert_no_banned_dependency(member_name: &str, banned: &[&str]) {
    let root = workspace_root();
    let all = members(&root);
    let member = all
        .iter()
        .find(|candidate| candidate.name == member_name)
        .unwrap_or_else(|| panic!("unknown workspace member `{member_name}`"));
    let mut violations = Vec::new();
    for (key, specs) in &member.deps {
        for spec in specs {
            let package = spec
                .get("package")
                .and_then(toml::Value::as_str)
                .unwrap_or(key);
            for banned_crate in banned {
                if key == banned_crate || package == *banned_crate {
                    violations.push(format!(
                        "ARCH-001: `{member_name}` must not depend on `{banned_crate}`"
                    ));
                }
            }
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// Asserts that every direct dependency of a member is in the allow-list.
fn assert_allow_listed(member_name: &str, allowed: &[&str]) {
    let root = workspace_root();
    let all = members(&root);
    let member = all
        .iter()
        .find(|candidate| candidate.name == member_name)
        .unwrap_or_else(|| panic!("unknown workspace member `{member_name}`"));
    let mut violations = Vec::new();
    for (key, specs) in &member.deps {
        for spec in specs {
            let package = spec
                .get("package")
                .and_then(toml::Value::as_str)
                .unwrap_or(key);
            if !allowed.contains(&key.as_str()) && !allowed.contains(&package) {
                violations.push(format!(
                    "ARCH-001: `{member_name}` may not depend on `{key}`; the approved allow-list is {allowed:?}"
                ));
            }
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn domain_has_no_infrastructure_or_ui_dependencies() {
    assert_no_banned_dependency("domain", DOMAIN_BANNED);
}

#[test]
fn application_has_no_infrastructure_or_ui_dependencies() {
    assert_no_banned_dependency("application", DOMAIN_BANNED);
}

#[test]
fn domain_dependencies_are_allow_listed() {
    assert_allow_listed("domain", DOMAIN_ALLOWED_CRATES);
}

#[test]
fn domain_and_application_have_no_http_dependencies() {
    assert_no_banned_dependency("domain", HTTP_BANNED);
    assert_no_banned_dependency("application", HTTP_BANNED);
}

#[test]
fn gpui_dependency_confined_to_desktop_app() {
    let root = workspace_root();
    let all = members(&root);
    let mut violations = Vec::new();
    for member in &all {
        if member.name == "desktop-gpui" {
            continue;
        }
        for (key, specs) in &member.deps {
            for spec in specs {
                let package = spec
                    .get("package")
                    .and_then(toml::Value::as_str)
                    .unwrap_or(key);
                let is_gpui = key == GPUI || package == GPUI;
                let is_gpui_platform = key == GPUI_PLATFORM || package == GPUI_PLATFORM;
                if is_gpui || is_gpui_platform {
                    violations.push(format!(
                        "ARCH-001: `{}` must not depend on `{key}`",
                        member.name
                    ));
                }
            }
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn gpui_types_never_appear_in_sources_outside_desktop_app() {
    let root = workspace_root();
    let needles = [
        format!("use {GPUI}"),
        format!("{GPUI}::"),
        format!("{GPUI_PLATFORM}::"),
    ];
    let desktop_app = fs::canonicalize(root.join("apps/desktop-gpui")).expect("desktop app dir");

    let mut files = Vec::new();
    collect_rust_sources(&root, &mut files);

    let mut violations = Vec::new();
    for file in files {
        let canonical = fs::canonicalize(&file).unwrap_or_else(|_| file.clone());
        if canonical.starts_with(&desktop_app) {
            continue;
        }
        let source = fs::read_to_string(&file)
            .unwrap_or_else(|err| panic!("read {}: {err}", file.display()));
        for needle in &needles {
            if source.contains(needle.as_str()) {
                violations.push(format!(
                    "ARCH-001: `{}` references `{needle}` outside apps/desktop-gpui",
                    file.display()
                ));
            }
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn domain_sources_do_not_reach_outside_the_process() {
    let root = workspace_root();
    let mut files = Vec::new();
    collect_rust_sources(&root.join("crates/domain"), &mut files);

    let mut violations = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file)
            .unwrap_or_else(|err| panic!("read {}: {err}", file.display()));
        for (line_index, line) in source.lines().enumerate() {
            for token in DOMAIN_FORBIDDEN_SOURCE_TOKENS {
                if line.contains(*token) {
                    violations.push(format!(
                        "ARCH-001: {}:{} references `{token}`; the domain must not use processes or FFI",
                        file.display(),
                        line_index + 1
                    ));
                }
            }
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// Color literals are forbidden outside the token module.
///
/// Design system rule 3: views must never contain loose color values. This
/// guard scans `apps/desktop-gpui/src/**` (except `ui/tokens.rs`) for
/// `#RGB`/`#RRGGBB`/`#RRGGBBAA` literals, for `rgb`/`rgba`/`rgb8`/`rgba8`/
/// `hsl`/`hsla`/`hsba` constructors, and for `Color::rgb(...)`/`Rgba { ... }`
/// built from three or more numeric literals. `//` only starts a comment when
/// it is outside a string and preceded by whitespace or the line start.
#[test]
fn no_color_literals_outside_tokens() {
    let root = workspace_root();
    let source_dir = root.join("apps").join("desktop-gpui").join("src");
    let allowed = source_dir.join("ui").join("tokens.rs");

    let mut files = Vec::new();
    collect_rust_sources(&source_dir, &mut files);

    let mut violations = Vec::new();
    for file in files {
        if file == allowed {
            continue;
        }
        let source = fs::read_to_string(&file)
            .unwrap_or_else(|err| panic!("read {}: {err}", file.display()));
        violations.extend(color_literal_violations(&file, &source));
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// Returns every color-literal violation in `source`.
fn color_literal_violations(file: &Path, source: &str) -> Vec<String> {
    let mut violations = Vec::new();
    for (line_index, raw_line) in source.lines().enumerate() {
        let line = strip_line_comment(raw_line);
        if let Some(literal) = find_hex_color(line) {
            violations.push(format!(
                "ARCH-001: {}:{} uses hex color literal `{literal}`; colors belong in ui/tokens.rs",
                file.display(),
                line_index + 1
            ));
        }

        let mut flagged_constructor = false;
        for constructor in ["rgb(", "rgba(", "rgb8(", "rgba8(", "hsl(", "hsla(", "hsba("] {
            if line.contains(constructor) {
                flagged_constructor = true;
                violations.push(format!(
                    "ARCH-001: {}:{} builds a color with `{constructor}`; colors belong in ui/tokens.rs",
                    file.display(),
                    line_index + 1
                ));
            }
        }

        // `Rgba { r: 0.1, g: 0.2, b: 0.3 }` has no constructor substring, so it
        // is only flagged when it really carries three numeric literals.
        if !flagged_constructor {
            if let Some(marker) = numeric_color_constructor(line) {
                violations.push(format!(
                    "ARCH-001: {}:{} builds a color with `{marker}` from numeric literals; colors belong in ui/tokens.rs",
                    file.display(),
                    line_index + 1
                ));
            }
        }
    }
    violations
}

/// Strips a trailing `//` comment, but only when it is outside a string (an
/// even number of `"` before it) and preceded by whitespace or the line start.
fn strip_line_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index] == b'/' && bytes[index + 1] == b'/' {
            let quotes = bytes[..index].iter().filter(|byte| **byte == b'"').count();
            let at_line_start = index == 0;
            let preceded_by_space = index > 0 && (bytes[index - 1] as char).is_whitespace();
            if quotes % 2 == 0 && (at_line_start || preceded_by_space) {
                return &line[..index];
            }
        }
        index += 1;
    }
    line
}

/// Detects `Color::rgb(...)` / `Rgba { ... }` built from three or more numeric
/// literals.
fn numeric_color_constructor(line: &str) -> Option<&'static str> {
    for marker in ["Color::rgb(", "Rgba {"] {
        if let Some(position) = line.find(marker) {
            let arguments = &line[position + marker.len()..];
            if count_numeric_literals(arguments) >= 3 {
                return Some(marker);
            }
        }
    }
    None
}

/// Counts numeric literals (integer or float) in `text`.
fn count_numeric_literals(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut count = 0;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_digit() {
            count += 1;
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_digit() || bytes[index] == b'.' || bytes[index] == b'_')
            {
                index += 1;
            }
        } else {
            index += 1;
        }
    }
    count
}

/// Finds a `#` color literal with 3, 6 or 8 hex digits.
fn find_hex_color(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'#' {
            let start = index;
            let mut end = index + 1;
            while end < bytes.len() && bytes[end].is_ascii_hexdigit() {
                end += 1;
            }
            let length = end - start - 1;
            if matches!(length, 3 | 6 | 8) {
                return Some(line[start..end].to_string());
            }
        }
        index += 1;
    }
    None
}

/// Recursively collects `.rs` files, skipping build output and VCS metadata.
fn collect_rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if path.is_dir() {
            if file_name == "target" || file_name == ".git" {
                continue;
            }
            collect_rust_sources(&path, out);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::{dependencies, resolve_path_dependencies};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    fn parse(manifest: &str) -> toml::Value {
        manifest
            .parse::<toml::Value>()
            .expect("synthetic manifest must parse")
    }

    #[test]
    fn target_specific_dependencies_are_collected() {
        let manifest = parse(
            r#"
[package]
name = "synthetic"

[target.'cfg(windows)'.dependencies]
reqwest = "0.12"
renamed = { package = "rusqlite", version = "0.32" }

[target.'cfg(unix)'.dev-dependencies]
axum = "0.7"

[target.'cfg(windows)'.build-dependencies]
tower-http = "0.5"
"#,
        );

        let deps = dependencies(&manifest);
        assert!(deps.contains_key("reqwest"), "target dependency missed");
        assert!(
            deps.contains_key("renamed"),
            "renamed target dependency missed"
        );
        assert_eq!(
            deps["renamed"][0]
                .get("package")
                .and_then(toml::Value::as_str),
            Some("rusqlite"),
            "package rename must be preserved"
        );
        assert!(deps.contains_key("axum"), "target dev-dependency missed");
        assert!(
            deps.contains_key("tower-http"),
            "target build-dependency missed"
        );
    }

    #[test]
    fn same_alias_in_two_targets_keeps_every_declaration() {
        // The cfg strings are ordered so a last-wins `BTreeMap` overwrite would
        // drop the forbidden `reqwest` declaration, making the failure explicit.
        let manifest = parse(
            r#"
[target.'cfg(unix)'.dependencies]
provider = { package = "reqwest", version = "0.12" }

[target.'cfg(windows)'.dependencies]
provider = { package = "serde", version = "1" }
"#,
        );

        let deps = dependencies(&manifest);
        let packages: BTreeSet<&str> = deps["provider"]
            .iter()
            .filter_map(|spec| spec.get("package").and_then(toml::Value::as_str))
            .collect();
        assert!(
            packages.contains("reqwest"),
            "the forbidden package must survive an alias shared with another target: {packages:?}"
        );
        assert_eq!(packages, BTreeSet::from(["reqwest", "serde"]));
    }

    #[test]
    fn path_dependencies_outside_the_workspace_are_unresolved() {
        let manifest = parse(
            r#"
[dependencies]
inside = { path = "../inside" }
missing = { path = "../missing" }
outside = { path = "../../../ai-provider" }
plain = "1.0"
"#,
        );
        let deps = dependencies(&manifest);

        let member_paths: BTreeMap<PathBuf, String> =
            [(PathBuf::from("fake/inside"), "inside".to_string())]
                .into_iter()
                .collect();

        let canonicalize = |path: &Path| {
            let text = path.to_string_lossy().replace('\\', "/");
            if text.ends_with("/inside") {
                Some(PathBuf::from("fake/inside"))
            } else if text.ends_with("/ai-provider") {
                Some(PathBuf::from("fake/ai-provider"))
            } else {
                None
            }
        };

        let resolved = resolve_path_dependencies(
            Path::new("C:/ws/crates/domain"),
            &deps,
            canonicalize,
            &member_paths,
        );

        assert!(resolved.targets.contains("inside"));
        let unresolved: BTreeSet<&str> = resolved
            .unresolved
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(
            unresolved,
            BTreeSet::from(["missing", "outside"]),
            "unresolvable and out-of-workspace path deps must be reported"
        );
    }
}
