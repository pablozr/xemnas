//! Files edited in another worktree of the session's repository.
//!
//! A session started in one worktree may edit files in a sibling worktree of
//! the same repository. Those edits belong to the project, so they are kept with
//! a path relative to the worktree they live in, which keeps diffs mapping to the
//! same component patterns whichever worktree they were edited in.

use std::collections::HashMap;
use std::path::Path;

use application::repo_identity::{git_output, normalize_dir};

use super::envelope::relative_inside;

/// Resolves edited paths against the session's cwd and its sibling worktrees.
///
/// Git runs once per distinct directory, and once for the cwd, per instance.
pub struct Worktrees {
    cwd: String,
    common_dir: Option<Option<String>>,
    toplevels: HashMap<String, Option<String>>,
}

impl Worktrees {
    /// Resolver for a session running in `cwd`.
    pub fn new(cwd: &str) -> Self {
        Self {
            cwd: cwd.to_string(),
            common_dir: None,
            toplevels: HashMap::new(),
        }
    }

    /// Path of `path` relative to the cwd, or to the root of another worktree
    /// of the cwd's repository; `None` when it lies elsewhere.
    pub fn relative(&mut self, path: &str) -> Option<String> {
        if let Some(file) = relative_inside(path, &self.cwd) {
            return Some(file);
        }
        let normalized = path.replace('\\', "/");
        let absolute = normalized.starts_with('/') || normalized.as_bytes().get(1) == Some(&b':');
        if !absolute {
            return None;
        }
        let directory = normalized.rsplit_once('/')?.0.to_string();
        let toplevel = self.toplevel_of(&directory)?;
        relative_inside(&normalized, &toplevel)
    }

    /// Root of the sibling worktree holding `directory`, if it is one.
    fn toplevel_of(&mut self, directory: &str) -> Option<String> {
        if let Some(known) = self.toplevels.get(directory) {
            return known.clone();
        }
        let resolved = self.resolve(directory);
        self.toplevels
            .insert(directory.to_string(), resolved.clone());
        resolved
    }

    fn resolve(&mut self, directory: &str) -> Option<String> {
        let cwd = &self.cwd;
        let ours = self
            .common_dir
            .get_or_insert_with(|| common_dir_and_toplevel(cwd).map(|(common, _)| common))
            .clone()?;
        // The edited file or its folder may not exist yet; ask from the
        // nearest folder that does.
        let existing = Path::new(directory)
            .ancestors()
            .find(|candidate| candidate.is_dir())?
            .to_string_lossy()
            .into_owned();
        let (theirs, toplevel) = common_dir_and_toplevel(&existing)?;
        (theirs == ours).then_some(toplevel)
    }
}

/// Normalized common git dir and the worktree root of `directory`.
fn common_dir_and_toplevel(directory: &str) -> Option<(String, String)> {
    let output = git_output(directory, &["--git-common-dir", "--show-toplevel"])?;
    let mut lines = output.lines();
    let common = normalize_dir(lines.next()?.trim())?;
    let toplevel = lines.next()?.trim().replace('\\', "/");
    (!toplevel.is_empty()).then_some((common, toplevel))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hook::envelope::build_envelope;
    use crate::hook::transcript::{FileEdit, Turn};
    use std::path::PathBuf;
    use std::process::Command;

    fn git(directory: &Path, args: &[&str]) -> bool {
        Command::new("git")
            .arg("-C")
            .arg(directory)
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Canonical path without the Windows verbatim prefix, as tools report it.
    fn plain(path: &Path) -> PathBuf {
        PathBuf::from(normalize_dir(&path.to_string_lossy()).expect("canonical"))
    }

    fn repo(root: &Path, name: &str) -> PathBuf {
        let path = root.join(name);
        std::fs::create_dir_all(&path).expect("dir");
        assert!(git(&path, &["init", "-q"]));
        assert!(git(&path, &["commit", "-q", "--allow-empty", "-m", "init"]));
        plain(&path)
    }

    fn edit(path: &Path) -> FileEdit {
        FileEdit {
            path: path.to_string_lossy().into_owned(),
            hunks: "@@ -1,1 +1,1 @@\n-a\n+b\n".to_string(),
        }
    }

    #[test]
    fn edits_in_a_sibling_worktree_are_kept_relative_to_it() {
        if !git(Path::new("."), &["--version"]) {
            eprintln!("skipped: git is unavailable");
            return;
        }
        let root = std::env::temp_dir().join(format!("xemnas-wt-hook-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let main = repo(&root, "main");
        let sibling = root.join("sibling");
        assert!(git(
            &main,
            &["worktree", "add", "-q", &sibling.to_string_lossy()]
        ));
        let sibling = plain(&sibling);
        let unrelated = repo(&root, "unrelated");
        let plain = root.join("plain");
        std::fs::create_dir_all(&plain).expect("dir");

        let turn = Turn {
            prompt_uuid: "p1".to_string(),
            edits: vec![
                edit(&main.join("a.rs")),
                // The folder does not exist yet inside the worktree.
                edit(&sibling.join("docs").join("adr").join("0001.md")),
                edit(&unrelated.join("x.rs")),
                edit(&plain.join("y.rs")),
            ],
            ..Turn::default()
        };
        let cwd = main.to_string_lossy().into_owned();
        let envelope = build_envelope(
            "s1",
            &cwd,
            &mut Worktrees::new(&cwd),
            &turn,
            "2026-01-01T00:00:00Z".to_string(),
        )
        .expect("envelope");

        let files: Vec<&str> = envelope
            .artifacts
            .iter()
            .filter_map(|artifact| artifact.metadata.get("file")?.as_str())
            .collect();
        assert_eq!(files, ["a.rs", "docs/adr/0001.md"]);
        let _ = std::fs::remove_dir_all(&root);
    }
}
