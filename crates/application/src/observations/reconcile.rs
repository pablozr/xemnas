//! Cheap gate for the periodic observation reconciler: a project is refreshed
//! only when the size or modification time of a manifest it reads has changed.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::time::SystemTime;

use super::ObservationStore;
use crate::projects::ProjectRepository;

/// Manifests every refresh reads at the project root.
const ROOT_MANIFESTS: [&str; 2] = ["Cargo.toml", "package.json"];

type Stamp = Vec<(String, Option<(u64, SystemTime)>)>;

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
                    .and_then(|m| Some((m.len(), m.modified().ok()?)));
                (path, metadata)
            })
            .collect();
        self.stamps.insert(project_id.to_string(), stamp.clone()) != Some(stamp)
    }
}
