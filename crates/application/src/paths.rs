//! Local paths shared by every composition root (desktop, tools, future MCP).

use std::ffi::OsString;
use std::path::PathBuf;

/// Environment variable that overrides the data directory.
pub const DATA_DIR_ENV: &str = "XEMNAS_DATA_DIR";

/// Environment variable that overrides the outbox root.
pub const OUTBOX_DIR_ENV: &str = "XEMNAS_OUTBOX_DIR";

/// Where xemnas keeps its local state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    /// Data directory root.
    pub data_dir: PathBuf,
    /// SQLite database file.
    pub database: PathBuf,
    /// Directory with the discovery and session token files.
    pub runtime_dir: PathBuf,
    /// Outbox root shared with the adapter.
    pub outbox_dir: PathBuf,
    /// AI profile settings file.
    pub ai_profile: PathBuf,
}

impl AppPaths {
    /// Resolves the paths from `XEMNAS_DATA_DIR`, `LOCALAPPDATA` and `XEMNAS_OUTBOX_DIR`.
    pub fn from_env() -> Self {
        let data_dir = non_empty(std::env::var_os(DATA_DIR_ENV))
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("LOCALAPPDATA")
                    .map(PathBuf::from)
                    .unwrap_or_else(std::env::temp_dir)
                    .join("xemnas")
            });
        let outbox = non_empty(std::env::var_os(OUTBOX_DIR_ENV)).map(PathBuf::from);
        Self::under(data_dir, outbox)
    }

    /// Lays out the paths under `data_dir`, with an optional outbox override.
    pub fn under(data_dir: PathBuf, outbox_dir: Option<PathBuf>) -> Self {
        let state = data_dir.join("state");
        Self {
            database: state.join("app.db"),
            runtime_dir: state,
            outbox_dir: outbox_dir.unwrap_or_else(|| data_dir.join("outbox")),
            ai_profile: data_dir.join("settings").join("ai-profile.json"),
            data_dir,
        }
    }
}

fn non_empty(value: Option<OsString>) -> Option<OsString> {
    value.filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::AppPaths;
    use std::path::PathBuf;

    #[test]
    fn lays_out_every_path_under_the_data_directory() {
        let root = PathBuf::from("dados");
        let paths = AppPaths::under(root.clone(), None);
        assert_eq!(paths.database, root.join("state").join("app.db"));
        assert_eq!(paths.runtime_dir, root.join("state"));
        assert_eq!(paths.outbox_dir, root.join("outbox"));
        assert_eq!(
            paths.ai_profile,
            root.join("settings").join("ai-profile.json")
        );
    }

    #[test]
    fn outbox_override_replaces_only_the_outbox() {
        let paths = AppPaths::under(PathBuf::from("dados"), Some(PathBuf::from("caixa")));
        assert_eq!(paths.outbox_dir, PathBuf::from("caixa"));
        assert_eq!(paths.data_dir, PathBuf::from("dados"));
    }
}
