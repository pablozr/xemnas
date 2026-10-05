//! When the person last looked at each project, kept beside the other
//! interface settings (`settings/visits.json` in the data folder).
//!
//! The Revisão opens with what changed since then, so resuming starts from a
//! cue instead of a hunt. Like the appearance, this is a preference of the
//! interface and not product data: a JSON map from project id to an RFC 3339
//! moment, tolerant of a missing or broken file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use gpui::{App, Global};

/// Where the visits are saved; set by the composition root when the data
/// folder is known.
pub struct VisitsFile(pub PathBuf);

impl Global for VisitsFile {}

/// A fixed previous visit, for demo captures.
pub struct DemoVisit(pub String);

impl Global for DemoVisit {}

fn load(path: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save(path: &Path, visits: &BTreeMap<String, String>) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(visits).map_err(std::io::Error::other)?;
    std::fs::write(path, text)
}

/// The moment the project was last looked at, if ever.
pub fn previous(cx: &App, project: &str) -> Option<String> {
    if let Some(demo) = cx.try_global::<DemoVisit>() {
        return Some(demo.0.clone());
    }
    let file = cx.try_global::<VisitsFile>()?;
    load(&file.0).get(project).cloned()
}

/// Records that the project was looked at `at`. A failed save is logged and
/// otherwise ignored: the briefing is a convenience.
pub fn mark(cx: &App, project: &str, at: &str) {
    let Some(file) = cx.try_global::<VisitsFile>() else {
        return;
    };
    let mut visits = load(&file.0);
    visits.insert(project.to_owned(), at.to_owned());
    if let Err(error) = save(&file.0, &visits) {
        tracing::warn!(error = %error, operation = "save_visits", "could not save");
    }
}

/// The current moment as an RFC 3339 UTC timestamp.
pub fn now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visits_round_trip_and_tolerate_a_broken_file() {
        let dir = std::env::temp_dir().join(format!("xemnas-visits-{}", std::process::id()));
        let path = dir.join("visits.json");
        let mut visits = BTreeMap::new();
        visits.insert("p1".to_owned(), "2026-10-01T10:00:00Z".to_owned());
        save(&path, &visits).expect("save");
        assert_eq!(load(&path), visits);
        std::fs::write(&path, "not json").expect("write");
        assert!(load(&path).is_empty());
        assert!(load(&dir.join("missing.json")).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
