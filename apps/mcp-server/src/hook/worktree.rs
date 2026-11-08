//! Files edited in another worktree of the session's repository.
//!
//! A session started in one worktree may edit files in a sibling worktree of
//! the same repository. Those edits belong to the project, so they are kept with
//! a path relative to the worktree they live in, which keeps diffs mapping to the
//! same component patterns whichever worktree they were edited in.

use super::envelope::relative_inside;
use super::root::{locate_directory, Commons, Located, Resolver};

/// Resolves edited paths against the session's cwd and its sibling worktrees.
///
/// A path is accepted by what the disk says it is, not by its prefix or by being
/// relative: it must really lie in the cwd's work tree, or in another work tree
/// of the same repository (same git common dir). A nested repository or a
/// junction to another repository inside the cwd is a different project. Git runs
/// once per distinct work tree, per instance.
pub struct Worktrees {
    cwd: String,
    /// Where the cwd lies; `None` when it is in no repository.
    ours: Option<Option<Located>>,
    commons: Commons,
    resolver: Resolver,
}

impl Worktrees {
    /// Resolver for a session running in `cwd`.
    pub fn new(cwd: &str) -> Self {
        Self {
            cwd: cwd.to_string(),
            ours: None,
            commons: Commons::default(),
            resolver: Resolver::new(cwd),
        }
    }

    /// Path of `path` relative to the cwd, or to the root of another worktree
    /// of the cwd's repository; `None` when it lies elsewhere.
    pub fn relative(&mut self, path: &str) -> Option<String> {
        let cwd = &self.cwd;
        let Some(ours) = self
            .ours
            .get_or_insert_with(|| locate_directory(cwd))
            .clone()
        else {
            // The cwd is in no repository: there is no identity to compare.
            return relative_inside(path, &self.cwd);
        };
        let placed = self.resolver.place(path)?;
        if !self.commons.same(&ours.top, &placed.top) {
            return None;
        }
        if placed.top == ours.top && !ours.relative.is_empty() {
            // Both are rooted at `top`: strip the cwd's own folder as a root.
            if let Some(file) = relative_inside(
                &format!("/{}", placed.relative),
                &format!("/{}", ours.relative),
            ) {
                return Some(file);
            }
        }
        Some(placed.relative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hook::envelope::build_envelope;
    use crate::hook::transcript::{FileEdit, Turn};
    use application::repo_identity::normalize_dir;
    use std::path::{Path, PathBuf};
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

    #[test]
    fn a_nested_repository_or_a_link_to_another_one_is_not_the_cwds_project() {
        if !git(Path::new("."), &["--version"]) {
            eprintln!("skipped: git is unavailable");
            return;
        }
        let root = std::env::temp_dir().join(format!("xemnas-wt-nest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let a = repo(&root, "a");
        let b = repo(&root, "b");
        let nested = repo(&a, "nested");
        let a_text = a.to_string_lossy().into_owned();
        let mut worktrees = Worktrees::new(&a_text);
        assert_eq!(
            worktrees
                .relative(&format!("{a_text}/src/one.rs"))
                .as_deref(),
            Some("src/one.rs")
        );
        assert_eq!(
            worktrees.relative(&nested.join("two.rs").to_string_lossy()),
            None
        );
        if crate::hook::root::tests::link(&a.join("link"), &b) {
            assert_eq!(
                worktrees.relative(&format!("{a_text}/link/foreign.rs")),
                None
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn relative_paths_get_the_same_identity_check_as_absolute_ones() {
        if !git(Path::new("."), &["--version"]) {
            eprintln!("skipped: git is unavailable");
            return;
        }
        let root = std::env::temp_dir().join(format!("xemnas-wt-rel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let a = repo(&root, "a");
        let b = repo(&root, "b");
        repo(&a, "nested");
        let a_text = a.to_string_lossy().into_owned();
        let mut worktrees = Worktrees::new(&a_text);
        assert_eq!(
            worktrees.relative("src/one.rs").as_deref(),
            Some("src/one.rs")
        );
        assert_eq!(worktrees.relative("nested/two.rs"), None);
        assert_eq!(worktrees.relative("../b/x.rs"), None);
        if crate::hook::root::tests::link(&a.join("link"), &b) {
            assert_eq!(worktrees.relative("link/foreign.rs"), None);
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_subst_drive_to_the_repository_keeps_its_root_and_diff() {
        #[cfg(windows)]
        {
            if !git(Path::new("."), &["--version"]) {
                return;
            }
            let root = std::env::temp_dir().join(format!("xemnas-wt-drive-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            let a = repo(&root, "a");
            let Some(drive) = crate::hook::root::tests::Drive::mount(&a) else {
                eprintln!("skipped: subst is unavailable");
                return;
            };
            let a_text = a.to_string_lossy().into_owned();
            let mut worktrees = Worktrees::new(&a_text);
            assert_eq!(
                worktrees
                    .relative(&format!("{}/one.rs", drive.0))
                    .as_deref(),
                Some("one.rs")
            );
            let _ = std::fs::remove_dir_all(&root);
        }
    }
}
