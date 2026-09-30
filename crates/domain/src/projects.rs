//! Projects: the directories the user chose to track.

use std::path::{Path, PathBuf};

/// Stable identifier of a tracked Project.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProjectId(String);

impl ProjectId {
    /// Wraps an already assigned identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrows the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Canonical location of a Project on disk.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProjectLocation(PathBuf);

impl ProjectLocation {
    /// Wraps an already canonical location.
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    /// Borrows the location as a [`Path`].
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// Derives a human-readable name from the last path component.
    pub fn display_name(&self) -> String {
        self.0
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| self.0.to_string_lossy().into_owned())
    }
}

impl std::fmt::Display for ProjectLocation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0.to_string_lossy())
    }
}

/// A tracked Project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    id: ProjectId,
    location: ProjectLocation,
    registered_at: String,
}

impl Project {
    /// Assembles a Project from its parts.
    pub fn new(id: ProjectId, location: ProjectLocation, registered_at: String) -> Self {
        Self {
            id,
            location,
            registered_at,
        }
    }

    /// Borrows the stable identifier.
    pub fn id(&self) -> &ProjectId {
        &self.id
    }

    /// Borrows the canonical location.
    pub fn location(&self) -> &ProjectLocation {
        &self.location
    }

    /// Borrows the RFC 3339 registration timestamp.
    pub fn registered_at(&self) -> &str {
        &self.registered_at
    }
}

/// Read model of a Project used to list tracked directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSummary {
    id: ProjectId,
    name: String,
    location: ProjectLocation,
    registered_at: String,
}

impl ProjectSummary {
    /// Assembles a summary from its parts.
    pub fn new(
        id: ProjectId,
        name: String,
        location: ProjectLocation,
        registered_at: String,
    ) -> Self {
        Self {
            id,
            name,
            location,
            registered_at,
        }
    }

    /// Borrows the stable identifier.
    pub fn id(&self) -> &ProjectId {
        &self.id
    }

    /// Borrows the derived display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Borrows the canonical location.
    pub fn location(&self) -> &ProjectLocation {
        &self.location
    }

    /// Borrows the RFC 3339 registration timestamp.
    pub fn registered_at(&self) -> &str {
        &self.registered_at
    }
}

#[cfg(test)]
mod tests {
    use super::{ProjectLocation, ProjectSummary};
    use std::path::PathBuf;

    #[test]
    fn display_name_uses_last_component() {
        let location = ProjectLocation::new(PathBuf::from("C:/work/xemnas"));
        assert_eq!(location.display_name(), "xemnas");
    }

    #[test]
    fn display_name_falls_back_to_full_path_for_root() {
        let location = ProjectLocation::new(PathBuf::from("/"));
        assert_eq!(location.display_name(), "/");
    }

    #[test]
    fn summary_exposes_its_parts() {
        let id = super::ProjectId::new("project-1");
        let location = ProjectLocation::new(PathBuf::from("C:/work/xemnas"));
        let summary = ProjectSummary::new(
            id.clone(),
            location.display_name(),
            location,
            "2026-01-01T00:00:00Z".to_string(),
        );
        assert_eq!(summary.id(), &id);
        assert_eq!(summary.name(), "xemnas");
        assert_eq!(summary.registered_at(), "2026-01-01T00:00:00Z");
    }
}
