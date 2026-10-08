//! The files of the project folder, as far as the map needs them: which cited
//! path exists, which file a partial path names, and which component a path
//! belongs to. ADR-0005 left the repository tree out of the derivation; ADR-0016
//! brings back the smallest piece of it that removes ghosts: a listing of the
//! file names (not their content), read on demand, cached for a few minutes,
//! bounded in size and never read on the interface thread.
//!
//! The listing comes from `git ls-files` (tracked files and untracked files
//! that are not ignored); without git, a walk that skips build output and
//! dependency folders.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use domain::entities::normalize_path;

use super::cache::FolderCache;
use super::mention::{path_term, Folded};
use crate::documents::is_documentation_path;

/// Files kept, at most; a larger project is `partial`.
pub(crate) const MAX_REPO_FILES: usize = 100_000;
/// How long the listing may take.
const LISTING_TIMEOUT: Duration = Duration::from_secs(3);
/// Bytes of `git ls-files` output kept (the rest is read and dropped).
const MAX_GIT_OUTPUT: usize = 32 * 1024 * 1024;
/// Folders the walk never enters.
const SKIPPED_FOLDERS: &[&str] = &["target", "node_modules", "dist", "build", "vendor", ".venv"];
/// Hidden folders the walk enters: CI and tool configuration are part of the
/// project's structure.
const KEPT_HIDDEN_FOLDERS: &[&str] = &[".github", ".gitlab", ".circleci", ".buildkite", ".cargo"];

/// What a cited path names in the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Resolved {
    /// The path itself exists.
    Exact(String),
    /// One file ends with the cited path (`examples/chat.rs` cited inside the
    /// crate that holds it).
    Unique(String),
    /// Several files end with it.
    Ambiguous,
    /// No file does.
    Missing,
}

/// The listing of a project folder.
#[derive(Debug)]
pub(crate) struct RepoFiles {
    files: Vec<String>,
    /// Lowercase path -> position.
    exact: HashMap<String, usize>,
    /// Lowercase file name -> positions.
    by_name: HashMap<String, Vec<usize>>,
    /// The listing was cut: a path missing from it may still exist.
    partial: bool,
    /// Definitions found in the code, read on first use.
    definitions: OnceLock<super::symbols::Definitions>,
}

impl RepoFiles {
    pub(crate) fn from_paths(mut files: Vec<String>, partial: bool) -> Self {
        files.sort();
        files.dedup();
        let mut exact = HashMap::with_capacity(files.len());
        let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
        for (position, file) in files.iter().enumerate() {
            let lower = file.to_lowercase();
            let name = lower.rsplit('/').next().unwrap_or("").to_string();
            exact.insert(lower, position);
            by_name.entry(name).or_default().push(position);
        }
        Self {
            files,
            exact,
            by_name,
            partial,
            definitions: OnceLock::new(),
        }
    }

    /// Whether the listing was cut, so a missing path proves nothing.
    pub(crate) fn partial(&self) -> bool {
        self.partial
    }

    /// Every file, sorted; a position in the listing is an index here.
    pub(crate) fn paths(&self) -> &[String] {
        &self.files
    }

    /// The file at `position` of the listing.
    pub(crate) fn file(&self, position: usize) -> &str {
        &self.files[position]
    }

    /// The files whose name is `name`, case aside.
    pub(crate) fn named(&self, name: &str) -> impl Iterator<Item = &str> {
        self.by_name
            .get(&name.to_lowercase())
            .into_iter()
            .flatten()
            .map(|position| self.files[*position].as_str())
    }

    /// The definitions of the code, scanned once for this listing.
    pub(crate) fn definitions(&self, root: &Path) -> &super::symbols::Definitions {
        self.definitions
            .get_or_init(|| super::symbols::scan(root, self))
    }

    /// What `cited` names: the path itself, else the one file that ends with
    /// it. A leading `./` is dropped; leading `../` mean the path was written
    /// relative to a folder below the root, so only a suffix can match it.
    pub(crate) fn resolve(&self, cited: &str) -> Resolved {
        let mut path = normalize_path(cited);
        let mut relative = false;
        while let Some(rest) = path.strip_prefix("../") {
            path = rest.to_string();
            relative = true;
        }
        let path = path.trim_start_matches("./").to_string();
        if path.is_empty() || path == ".." {
            return Resolved::Missing;
        }
        let lower = path.to_lowercase();
        if !relative {
            if let Some(position) = self.exact.get(&lower) {
                return Resolved::Exact(self.files[*position].clone());
            }
        }
        let name = lower.rsplit('/').next().unwrap_or("");
        let suffix = format!("/{lower}");
        let mut found = self
            .by_name
            .get(name)
            .into_iter()
            .flatten()
            .filter(|position| {
                let candidate = self.files[**position].to_lowercase();
                candidate.ends_with(&suffix) || (relative && candidate == lower)
            });
        match (found.next(), found.next()) {
            (Some(position), None) => Resolved::Unique(self.files[*position].clone()),
            (Some(_), Some(_)) => Resolved::Ambiguous,
            _ => Resolved::Missing,
        }
    }
}

/// The listing of `root`, kept for a few minutes. `None` when `root` is not a
/// folder, so nothing can be checked there.
pub(crate) fn repo_files(root: &Path) -> Arc<Option<RepoFiles>> {
    static CACHE: OnceLock<FolderCache<Option<RepoFiles>>> = OnceLock::new();
    CACHE
        .get_or_init(FolderCache::new)
        .get_or_make(root, || list(root))
}

/// The listing of `root` without the cache, for the scale gate.
#[doc(hidden)]
pub fn index_repository(root: &Path) -> Option<(usize, bool)> {
    list(root).map(|listing| (listing.files.len(), listing.partial))
}

/// Scans the definitions of the code under `root` without the cache, for the
/// scale gate: how many names it found.
#[doc(hidden)]
pub fn scan_symbols(root: &Path) -> Option<usize> {
    let listing = list(root)?;
    Some(listing.definitions(root).count())
}

fn list(root: &Path) -> Option<RepoFiles> {
    if !root.is_dir() {
        return None;
    }
    let (files, partial) = git_listing(root).unwrap_or_else(|| walk_listing(root));
    Some(RepoFiles::from_paths(files, partial))
}

/// `git ls-files` of the folder: tracked files and untracked files that are
/// not ignored. `None` when git is missing, fails or takes too long.
fn git_listing(root: &Path) -> Option<(Vec<String>, bool)> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(root)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    // Drained on its own thread: a full pipe would block git.
    let reader = std::thread::spawn(move || {
        let mut kept: Vec<u8> = Vec::new();
        let mut cut = false;
        let mut chunk = [0u8; 64 * 1024];
        while let Ok(read) = stdout.read(&mut chunk) {
            if read == 0 {
                break;
            }
            if kept.len() + read <= MAX_GIT_OUTPUT {
                kept.extend_from_slice(&chunk[..read]);
            } else {
                cut = true;
            }
        }
        (kept, cut)
    });
    let started = Instant::now();
    let success = loop {
        match child.try_wait().ok()? {
            Some(status) => break status.success(),
            None if started.elapsed() > LISTING_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            None => std::thread::sleep(Duration::from_millis(2)),
        }
    };
    let (bytes, cut) = reader.join().ok()?;
    if !success {
        return None;
    }
    let mut files: Vec<String> = bytes
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| String::from_utf8_lossy(path).replace('\\', "/"))
        .collect();
    let partial = cut || files.len() > MAX_REPO_FILES;
    files.truncate(MAX_REPO_FILES);
    Some((files, partial))
}

/// A bounded walk of the folder, for a project without git.
fn walk_listing(root: &Path) -> (Vec<String>, bool) {
    let started = Instant::now();
    let mut files: Vec<String> = Vec::new();
    let mut partial = false;
    let mut pending: Vec<String> = vec![String::new()];
    while let Some(folder) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(root.join(&folder)) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            if files.len() >= MAX_REPO_FILES || started.elapsed() > LISTING_TIMEOUT {
                partial = true;
                return (files, partial);
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let relative = if folder.is_empty() {
                name.clone()
            } else {
                format!("{folder}/{name}")
            };
            if kind.is_dir() {
                let hidden = name.starts_with('.') && !KEPT_HIDDEN_FOLDERS.contains(&name.as_str());
                if !hidden && !SKIPPED_FOLDERS.contains(&name.as_str()) {
                    pending.push(relative);
                }
            } else if kind.is_file() {
                files.push(relative);
            }
        }
    }
    (files, partial)
}

/// The files of a decision that count as ties to the map, as paths of the
/// project.
///
/// Documentation never counts. A decision taken from a document has the paths
/// of the whole document among its files, so one of them counts only when the
/// decision's own text cites it (and not to exclude it). With the listing, a
/// file must exist: it is resolved to its path, and one that is missing or
/// that several files answer to is dropped. Without a usable listing (no
/// folder, or a project too large to list) the files stand as written.
pub(crate) fn counted_files(
    cited: &[String],
    from_document: bool,
    texts: &[Folded],
    repo: Option<&RepoFiles>,
) -> Vec<String> {
    let mut counted: Vec<String> = Vec::new();
    for file in cited.iter().filter(|file| !is_documentation_path(file)) {
        if from_document {
            let cites = path_term(file)
                .is_some_and(|term| texts.iter().any(|text| text.mention(&term).is_some()));
            if !cites {
                continue;
            }
        }
        let path = match repo.filter(|repo| !repo.partial()) {
            Some(repo) => match repo.resolve(file) {
                Resolved::Exact(path) | Resolved::Unique(path) => path,
                Resolved::Ambiguous | Resolved::Missing => continue,
            },
            None => file.clone(),
        };
        if !counted.contains(&path) {
            counted.push(path);
        }
    }
    counted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(paths: &[&str]) -> RepoFiles {
        RepoFiles::from_paths(
            paths.iter().map(|path| (*path).to_string()).collect(),
            false,
        )
    }

    #[test]
    fn a_cited_path_is_exact_a_unique_suffix_ambiguous_or_missing() {
        let files = repo(&[
            "crates/net/examples/chat.rs",
            "crates/net/src/lib.rs",
            "crates/core/src/lib.rs",
            "README.md",
        ]);
        let resolved = |cited: &str| files.resolve(cited);
        assert_eq!(
            resolved("crates/net/src/lib.rs"),
            Resolved::Exact("crates/net/src/lib.rs".into())
        );
        assert_eq!(resolved("./README.md"), Resolved::Exact("README.md".into()));
        assert_eq!(
            resolved("CRATES\\NET\\SRC\\LIB.RS"),
            Resolved::Exact("crates/net/src/lib.rs".into())
        );
        // Relative to the crate that holds the file.
        assert_eq!(
            resolved("examples/chat.rs"),
            Resolved::Unique("crates/net/examples/chat.rs".into())
        );
        assert_eq!(
            resolved("../core/src/lib.rs"),
            Resolved::Unique("crates/core/src/lib.rs".into())
        );
        // Two crates have a src/lib.rs.
        assert_eq!(resolved("src/lib.rs"), Resolved::Ambiguous);
        // `../` never matches a root file by its exact path.
        assert_eq!(
            resolved("../README.md"),
            Resolved::Unique("README.md".into())
        );
        assert_eq!(resolved("examples/missing.rs"), Resolved::Missing);
        assert_eq!(resolved("../.."), Resolved::Missing);
        assert_eq!(resolved(""), Resolved::Missing);
    }

    #[test]
    fn the_walk_lists_files_and_skips_build_output_and_hidden_folders() {
        let root = std::env::temp_dir().join(format!("xemnas-walk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for file in [
            "src/lib.rs",
            ".github/workflows/ci.yml",
            ".git/config",
            "target/debug/x",
            "node_modules/a/index.js",
            "docs/readme.md",
        ] {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "x").unwrap();
        }
        let (mut files, partial) = walk_listing(&root);
        files.sort();
        assert!(!partial);
        assert_eq!(
            files,
            vec![".github/workflows/ci.yml", "docs/readme.md", "src/lib.rs"]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_git_listing_has_tracked_and_untracked_files_but_not_ignored_ones() {
        let root = std::env::temp_dir().join(format!("xemnas-git-list-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let git = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                .args(args)
                .stdin(Stdio::null())
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        };
        if !git(&["init", "-q"]) {
            let _ = std::fs::remove_dir_all(&root);
            return; // no git here: the walk covers this machine
        }
        for (file, text) in [
            (
                ".gitignore",
                "ignored.txt
",
            ),
            ("src/lib.rs", "x"),
            ("ignored.txt", "x"),
            ("new.rs", "x"),
        ] {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        assert!(git(&["add", "src/lib.rs"]));
        let (mut files, partial) = git_listing(&root).expect("git lists the folder");
        files.sort();
        assert!(!partial);
        assert_eq!(files, vec![".gitignore", "new.rs", "src/lib.rs"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_folder_that_does_not_exist_has_no_listing() {
        let missing = std::env::temp_dir().join("xemnas-no-such-folder-for-repo-files");
        assert!(repo_files(&missing).is_none());
    }

    fn texts(list: &[&str]) -> Vec<Folded> {
        list.iter().map(|text| Folded::new(text)).collect()
    }

    #[test]
    fn a_document_decision_counts_only_the_files_its_text_cites() {
        let files = repo(&[
            "crates/net/examples/chat.rs",
            "crates/core/src/lib.rs",
            "docs/adr/0011.md",
        ]);
        let cited: Vec<String> = [
            "docs/adr/0011.md",
            "examples/chat.rs",
            "../core/src/lib.rs",
            "examples/gone.rs",
        ]
        .iter()
        .map(|file| (*file).to_string())
        .collect();
        // The decision cites one file, and one that no longer exists.
        let cites = texts(&["O exemplo examples/chat.rs mostra a reconexão."]);
        assert_eq!(
            counted_files(&cited, true, &cites, Some(&files)),
            vec!["crates/net/examples/chat.rs"]
        );
        let gone = texts(&["Apagar examples/gone.rs depois."]);
        assert!(counted_files(&cited, true, &gone, Some(&files)).is_empty());
        // Named to be left out is not a citation.
        let excluded = texts(&["Sem examples/chat.rs no pacote."]);
        assert!(counted_files(&cited, true, &excluded, Some(&files)).is_empty());
        // A decision from a diff counts every file that exists.
        let from_diff = counted_files(&cited, false, &[], Some(&files));
        assert_eq!(
            from_diff,
            vec!["crates/net/examples/chat.rs", "crates/core/src/lib.rs"]
        );
        // Without a listing the files stand as written.
        assert_eq!(counted_files(&cited, false, &[], None).len(), 3);
        // A cut listing proves nothing about a missing file.
        let cut = RepoFiles::from_paths(vec!["a.rs".into()], true);
        assert_eq!(counted_files(&cited, false, &[], Some(&cut)).len(), 3);
    }
}
