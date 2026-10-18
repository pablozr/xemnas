//! Cheap gate for the periodic observation reconciler: a project is refreshed
//! only when the size or modification time of a manifest it reads has changed.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use super::{ObservationStore, ROOT_MANIFESTS};
use crate::projects::ProjectRepository;

type Stamp = Vec<(String, crate::documents::FileStamp)>;

/// Remembers, in memory, the (size, mtime) of each project's manifests.
///
/// The first look at a project after the app opens always reports a change, so
/// edits made while the app was closed are still picked up. A workspace member
/// that appears through a glob without any change to the root manifest is seen
/// only on the next capture or edit trigger, or when the app reopens.
#[derive(Debug, Default)]
pub struct ReconcileGate {
    stamps: HashMap<String, Stamp>,
}

impl ReconcileGate {
    /// Whether the project's manifests differ from the last look; any store or
    /// project error reports a change, so the reconciler fails open as before.
    pub fn changed<S: ObservationStore + ProjectRepository>(
        &mut self,
        store: &S,
        project_id: &str,
    ) -> bool {
        let Ok(Some(project)) = store.get(project_id) else {
            return true;
        };
        let Ok(sources) = store.current_sources(project_id) else {
            return true;
        };
        let paths: BTreeSet<String> = ROOT_MANIFESTS
            .iter()
            .map(|path| (*path).to_string())
            .chain(sources.into_iter().map(|s| s.project_relative_path))
            .collect();
        let root = Path::new(&project.location);
        let stamp: Stamp = paths
            .into_iter()
            .map(|path| {
                let metadata = std::fs::metadata(root.join(&path))
                    .ok()
                    .and_then(|m| crate::documents::file_stamp(&m));
                (path, metadata)
            })
            .collect();
        if self.stamps.get(project_id) == Some(&stamp) {
            return false;
        }
        self.stamps.insert(project_id.to_string(), stamp);
        true
    }

    /// One reconciler look at a project: schedules a refresh only when
    /// [`Self::changed`] says so, and returns whether it did.
    pub fn reconcile<S: ObservationStore + ProjectRepository>(
        &mut self,
        store: &S,
        project_id: &str,
    ) -> Result<bool, super::ObservationError> {
        if !self.changed(store, project_id) {
            return Ok(false);
        }
        store.request_refresh(&super::RefreshRequest {
            project_id: project_id.to_string(),
            capture_trigger: "reconciler".into(),
            requested_at: crate::clock::now_rfc3339(),
        })?;
        Ok(true)
    }
}
