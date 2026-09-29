//! Projects use cases: register, list and remove tracked directories.
//!
//! This module owns the [`ProjectRepository`] port that infrastructure
//! implements. Registering canonicalizes the directory (and only that: the
//! directory's content is never read, written or copied) and mints a UUID v7
//! identity, so a location registered twice is rejected through the storage
//! `UNIQUE` constraint.

use std::path::{Path, PathBuf};

use domain::projects::{Project, ProjectId, ProjectLocation, ProjectSummary};

use crate::clock::now_rfc3339;

/// A Project row as it is persisted.
///
/// This is a plain persistence record: it carries the identifier, the
/// normalized location string and the RFC 3339 timestamp exactly as they are
/// stored, so the application and storage layers can exchange it without
/// depending on a shared database type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRecord {
    /// Stable identifier of the Project.
    pub id: String,
    /// Normalized canonical location of the Project directory.
    pub location: String,
    /// RFC 3339 timestamp of when the Project was registered.
    pub registered_at: String,
}

impl ProjectRecord {
    /// Assembles a record from its persisted parts.
    pub fn new(id: String, location: String, registered_at: String) -> Self {
        Self {
            id,
            location,
            registered_at,
        }
    }
}

/// Failure modes of the Project use cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectError {
    /// The supplied path does not exist, cannot be canonicalized, or is not a
    /// directory. Nothing is persisted.
    InvalidLocation,
    /// The canonical location is already tracked.
    AlreadyRegistered,
    /// No Project with the requested identifier exists.
    NotFound,
    /// The storage backend failed; the message is diagnostic only.
    Storage(String),
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLocation => {
                formatter.write_str("the location must be an existing directory")
            }
            Self::AlreadyRegistered => {
                formatter.write_str("this location is already being tracked")
            }
            Self::NotFound => formatter.write_str("project not found"),
            Self::Storage(message) => write!(formatter, "storage error: {message}"),
        }
    }
}

impl std::error::Error for ProjectError {}

/// Port that persists Projects.
///
/// Infrastructure crates implement this trait; the application never knows how
/// the rows are stored.
pub trait ProjectRepository {
    /// Inserts a new record.
    ///
    /// Returns [`ProjectError::AlreadyRegistered`] when the identifier or the
    /// canonical location already exists.
    fn insert(&self, record: &ProjectRecord) -> Result<(), ProjectError>;

    /// Returns every record ordered by registration time.
    fn list(&self) -> Result<Vec<ProjectRecord>, ProjectError>;

    /// Returns the record with the given identifier, if any.
    fn get(&self, id: &str) -> Result<Option<ProjectRecord>, ProjectError>;

    /// Returns the record whose normalized canonical location matches, if any.
    ///
    /// The ingest use case uses this as the project allow-list check. The
    /// default scans [`ProjectRepository::list`]; storage overrides it with an
    /// indexed query, and the default keeps in-memory test doubles compatible.
    fn find_by_location(&self, location: &str) -> Result<Option<ProjectRecord>, ProjectError> {
        Ok(self
            .list()?
            .into_iter()
            .find(|record| record.location == location))
    }

    /// Removes the record with the given identifier.
    ///
    /// Returns `true` when a row was deleted. Removing the tracking note never
    /// touches the directory on disk.
    fn remove(&self, id: &str) -> Result<bool, ProjectError>;
}

/// Canonicalizes a project path to the stable string persisted in `projects`.
///
/// Shared by [`Projects::register`] and the ingest use case so the allow-list
/// lookup cannot drift from the stored representation. A missing path, a
/// non-directory, or an OS canonicalization failure returns
/// [`ProjectError::InvalidLocation`]; only `canonicalize`/`metadata` are used,
/// the directory is never read or written.
pub fn canonicalize_location(location: &str) -> Result<String, ProjectError> {
    let path = Path::new(location);
    let metadata = std::fs::metadata(path).map_err(|_| ProjectError::InvalidLocation)?;
    if !metadata.is_dir() {
        return Err(ProjectError::InvalidLocation);
    }
    let canonical = std::fs::canonicalize(path).map_err(|_| ProjectError::InvalidLocation)?;
    Ok(normalize_canonical_path(canonical)
        .to_string_lossy()
        .into_owned())
}

/// Project use cases over a [`ProjectRepository`].
pub struct Projects<R> {
    repository: R,
}

impl<R: ProjectRepository> Projects<R> {
    /// Wraps a repository with the Project use cases.
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    /// Registers a directory to track.
    ///
    /// The path is canonicalized; a missing path or a non-directory is rejected
    /// with [`ProjectError::InvalidLocation`] before anything is persisted. The
    /// directory is never opened, read or written.
    pub fn register(&self, location: impl AsRef<Path>) -> Result<Project, ProjectError> {
        let text = location.as_ref().to_string_lossy().into_owned();
        let canonical = canonicalize_location(&text)?;
        let location = ProjectLocation::new(PathBuf::from(canonical));

        let id = ProjectId::new(uuid::Uuid::now_v7().to_string());
        let registered_at = now_rfc3339();
        let record = ProjectRecord::new(
            id.as_str().to_string(),
            location.to_string(),
            registered_at.clone(),
        );
        self.repository.insert(&record)?;

        Ok(Project::new(id, location, registered_at))
    }

    /// Returns the Project with the given identifier.
    pub fn get(&self, id: &str) -> Result<Project, ProjectError> {
        let record = self.repository.get(id)?.ok_or(ProjectError::NotFound)?;
        Ok(project_from_record(record))
    }

    /// Lists every tracked Project as a read model.
    pub fn list(&self) -> Result<Vec<ProjectSummary>, ProjectError> {
        let records = self.repository.list()?;
        Ok(records
            .into_iter()
            .map(|record| {
                let location = ProjectLocation::new(PathBuf::from(&record.location));
                let name = location.display_name();
                ProjectSummary::new(
                    ProjectId::new(record.id),
                    name,
                    location,
                    record.registered_at,
                )
            })
            .collect())
    }

    /// Removes the tracking note for a Project.
    ///
    /// Returns `true` when a Project was removed. Only the persisted row is
    /// deleted; the directory on disk is left untouched.
    pub fn remove(&self, id: &str) -> Result<bool, ProjectError> {
        self.repository.remove(id)
    }
}

/// Rebuilds a [`Project`] from a persisted record.
fn project_from_record(record: ProjectRecord) -> Project {
    Project::new(
        ProjectId::new(record.id),
        ProjectLocation::new(PathBuf::from(record.location)),
        record.registered_at,
    )
}

/// Normalizes a canonical Windows path to a stable, prefix-free representation.
///
/// `std::fs::canonicalize` on Windows returns verbatim paths such as
/// `\\?\C:\work\xemnas`. That prefix is stripped for drive-letter paths (and
/// the equivalent `\\?\UNC\server\share` form is rewritten as
/// `\\server\share`) so the same directory is persisted with a single
/// representation regardless of how it was typed and across restarts. The
/// function is a no-op on other platforms and for verbatim paths it does not
/// recognize.
fn normalize_canonical_path(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = text.strip_prefix(r"\\?\") {
        let bytes = rest.as_bytes();
        let is_drive_path =
            bytes.len() >= 3 && bytes[1] == b':' && matches!(bytes[2], b'\\' | b'/');
        if is_drive_path {
            return PathBuf::from(rest);
        }
    }
    path
}

// The accessor methods on `Project`, `ProjectSummary` and `ProjectLocation`
// are reached directly through the domain re-exports imported above.

#[cfg(test)]
mod tests {
    use super::{
        normalize_canonical_path, ProjectError, ProjectRecord, ProjectRepository, Projects,
    };
    use std::path::PathBuf;

    struct FakeRepository {
        records: std::cell::RefCell<Vec<ProjectRecord>>,
    }

    impl FakeRepository {
        fn new() -> Self {
            Self {
                records: std::cell::RefCell::new(Vec::new()),
            }
        }
    }

    impl ProjectRepository for FakeRepository {
        fn insert(&self, record: &ProjectRecord) -> Result<(), ProjectError> {
            let mut records = self.records.borrow_mut();
            if records
                .iter()
                .any(|existing| existing.location == record.location)
            {
                return Err(ProjectError::AlreadyRegistered);
            }
            records.push(record.clone());
            Ok(())
        }

        fn list(&self) -> Result<Vec<ProjectRecord>, ProjectError> {
            Ok(self.records.borrow().clone())
        }

        fn get(&self, id: &str) -> Result<Option<ProjectRecord>, ProjectError> {
            Ok(self
                .records
                .borrow()
                .iter()
                .find(|record| record.id == id)
                .cloned())
        }

        fn find_by_location(&self, location: &str) -> Result<Option<ProjectRecord>, ProjectError> {
            Ok(self
                .records
                .borrow()
                .iter()
                .find(|record| record.location == location)
                .cloned())
        }

        fn remove(&self, id: &str) -> Result<bool, ProjectError> {
            let mut records = self.records.borrow_mut();
            let before = records.len();
            records.retain(|record| record.id != id);
            Ok(records.len() != before)
        }
    }

    #[test]
    fn windows_verbatim_prefix_is_stripped_for_drive_paths() {
        let normalized = normalize_canonical_path(PathBuf::from(r"\\?\C:\work\xemnas"));
        assert_eq!(normalized, PathBuf::from(r"C:\work\xemnas"));
    }

    #[test]
    fn windows_verbatim_unc_prefix_is_rewritten() {
        let normalized = normalize_canonical_path(PathBuf::from(r"\\?\UNC\server\share\xemnas"));
        assert_eq!(normalized, PathBuf::from(r"\\server\share\xemnas"));
    }

    #[test]
    fn missing_location_is_invalid_and_persists_nothing() {
        let projects = Projects::new(FakeRepository::new());
        let missing = std::env::temp_dir().join("xemnas-application-missing-location");
        assert_eq!(
            projects.register(&missing),
            Err(ProjectError::InvalidLocation)
        );
        assert!(projects.list().unwrap().is_empty());
    }
}
