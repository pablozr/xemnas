//! Session discovery file and token file.
//!
//! The adapter discovers the loopback port, protocol version and instance id
//! from `{runtime}/discovery.json`, and reads the bearer token from
//! `{runtime}/api-token`. Both are rewritten on every startup and removed on a
//! graceful shutdown (stack doc "API local → Ciclo de vida").
//!
//! On Unix the files are created with mode `600`. On Windows the token relies on
//! the per-user profile ACL, which is the platform's equivalent protection for
//! files under `%LOCALAPPDATA%`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Name of the discovery file.
pub const DISCOVERY_FILE: &str = "discovery.json";

/// Name of the raw token file.
pub const TOKEN_FILE: &str = "api-token";

/// Protocol version advertised by this build.
pub const PROTOCOL_VERSION: u32 = 1;

/// Generates a fresh per-session instance identifier.
pub fn generate_instance_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// Contents of the discovery file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryInfo {
    /// Protocol version the server speaks.
    pub protocol_version: u32,
    /// Actual bound loopback port.
    pub port: u16,
    /// Per-session instance identifier.
    pub instance_id: String,
}

/// Writes the discovery and token files, creating the runtime directory.
///
/// Overwrites any previous session files.
pub fn write_runtime_files(
    runtime_dir: &Path,
    discovery: &DiscoveryInfo,
    token: &str,
) -> std::io::Result<()> {
    std::fs::create_dir_all(runtime_dir)?;
    let discovery_json = serde_json::to_string_pretty(discovery)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    write_private(&runtime_dir.join(DISCOVERY_FILE), discovery_json.as_bytes())?;
    write_private(&runtime_dir.join(TOKEN_FILE), token.as_bytes())?;
    Ok(())
}

/// Removes the discovery and token files; missing files are ignored.
pub fn remove_runtime_files(runtime_dir: &Path) {
    let _ = std::fs::remove_file(runtime_dir.join(DISCOVERY_FILE));
    let _ = std::fs::remove_file(runtime_dir.join(TOKEN_FILE));
}

/// Writes `bytes` and restricts the file to the user where the OS supports it.
fn write_private(path: &PathBuf, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{remove_runtime_files, write_runtime_files, DiscoveryInfo, PROTOCOL_VERSION};

    #[test]
    fn runtime_files_are_written_and_removed() {
        let root = std::env::temp_dir().join(format!(
            "xemnas-discovery-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        ));
        let discovery = DiscoveryInfo {
            protocol_version: PROTOCOL_VERSION,
            port: 4321,
            instance_id: "instance-1".to_string(),
        };
        write_runtime_files(&root, &discovery, "token-value").expect("write files");

        let text = std::fs::read_to_string(root.join(super::DISCOVERY_FILE)).expect("read");
        let parsed: DiscoveryInfo = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed, discovery);
        assert_eq!(
            std::fs::read_to_string(root.join(super::TOKEN_FILE)).expect("token"),
            "token-value"
        );

        remove_runtime_files(&root);
        assert!(!root.join(super::DISCOVERY_FILE).exists());
        assert!(!root.join(super::TOKEN_FILE).exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
