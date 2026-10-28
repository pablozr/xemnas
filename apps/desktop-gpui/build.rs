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
    println!(
        "cargo:rustc-env=XEMNAS_COMMIT={}",
        commit.unwrap_or_else(|| "dev".to_owned())
    );
    println!("cargo:rerun-if-env-changed=XEMNAS_COMMIT");
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
