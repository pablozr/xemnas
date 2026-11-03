//! Project root of a capture when the session's cwd is not a repository.
//!
//! Agents started by a launcher keep their cwd in a folder of their own and edit
//! other repositories by absolute path. The project is then the repository the
//! turn worked in, not the cwd. A cwd inside a work tree always stays the root.
//! Which repository a path belongs to is read from the disk (links resolved, the
//! nearest `.git`), never from its text: a nested repository or a junction inside
//! a repository belongs to another one. [`resolve`] is the one place that does it,
//! relative paths (taken from a cwd), drive roots and UNC shares included.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

use application::repo_identity::{git_output, normalize_dir};

/// Score of a file the turn edited; reading a file scores 1.
const EDIT_SCORE: usize = 3;

/// `path` with forward slashes when it is absolute: rooted, or a drive followed
/// by a separator (`C:repo` is relative to the drive's current folder).
pub fn absolute(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    let bytes = normalized.as_bytes();
    let drive = bytes.first().is_some_and(u8::is_ascii_alphabetic)
        && bytes.get(1) == Some(&b':')
        && bytes.get(2) == Some(&b'/');
    (normalized.starts_with('/') || drive).then_some(normalized)
}

/// A path placed in the repository it really belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located {
    /// Work tree root, with forward slashes.
    pub top: String,
    /// The path relative to `top`; empty for the root itself.
    pub relative: String,
}

fn join(parent: &str, name: &str) -> String {
    match (parent.is_empty(), name.is_empty()) {
        (true, _) => name.to_string(),
        (_, true) => parent.to_string(),
        _ => format!("{parent}/{name}"),
    }
}

/// Where the absolute `directory` really lies: its nearest existing folder is
/// resolved through links, junctions and `subst` drives, and the nearest folder at
/// or above that holding `.git` is the work tree.
pub fn locate_directory(directory: &str) -> Option<Located> {
    let directory = absolute(directory)?;
    let directory = if directory.len() > 3 {
        directory.trim_end_matches('/').to_string()
    } else {
        directory
    };
    // An edited file's own folder may not exist yet.
    let existing = Path::new(&directory)
        .ancestors()
        .find(|candidate| candidate.is_dir())?;
    let existing_text = existing.to_string_lossy().replace('\\', "/");
    let tail = directory
        .get(existing_text.trim_end_matches('/').len()..)
        .unwrap_or_default()
        .trim_matches('/');
    let real = normalize_dir(&existing.to_string_lossy())?.replace('\\', "/");
    // A canonical path that is not absolute or a tail that climbs out of the
    // folder found proves nothing about the repository.
    absolute(&real)?;
    if tail.split('/').any(|part| part == "..") {
        return None;
    }
    let top = Path::new(&real)
        .ancestors()
        .find(|folder| folder.join(".git").exists())?
        .to_string_lossy()
        .replace('\\', "/");
    let inside = real
        .get(top.trim_end_matches('/').len()..)
        .unwrap_or_default()
        .trim_matches('/');
    Some(Located {
        relative: join(inside, tail),
        top,
    })
}

/// `path` as an absolute text with forward slashes; a relative one is taken from
/// `cwd`. `C:repo` (relative to a drive's current folder) has no place.
fn absolute_from(path: &str, cwd: &str) -> Option<String> {
    if let Some(found) = absolute(path) {
        return Some(found);
    }
    let normalized = path.replace('\\', "/");
    if normalized.as_bytes().get(1) == Some(&b':') {
        return None;
    }
    let mut relative = normalized.as_str();
    while let Some(rest) = relative.strip_prefix("./") {
        relative = rest;
    }
    let base = absolute(cwd)?;
    (!relative.is_empty()).then(|| format!("{}/{relative}", base.trim_end_matches('/')))
}

/// The folder and the name of an absolute file path; a drive root (`Z:`) or the
/// filesystem root keeps its slash.
fn split_file(path: &str) -> Option<(String, &str)> {
    let (directory, name) = path.rsplit_once('/')?;
    let directory = if directory.is_empty() || directory.ends_with(':') {
        format!("{directory}/")
    } else {
        directory.to_string()
    };
    Some((directory, name))
}

/// Where the absolute file `path` (which may not exist) really lies.
fn locate_file(path: &str) -> Option<Located> {
    let (directory, name) = split_file(path)?;
    locate_directory(&directory).map(|placed| place_file(placed, name))
}

/// `placed`, the folder of a file, extended by the file's `name`.
fn place_file(placed: Located, name: &str) -> Located {
    Located {
        relative: join(&placed.relative, name),
        top: placed.top,
    }
}

/// Where `path` really lies: a file, which may not exist, or a folder, absolute or
/// relative to `cwd`.
pub fn resolve(path: &str, cwd: &str) -> Option<Located> {
    let path = absolute_from(path, cwd)?;
    if Path::new(&path).is_dir() {
        locate_directory(&path)
    } else {
        locate_file(&path)
    }
}

/// Work tree root of the absolute `path` (a file or a folder), or `None` when it
/// is relative or lies outside any repository.
pub fn toplevel_of_path(path: &str) -> Option<String> {
    resolve(path, "").map(|located| located.top)
}

/// [`resolve`] for many files: each distinct folder is read from the disk once.
pub struct Resolver {
    cwd: String,
    folders: HashMap<String, Option<Located>>,
}

impl Resolver {
    /// Resolver of paths relative to `cwd`.
    pub fn new(cwd: &str) -> Self {
        Self {
            cwd: cwd.to_string(),
            folders: HashMap::new(),
        }
    }

    /// Folders read from the disk so far.
    fn folders(&self) -> usize {
        self.folders.len()
    }

    /// Where the file `path` lies; `Err` when its folder is new and `may_lookup` is false.
    fn lookup(&mut self, path: &str, may_lookup: bool) -> Result<Option<Located>, ()> {
        let Some(path) = absolute_from(path, &self.cwd) else {
            return Ok(None);
        };
        let Some((directory, name)) = split_file(&path) else {
            return Ok(None);
        };
        let placed = match self.folders.get(&directory) {
            Some(known) => known.clone(),
            None if may_lookup => {
                let found = locate_directory(&directory);
                self.folders.insert(directory, found.clone());
                found
            }
            None => return Err(()),
        };
        Ok(placed.map(|placed| place_file(placed, name)))
    }

    /// Where the file `path` lies, or `None` when it lies in no repository.
    pub fn place(&mut self, path: &str) -> Option<Located> {
        self.lookup(path, true).ok().flatten()
    }
}

/// Git common dirs of work trees, asked once each: the work trees of one
/// repository (linked worktrees) share it, and nothing else does.
#[derive(Default)]
pub struct Commons(HashMap<String, Option<String>>);

impl Commons {
    fn of(&mut self, top: &str) -> Option<String> {
        self.0
            .entry(top.to_string())
            .or_insert_with(|| {
                let output = git_output(top, &["--git-common-dir"])?;
                Some(normalize_dir(output.trim())?.replace('\\', "/"))
            })
            .clone()
    }

    /// Whether the work trees `a` and `b` are the same repository.
    pub fn same(&mut self, a: &str, b: &str) -> bool {
        a == b || matches!((self.of(a), self.of(b)), (Some(x), Some(y)) if x == y)
    }
}

/// Key under which two spellings of one file meet.
fn path_key(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    if cfg!(windows) {
        normalized.to_ascii_lowercase()
    } else {
        normalized
    }
}

/// Picks the root of a session. Folders are resolved once per distinct folder, per instance.
pub struct Roots {
    cwd: String,
    /// Most folders resolved for paths; beyond it the best repository found so far wins.
    max_lookups: usize,
    deadline: Option<Instant>,
    cwd_in_worktree: Option<bool>,
    resolver: Resolver,
}

impl Roots {
    /// Roots for a session running in `cwd`, resolving at most `max_lookups` folders for paths.
    pub fn new(cwd: &str, max_lookups: usize) -> Self {
        Self {
            cwd: cwd.to_string(),
            max_lookups,
            deadline: None,
            cwd_in_worktree: None,
            resolver: Resolver::new(cwd),
        }
    }

    /// Stops resolving folders at `deadline`; what was resolved by then decides.
    #[must_use]
    pub fn until(mut self, deadline: Instant) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Whether the cwd really lies inside a git work tree (a `.git` folder or file
    /// at or above it, links resolved). No process, so a session inside a
    /// repository pays nothing for the root.
    fn cwd_in_worktree(&mut self) -> bool {
        let cwd = &self.cwd;
        *self
            .cwd_in_worktree
            .get_or_insert_with(|| locate_directory(cwd).is_some())
    }

    /// The root of a turn that edited `edits` and read `touched`. A cwd inside a
    /// work tree is the root. Otherwise the repository with the highest score wins
    /// (each distinct edited file 3, each other distinct read file 1; a file is
    /// distinct by where it really lies, so aliases of one file count once;
    /// relative paths are taken from the cwd); a tie goes to the one met first,
    /// edits before reads. When the lookups run out the best repository found so
    /// far wins. No repository keeps the cwd.
    pub fn root_for(&mut self, edits: &[String], touched: &[String]) -> String {
        if (edits.is_empty() && touched.is_empty()) || self.cwd_in_worktree() {
            return self.cwd.clone();
        }
        let mut seen: HashSet<String> = HashSet::new();
        let mut scores: Vec<(String, usize)> = Vec::new();
        let paths = edits
            .iter()
            .map(|path| (path, EDIT_SCORE))
            .chain(touched.iter().map(|path| (path, 1)));
        for (path, score) in paths {
            let may_lookup = self.resolver.folders() < self.max_lookups
                && self
                    .deadline
                    .is_none_or(|deadline| Instant::now() < deadline);
            let Ok(found) = self.resolver.lookup(path, may_lookup) else {
                break;
            };
            let Some(Located { top, relative }) = found else {
                continue;
            };
            if !seen.insert(path_key(&join(&top, &relative))) {
                continue;
            }
            match scores.iter_mut().find(|(known, _)| *known == top) {
                Some((_, total)) => *total += score,
                None => scores.push((top, score)),
            }
        }
        let mut best: Option<&(String, usize)> = None;
        for candidate in &scores {
            if best.is_none_or(|current| candidate.1 > current.1) {
                best = Some(candidate);
            }
        }
        best.map_or_else(|| self.cwd.clone(), |(top, _)| top.clone())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
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

    fn scratch(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("xemnas-root-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("scratch");
        PathBuf::from(normalize_dir(&root.to_string_lossy()).expect("canonical"))
    }

    fn text(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    fn repo(path: &Path) -> String {
        std::fs::create_dir_all(path).expect("dir");
        assert!(git(path, &["init", "-q"]));
        text(path)
    }

    /// A directory link to `target`: a junction on Windows, a symlink elsewhere.
    pub(crate) fn link(at: &Path, target: &Path) -> bool {
        #[cfg(windows)]
        let made = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(at)
            .arg(target)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        #[cfg(not(windows))]
        let made = std::os::unix::fs::symlink(target, at).is_ok();
        made
    }

    fn git_available() -> bool {
        git(Path::new("."), &["--version"])
    }

    #[test]
    fn a_nested_repository_is_its_own_root_not_its_parents() {
        let base = scratch("nested");
        let a = repo(&base.join("a"));
        let nested = repo(&base.join("a").join("nested"));
        let home = text(&base.join("agent"));
        std::fs::create_dir_all(&home).expect("home");
        let root = Roots::new(&home, 16).root_for(
            &[
                format!("{a}/one.rs"),
                format!("{nested}/two.rs"),
                format!("{nested}/three.rs"),
            ],
            &[],
        );
        assert_eq!(root, nested);
        assert_eq!(
            resolve(&format!("{nested}/two.rs"), ""),
            Some(Located {
                top: nested,
                relative: "two.rs".to_string()
            })
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_link_inside_a_repository_belongs_to_its_target() {
        let base = scratch("link");
        let a = repo(&base.join("a"));
        let b = repo(&base.join("b"));
        if !link(&base.join("a").join("link"), &base.join("b")) {
            eprintln!("skipped: links are unavailable");
            return;
        }
        assert_eq!(
            resolve(&format!("{a}/link/foreign.rs"), ""),
            Some(Located {
                top: b,
                relative: "foreign.rs".to_string()
            })
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_repository_root_is_its_own_toplevel_and_a_new_folder_resolves_from_its_parent() {
        let base = scratch("dir");
        let a = repo(&base.join("a"));
        assert_eq!(toplevel_of_path(&a), Some(a.clone()));
        assert_eq!(
            resolve(&format!("{a}/src/deep/new.rs"), ""),
            Some(Located {
                top: a,
                relative: "src/deep/new.rs".to_string()
            })
        );
        assert_eq!(toplevel_of_path("relative/x.rs"), None);
        assert_eq!(toplevel_of_path("C:repo/x.rs"), None);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn running_out_of_lookups_keeps_the_best_repository_found() {
        let base = scratch("budget");
        let a = repo(&base.join("a"));
        let home = text(&base.join("agent"));
        std::fs::create_dir_all(&home).expect("home");
        let mut reads = vec![format!("{a}/one.rs")];
        for n in 0..4 {
            let folder = base.join(format!("plain{n}"));
            std::fs::create_dir_all(&folder).expect("plain");
            reads.push(format!("{}/x.rs", text(&folder)));
        }
        assert_eq!(Roots::new(&home, 4).root_for(&[], &reads), a);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_edited_file_is_not_scored_again_as_a_read() {
        let base = scratch("score");
        let a = repo(&base.join("a"));
        let b = repo(&base.join("b"));
        let home = text(&base.join("agent"));
        std::fs::create_dir_all(&home).expect("home");
        let edit = vec![format!("{a}/one.rs")];
        let mut touched = vec![edit[0].clone()];
        touched.extend((1..=4).map(|n| format!("{b}/{n}.rs")));
        // 1 edit (3) against 4 reads (4): B, not a 6:4 win for A.
        assert_eq!(Roots::new(&home, 16).root_for(&edit, &touched), b);
        // The same edit repeated is still one file.
        let twice = vec![edit[0].clone(), edit[0].clone()];
        assert_eq!(Roots::new(&home, 16).root_for(&twice, &touched), b);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_expired_deadline_stops_the_lookups() {
        let base = scratch("deadline");
        let a = repo(&base.join("a"));
        let home = text(&base.join("agent"));
        std::fs::create_dir_all(&home).expect("home");
        let mut roots = Roots::new(&home, 16).until(Instant::now());
        assert_eq!(roots.root_for(&[format!("{a}/x.rs")], &[]), home);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn work_trees_of_one_repository_share_the_common_dir_and_others_do_not() {
        if !git_available() {
            return;
        }
        let base = scratch("commons");
        let main = repo(&base.join("main"));
        assert!(git(
            Path::new(&main),
            &["commit", "-q", "--allow-empty", "-m", "i"]
        ));
        let sibling = base.join("sibling");
        assert!(git(
            Path::new(&main),
            &["worktree", "add", "-q", &sibling.to_string_lossy()]
        ));
        let other = repo(&base.join("other"));
        let sibling = text(&sibling);
        let mut commons = Commons::default();
        assert!(commons.same(&main, &sibling));
        assert!(!commons.same(&main, &other));
        let _ = std::fs::remove_dir_all(&base);
    }

    /// A `subst` drive for `target`, removed on drop.
    #[cfg(windows)]
    pub(crate) struct Drive(pub String);

    #[cfg(windows)]
    impl Drive {
        /// `None` when no drive letter is free or `subst` is unavailable.
        pub(crate) fn mount(target: &Path) -> Option<Drive> {
            ('P'..='Z').rev().find_map(|letter| {
                let drive = format!("{letter}:");
                if Path::new(&format!("{drive}/")).exists() {
                    return None;
                }
                Command::new("subst")
                    .arg(&drive)
                    .arg(target)
                    .output()
                    .ok()
                    .filter(|output| output.status.success())
                    .map(|_| Drive(drive))
            })
        }
    }

    #[cfg(windows)]
    impl Drop for Drive {
        fn drop(&mut self) {
            let _ = Command::new("subst").args([&self.0, "/D"]).output();
        }
    }

    #[test]
    fn relative_paths_are_placed_from_the_cwd_by_the_disk() {
        let base = scratch("relative");
        let a = repo(&base.join("a"));
        let b = repo(&base.join("b"));
        let nested = repo(&base.join("a").join("nested"));
        let linked = link(&base.join("a").join("link"), &base.join("b"));
        assert_eq!(
            resolve("src/one.rs", &a),
            Some(Located {
                top: a.clone(),
                relative: "src/one.rs".to_string()
            })
        );
        assert_eq!(
            resolve("./nested/two.rs", &a),
            Some(Located {
                top: nested,
                relative: "two.rs".to_string()
            })
        );
        if linked {
            assert_eq!(
                resolve("link/foreign.rs", &a),
                Some(Located {
                    top: b,
                    relative: "foreign.rs".to_string()
                })
            );
        }
        assert_eq!(resolve("C:repo/x.rs", &a), None);
        assert_eq!(resolve("x.rs", "relative-cwd"), None);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_cwd_in_a_link_to_a_repository_is_inside_it_whatever_its_text() {
        let base = scratch("cwdlink");
        let a = repo(&base.join("a"));
        repo(&base.join("b"));
        std::fs::create_dir_all(base.join("b").join("src")).expect("src");
        let home = base.join("agent");
        std::fs::create_dir_all(&home).expect("home");
        if !link(&home.join("alias"), &base.join("b").join("src")) {
            eprintln!("skipped: links are unavailable");
            return;
        }
        let cwd = text(&home.join("alias"));
        // The cwd's text climbs to no repository, its disk lies in B: it stays the root.
        let root = Roots::new(&cwd, 16).root_for(&[format!("{a}/one.rs")], &[]);
        assert_eq!(root, cwd);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_drive_root_keeps_its_slash_and_a_subst_drive_is_its_target() {
        assert_eq!(split_file("Z:/one.rs"), Some(("Z:/".to_string(), "one.rs")));
        assert_eq!(split_file("/one.rs"), Some(("/".to_string(), "one.rs")));
        assert_eq!(
            split_file("C:/a/one.rs"),
            Some(("C:/a".to_string(), "one.rs"))
        );
        #[cfg(windows)]
        {
            let base = scratch("drive");
            let a = repo(&base.join("a"));
            let home = text(&base.join("agent"));
            std::fs::create_dir_all(&home).expect("home");
            let Some(drive) = Drive::mount(&base.join("a")) else {
                eprintln!("skipped: subst is unavailable");
                return;
            };
            let file = format!("{}/one.rs", drive.0);
            assert_eq!(
                resolve(&file, ""),
                Some(Located {
                    top: a.clone(),
                    relative: "one.rs".to_string()
                })
            );
            assert_eq!(Roots::new(&home, 16).root_for(&[file], &[]), a);
            let _ = std::fs::remove_dir_all(&base);
        }
    }

    #[test]
    fn one_file_under_two_spellings_is_scored_once() {
        let base = scratch("alias");
        let a = repo(&base.join("a"));
        let b = repo(&base.join("b"));
        let home = text(&base.join("agent"));
        std::fs::create_dir_all(&home).expect("home");
        if !link(&base.join("alias"), &base.join("b")) {
            eprintln!("skipped: links are unavailable");
            return;
        }
        let alias = text(&base.join("alias"));
        let edit = vec![format!("{a}/one.rs")];
        // B read under two spellings is 2 files, not 4: 1 edit (3) beats 2 reads.
        let touched = vec![
            format!("{b}/1.rs"),
            format!("{alias}/1.rs"),
            format!("{b}/2.rs"),
            format!("{alias}/2.rs"),
        ];
        assert_eq!(Roots::new(&home, 16).root_for(&edit, &touched), a);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_unc_share_is_an_absolute_path() {
        assert_eq!(
            absolute(r"\\server\share\repo\x.rs").as_deref(),
            Some("//server/share/repo/x.rs")
        );
        assert_eq!(
            absolute_from("x.rs", r"\\server\share\repo").as_deref(),
            Some("//server/share/repo/x.rs")
        );
        assert_eq!(
            split_file("//server/share/x.rs"),
            Some(("//server/share".to_string(), "x.rs"))
        );
    }

    #[test]
    fn drive_relative_paths_are_not_absolute() {
        assert_eq!(absolute("C:repo"), None);
        assert_eq!(absolute("C:\\repo").as_deref(), Some("C:/repo"));
        assert_eq!(absolute("/repo").as_deref(), Some("/repo"));
        assert_eq!(absolute(""), None);
    }
}
