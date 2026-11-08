//! Records the commit the binary was built from, for the colophon.

use std::process::Command;

fn main() {
    let commit = std::env::var("XEMNAS_COMMIT")
        .ok()
        .filter(|c| !c.is_empty())
        .or_else(|| {
            let output = Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .output()
                .ok()
                .filter(|output| output.status.success())?;
            String::from_utf8(output.stdout)
                .ok()
                .map(|c| c.trim().to_owned())
        });
    // Uncommitted changes mark the hash, so a binary never passes for a commit it is not.
    let dirty = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=no"])
        .output()
        .ok()
        .is_some_and(|output| output.status.success() && !output.stdout.is_empty());
    let commit = commit.unwrap_or_else(|| "dev".to_owned());
    let commit = if dirty && std::env::var("XEMNAS_COMMIT").is_err() {
        format!("{commit}-dirty")
    } else {
        commit
    };
    println!("cargo:rustc-env=XEMNAS_COMMIT={commit}");
    println!("cargo:rerun-if-env-changed=XEMNAS_COMMIT");
    println!("cargo:rerun-if-changed=src");
    // The reflog of HEAD grows with each commit; the path differs inside a worktree.
    if let Some(head) = Command::new("git")
        .args(["rev-parse", "--git-path", "logs/HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
    {
        println!("cargo:rerun-if-changed={}", head.trim());
    }
}
