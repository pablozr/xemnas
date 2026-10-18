//! Watches the documentation of the projects for edits, without AI and without
//! reading a single file: one cheap stamp per project (see
//! [`documentation_stamp`]), compared on every pass of the reconciler.
//!
//! A change is not acted on at once. The pass that sees a new stamp marks the
//! project as settling; the next pass, if the stamp is the same, re-reads the
//! folder and queues only the documents whose content changed, so five edits
//! of the same ADR in a few minutes cost one analysis. The first look at a
//! project after the app opens counts as a change, which catches what was
//! edited while the app was closed. A project that was never indexed is left
//! alone, and documents imported from outside the folder are not stamped.

use std::collections::HashMap;

use super::{
    documentation_stamp, CaptureRepository, DocumentError, DocumentStore, Documents,
    ProjectRepository, PROPOSE_PER_RUN,
};

/// What one pass over a project did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchOutcome {
    /// The project or its folder is gone.
    Unavailable,
    /// The stamp changed; it is checked again on the next pass.
    Settling,
    /// Nothing changed since the last pass.
    Unchanged,
    /// The folder was read again; this many analyses were queued.
    Refreshed {
        /// Documents queued for analysis.
        queued: usize,
    },
}

/// The stamp last seen of a project and whether it still has to be acted on.
#[derive(Debug, Clone, Copy)]
struct Seen {
    stamp: u64,
    pending: bool,
}

/// The stamp last seen per project and whether it still has to be acted on.
#[derive(Debug, Default)]
pub struct DocumentWatch {
    state: HashMap<String, Seen>,
}

impl DocumentWatch {
    /// One pass over `project_id`.
    ///
    /// # Errors
    ///
    /// `storage`, or the error of the re-read; the change stays pending and
    /// the next pass tries again.
    pub fn tick<S>(
        &mut self,
        documents: &Documents<S>,
        project_id: &str,
    ) -> Result<WatchOutcome, DocumentError>
    where
        S: DocumentStore + ProjectRepository + CaptureRepository,
    {
        let root = match documents.location(project_id) {
            Ok(root) if root.is_dir() => root,
            Ok(_) | Err(DocumentError::ProjectNotFound) => {
                self.state.remove(project_id);
                return Ok(WatchOutcome::Unavailable);
            }
            Err(error) => return Err(error),
        };
        let stamp = documentation_stamp(&root);
        match self.state.get(project_id).copied() {
            Some(Seen {
                stamp: known,
                pending: true,
            }) if known == stamp => {
                let queued = documents.refresh_changed(project_id, PROPOSE_PER_RUN)?;
                self.state.insert(
                    project_id.to_string(),
                    Seen {
                        stamp,
                        pending: false,
                    },
                );
                Ok(WatchOutcome::Refreshed { queued })
            }
            Some(Seen {
                stamp: known,
                pending: false,
            }) if known == stamp => Ok(WatchOutcome::Unchanged),
            _ => {
                self.state.insert(
                    project_id.to_string(),
                    Seen {
                        stamp,
                        pending: true,
                    },
                );
                Ok(WatchOutcome::Settling)
            }
        }
    }
}
