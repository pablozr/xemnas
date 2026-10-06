//! Repository identity: lets a git worktree count as the project it belongs to.
//!
//! Two directories share an identity when `git rev-parse --git-common-dir`
//! points at the same place, which is true for a repository and every
//! worktree created from it with `git worktree add`.

use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// How long git may take before the directory is treated as having no identity.
const GIT_TIMEOUT: Duration = Duration::from_secs(2);
/// How long a result, hit or miss, is reused.
const CACHE_TTL: Duration = Duration::from_secs(300);
/// Upper bound of cached directories; the cache is dropped when it is exceeded.
const CACHE_CAPACITY: usize = 256;

/// Port that names the repository a directory belongs to.
pub trait RepoIdentity {
    /// Normalized common git dir of `directory`, or `None` when it is not a
    /// git repository or git is unavailable.
    fn common_dir(&self, directory: &str) -> Option<String>;

    /// Normalized root of the worktree that contains `directory`, if any.
    fn worktree_root(&self, _directory: &str) -> Option<String> {
        None
    }
}

/// Runs `git` without a shell to read the common dir.
#[derive(Debug, Clone, Copy, Default)]
pub struct GitRepoIdentity;

impl RepoIdentity for GitRepoIdentity {
    fn common_dir(&self, directory: &str) -> Option<String> {
        let output = git_output(directory, &["--git-common-dir"])?;
        normalize_dir(output.lines().next()?.trim())
    }

    fn worktree_root(&self, directory: &str) -> Option<String> {
        let output = git_output(directory, &["--show-toplevel"])?;
        normalize_dir(output.lines().next()?.trim())
    }
}

/// Runs `git -C <directory> rev-parse --path-format=absolute <args>` without a
/// shell and returns its stdout, or `None` on failure or timeout.
pub fn git_output(directory: &str, args: &[&str]) -> Option<String> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(directory)
        .args(["rev-parse", "--path-format=absolute"])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().ok()?;

    let started = Instant::now();
    loop {
        match child.try_wait().ok()? {
            Some(status) if status.success() => break,
            Some(_) => return None,
            None if started.elapsed() > GIT_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }

    let mut output = String::new();
    std::io::Read::read_to_string(&mut child.stdout.take()?, &mut output).ok()?;
    Some(output)
}

/// Canonicalizes so spellings of one directory compare equal.
pub fn normalize_dir(path: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    let canonical = std::fs::canonicalize(Path::new(path)).ok()?;
    let text = canonical.to_string_lossy();
    let text = text.strip_prefix(r"\\?\").unwrap_or(&text);
    Some(text.to_string())
}

/// Caches another identity, misses included, bounded and expiring.
pub struct CachedRepoIdentity<I> {
    inner: I,
    entries: Mutex<HashMap<String, (Instant, Option<String>)>>,
}

impl<I> CachedRepoIdentity<I> {
    /// Wraps `inner` with an empty cache.
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            entries: Mutex::new(HashMap::new()),
        }
    }
}

impl<I: RepoIdentity> RepoIdentity for CachedRepoIdentity<I> {
    fn common_dir(&self, directory: &str) -> Option<String> {
        if let Ok(entries) = self.entries.lock() {
            if let Some((at, value)) = entries.get(directory) {
                if at.elapsed() < CACHE_TTL {
                    return value.clone();
                }
            }
        }

        let value = self.inner.common_dir(directory);
        if let Ok(mut entries) = self.entries.lock() {
            if entries.len() >= CACHE_CAPACITY {
                entries.clear();
            }
            entries.insert(directory.to_string(), (Instant::now(), value.clone()));
        }
        value
    }

    fn worktree_root(&self, directory: &str) -> Option<String> {
        self.inner.worktree_root(directory)
    }
}

/// Process-wide git identity shared by every request.
pub fn shared() -> &'static CachedRepoIdentity<GitRepoIdentity> {
    static SHARED: OnceLock<CachedRepoIdentity<GitRepoIdentity>> = OnceLock::new();
    SHARED.get_or_init(|| CachedRepoIdentity::new(GitRepoIdentity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projects::{
        find_project_with_identity, ProjectError, ProjectRecord, ProjectRepository,
    };
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

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

    fn repo(root: &Path, name: &str) -> PathBuf {
        let path = root.join(name);
        std::fs::create_dir_all(&path).expect("dir");
        assert!(git(&path, &["init", "-q"]));
        assert!(git(&path, &["commit", "-q", "--allow-empty", "-m", "init"]));
        path
    }

    struct Projects(Vec<ProjectRecord>);

    impl ProjectRepository for Projects {
        fn insert(&self, _: &ProjectRecord) -> Result<(), ProjectError> {
            Ok(())
        }
        fn list(&self) -> Result<Vec<ProjectRecord>, ProjectError> {
            Ok(self.0.clone())
        }
        fn get(&self, _: &str) -> Result<Option<ProjectRecord>, ProjectError> {
            Ok(None)
        }
        fn remove(&self, _: &str) -> Result<bool, ProjectError> {
            Ok(false)
        }
    }

    struct Counting(AtomicUsize);

    impl RepoIdentity for Counting {
        fn common_dir(&self, directory: &str) -> Option<String> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Some(directory.to_string())
        }
    }

    fn record(location: &Path) -> ProjectRecord {
        let location = crate::projects::canonicalize_location(&location.to_string_lossy())
            .expect("canonical location");
        ProjectRecord::new(
            "p1".to_string(),
            location,
            "2026-01-01T00:00:00Z".to_string(),
        )
    }

    #[test]
    fn a_worktree_resolves_to_the_registered_project_and_others_do_not() {
        if !git(Path::new("."), &["--version"]) {
            eprintln!("skipped: git is unavailable");
            return;
        }
        let root = std::env::temp_dir().join(format!("xemnas-wt-app-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let main = repo(&root, "main");
        let sibling = root.join("sibling");
        assert!(git(
            &main,
            &["worktree", "add", "-q", &sibling.to_string_lossy()]
        ));
        let unrelated = repo(&root, "unrelated");
        let plain = root.join("plain");
        std::fs::create_dir_all(&plain).expect("dir");

        let projects = Projects(vec![record(&main)]);
        let find = |path: &Path| {
            find_project_with_identity(&projects, &GitRepoIdentity, &path.to_string_lossy())
                .expect("lookup")
        };
        assert_eq!(find(&main).map(|found| found.id), Some("p1".to_string()));
        assert_eq!(find(&sibling).map(|found| found.id), Some("p1".to_string()));
        assert!(find(&unrelated).is_none());
        assert!(find(&plain).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_cache_spawns_git_once_per_directory_misses_included() {
        let cached = CachedRepoIdentity::new(Counting(AtomicUsize::new(0)));
        for _ in 0..3 {
            cached.common_dir("a");
            cached.common_dir("b");
        }
        assert_eq!(cached.inner.0.load(Ordering::SeqCst), 2);
    }
}
