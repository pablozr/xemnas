//! What happened in a project since the person last looked: the cue that
//! lets them resume without hunting for what changed.
//!
//! Resuming costs most when nothing says where things stand (a study of
//! 10,000 programming sessions found only one in ten picked up again within a
//! minute), and cues of the previous state shorten it. The briefing is four
//! counts, read from the store; it never lists items.

use serde::{Deserialize, Serialize};

/// Counts of what changed since a moment.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Briefing {
    /// RFC 3339 moment the counts start from (the previous visit).
    pub since: String,
    /// Candidates extracted since then, whatever became of them.
    pub new_candidates: usize,
    /// Decisions confirmed since then.
    pub decisions: usize,
    /// Context blocks sent to agents since then.
    pub deliveries: usize,
    /// Distinct agent sessions that received them.
    pub sessions: usize,
}

impl Briefing {
    /// Whether nothing changed, so there is nothing to say.
    pub fn is_quiet(&self) -> bool {
        self.new_candidates == 0 && self.decisions == 0 && self.deliveries == 0
    }
}
