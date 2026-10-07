//! Projects use cases: register, list and remove tracked directories.

use std::path::{Path, PathBuf};

use domain::projects::{Project, ProjectId, ProjectLocation, ProjectSummary};

use crate::clock::now_rfc3339;

/// A Project row as it is persisted.
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
pub trait ProjectRepository {
    /// Inserts a new record.
    fn insert(&self, record: &ProjectRecord) -> Result<(), ProjectError>;

    /// Returns every record ordered by registration time.
    fn list(&self) -> Result<Vec<ProjectRecord>, ProjectError>;

    /// Returns the record with the given identifier, if any.
    fn get(&self, id: &str) -> Result<Option<ProjectRecord>, ProjectError>;

    /// Returns the record whose normalized canonical location matches, if any.
    fn find_by_location(&self, location: &str) -> Result<Option<ProjectRecord>, ProjectError> {
        Ok(self
            .list()?
            .into_iter()
            .find(|record| record.location == location))
    }

    /// Removes the record with the given identifier.
    fn remove(&self, id: &str) -> Result<bool, ProjectError>;

    /// Counts what [`ProjectRepository::purge`] would delete.
    fn removal_impact(&self, _id: &str) -> Result<RemovalImpact, ProjectError> {
        Ok(RemovalImpact::default())
    }

    /// Deletes the project and every row that belongs to it, in one transaction.
    fn purge(&self, id: &str) -> Result<bool, ProjectError> {
        self.remove(id)
    }
}

/// What removing a project with its data deletes; the repository on disk is never touched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RemovalImpact {
    /// Engineering Decisions, with their revisions, evidence and relations.
    pub decisions: i64,
    /// Decision Candidates.
    pub candidates: i64,
    /// Context Claims.
    pub claims: i64,
    /// Captures, with their artifacts, checkpoints, assessments and jobs.
    pub captures: i64,
    /// Context injection audit rows.
    pub injections: i64,
    /// Components and technologies of the map, with their links.
    pub entities: i64,
}

impl RemovalImpact {
    /// Whether nothing besides the project row would be deleted.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Failure modes of [`Projects::remove_with_data`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemovalError {
    /// The project lookup or the deletion failed.
    Project(ProjectError),
    /// The typed confirmation does not match the project name.
    ConfirmationMismatch,
}

impl RemovalError {
    /// Short, stable code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Project(ProjectError::NotFound) => "not_found",
            Self::Project(_) => "storage",
            Self::ConfirmationMismatch => "confirmation_mismatch",
        }
    }
}

impl std::fmt::Display for RemovalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Project(error) => write!(formatter, "{error}"),
            Self::ConfirmationMismatch => {
                formatter.write_str("digite exatamente o nome do projeto para confirmar")
            }
        }
    }
}

impl std::error::Error for RemovalError {}

impl From<ProjectError> for RemovalError {
    fn from(error: ProjectError) -> Self {
        Self::Project(error)
    }
}

/// Canonicalizes a project path to the stable string persisted in `projects`.
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

/// Finds the registered project for an agent's working directory, if any.
///
/// A directory that is not registered still resolves to a project when it is
/// a git worktree of the repository that project lives in.
///
/// # Errors
///
/// `Storage` on query failure; an invalid or unregistered directory is `Ok(None)`.
pub fn find_project_by_directory<R: ProjectRepository + ?Sized>(
    repository: &R,
    directory: &str,
) -> Result<Option<ProjectRecord>, ProjectError> {
    find_project_with_identity(repository, crate::repo_identity::shared(), directory)
}

/// [`find_project_by_directory`] with an explicit repository identity port.
pub fn find_project_with_identity<R: ProjectRepository + ?Sized>(
    repository: &R,
    identity: &dyn crate::repo_identity::RepoIdentity,
    directory: &str,
) -> Result<Option<ProjectRecord>, ProjectError> {
    let Ok(location) = canonicalize_location(directory) else {
        return Ok(None);
    };
    if let Some(record) = repository.find_by_location(&location)? {
        return Ok(Some(record));
    }

    let Some(common_dir) = identity.common_dir(&location) else {
        return Ok(None);
    };
    Ok(repository
        .list()?
        .into_iter()
        .find(|record| identity.common_dir(&record.location).as_deref() == Some(&common_dir)))
}

/// Project use cases over a [`ProjectRepository`].
pub struct Projects<R> {
    repository: R,
    after_register: Option<crate::graph::MapPreparer>,
}

impl<R: ProjectRepository> Projects<R> {
    /// Wraps a repository with the Project use cases.
    pub fn new(repository: R) -> Self {
        Self {
            repository,
            after_register: None,
        }
    }

    /// Runs `preparer` for each project right after it is registered, so its
    /// map exists before the first analysis. Registration is already off the
    /// UI thread in the app.
    #[must_use]
    pub fn with_map_preparer(mut self, preparer: crate::graph::MapPreparer) -> Self {
        self.after_register = Some(preparer);
        self
    }

    /// Registers a directory to track.
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
        if let Some(prepare) = &self.after_register {
            prepare(id.as_str());
        }

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

    /// Removes a Project that has no data; one with data fails and keeps everything.
    pub fn remove(&self, id: &str) -> Result<bool, ProjectError> {
        self.repository.remove(id)
    }

    /// Counts what [`Projects::remove_with_data`] would delete, for the warning.
    ///
    /// # Errors
    ///
    /// `NotFound` for an unknown project, `Storage` on query failure.
    pub fn removal_impact(&self, id: &str) -> Result<RemovalImpact, ProjectError> {
        self.get(id)?;
        self.repository.removal_impact(id)
    }

    /// Deletes a Project and all its data after the user types its name.
    ///
    /// # Errors
    ///
    /// `confirmation_mismatch` when `typed_name` differs from the project name;
    /// nothing is deleted in that case.
    pub fn remove_with_data(
        &self,
        id: &str,
        typed_name: &str,
    ) -> Result<RemovalImpact, RemovalError> {
        let project = self.get(id)?;
        if typed_name.trim() != project.location().display_name() {
            return Err(RemovalError::ConfirmationMismatch);
        }
        let impact = self.repository.removal_impact(id)?;
        if !self.repository.purge(id)? {
            return Err(RemovalError::Project(ProjectError::NotFound));
        }
        Ok(impact)
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
