//! Links to components measured on the user's own data, run by hand on their
//! machine. Nothing it reads or writes is versioned: the database is copied
//! with `VACUUM INTO` (the given file is opened read-only, so even a copy of
//! the live one is never changed), the copy is migrated and refreshed in every
//! project, and stdout carries only counts, so the run can be shared without
//! its content. Set `XEMNAS_DOGFOOD_NAMES=1` to also print the names of the
//! phantom components and of the parts whose links were dropped.
//!
//! ```text
//! XEMNAS_DOGFOOD_DB=<a copy of the database>
//! XEMNAS_DOGFOOD_OUT=<folder outside the repository>   (default: beside the database)
//! cargo test -p storage-sqlite --test dogfood_links -- --ignored --nocapture
//! ```
//!
//! No AI is called: the refresh only derives, revalidates and queues jobs in
//! the copy, and `calls avoided` counts the decisions that no longer need the
//! AI to find a place for them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use application::graph::{GraphStore, KnowledgeGraph};
use application::link_suggestions::needs_links;
use rusqlite::{Connection, OpenFlags};
use storage_sqlite::SqliteStore;

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

fn count(connection: &Connection, sql: &str) -> i64 {
    connection.query_row(sql, [], |row| row.get(0)).unwrap_or(0)
}

#[test]
#[ignore = "reads the user's local database; run by hand on their machine"]
fn dogfood_link_counts() {
    let live = env_path("XEMNAS_DOGFOOD_DB").expect("set XEMNAS_DOGFOOD_DB; see the header");
    let folder = env_path("XEMNAS_DOGFOOD_OUT").unwrap_or_else(|| {
        live.parent()
            .expect("database inside a folder")
            .join("links-copy")
    });
    std::fs::create_dir_all(&folder).expect("output folder");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root");
    assert!(
        !folder
            .canonicalize()
            .expect("output folder")
            .starts_with(repository),
        "keep the output folder outside the repository"
    );
    let copy = folder.join("dogfood-links.db");
    let _ = std::fs::remove_file(&copy);
    Connection::open_with_flags(&live, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("open the database read-only")
        .execute("VACUUM INTO ?1", [copy.to_string_lossy()])
        .expect("copy the database");

    // The schema before this build: human edges, to count what is relabeled.
    let human_before = {
        let raw = Connection::open(&copy).expect("open the copy");
        count(
            &raw,
            "SELECT COUNT(*) FROM entity_edges WHERE origin = 'human'",
        )
    };
    let store = SqliteStore::open(&copy).expect("open and migrate the copy");
    let names = std::env::var_os("XEMNAS_DOGFOOD_NAMES").is_some();
    let graph = KnowledgeGraph::new(store.clone());

    let projects: Vec<String> = {
        let raw = Connection::open(&copy).expect("open the copy");
        let mut statement = raw
            .prepare("SELECT id FROM projects ORDER BY id")
            .expect("projects");
        statement
            .query_map([], |row| row.get(0))
            .expect("rows")
            .collect::<Result<_, _>>()
            .expect("ids")
    };
    {
        let raw = Connection::open(&copy).expect("open the copy");
        let human_after = count(
            &raw,
            "SELECT COUNT(*) FROM entity_edges WHERE origin = 'human'",
        );
        println!(
            "dogfood links: migration relabeled {} human edges as derived; by actor now: {}",
            human_before - human_after,
            ["person", "rules", "ai", "inherited"]
                .iter()
                .map(|actor| format!(
                    "{actor}={}",
                    count(
                        &raw,
                        &format!(
                            "SELECT COUNT(*) FROM entity_edges WHERE confirmed_by = '{actor}'"
                        )
                    )
                ))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }

    for (number, project) in projects.iter().enumerate() {
        let edges_before = store.project_edges(project).expect("edges");
        let before_ids: BTreeSet<String> = edges_before
            .iter()
            .map(|edge| edge.edge_id.clone())
            .collect();
        let decisions = store.project_decisions(project).expect("decisions");
        let needing_before = decisions
            .iter()
            .filter(|decision| needs_links(&edges_before, &decision.decision_id))
            .count();
        let report = match graph.refresh_suggestions(project) {
            Ok(report) => report,
            Err(error) => {
                println!("project {}: refresh failed: {}", number + 1, error.code());
                continue;
            }
        };
        let edges_after = store.project_edges(project).expect("edges");
        let needing_after = decisions
            .iter()
            .filter(|decision| needs_links(&edges_after, &decision.decision_id))
            .count();
        let structural = |reason: &str| {
            application::graph::mention_quote(reason).is_none()
                && application::graph::ai_link_quote(reason).is_none()
                && !reason.is_empty()
        };
        let new_structural = edges_after
            .iter()
            .filter(|edge| !before_ids.contains(&edge.edge_id) && structural(&edge.reason))
            .count();
        let new_mentions = edges_after
            .iter()
            .filter(|edge| {
                !before_ids.contains(&edge.edge_id)
                    && application::graph::mention_quote(&edge.reason).is_some()
            })
            .count();
        println!(
            "project {}: decisions={} edges {}->{} revalidated={} new_structural={} \
             new_mentions={} phantoms={} needing_ai {}->{} (calls avoided {})",
            number + 1,
            decisions.len(),
            edges_before.len(),
            edges_after.len(),
            report.revalidated,
            new_structural,
            new_mentions,
            report.stale_components.len(),
            needing_before,
            needing_after,
            needing_before.saturating_sub(needing_after),
        );
        if names {
            let entities = store.project_entities(project).expect("entities");
            let name_of = |id: &str| {
                entities
                    .iter()
                    .find(|entity| entity.entity_id == id)
                    .map_or("?", |entity| entity.name.as_str())
            };
            for stale in &report.stale_components {
                println!(
                    "  phantom `{}` {:?} owner {:?}",
                    stale.name, stale.patterns, stale.owner
                );
            }
            let mut dropped: BTreeMap<&str, usize> = BTreeMap::new();
            for edge in &edges_after {
                let was_live = edges_before
                    .iter()
                    .find(|before| before.edge_id == edge.edge_id)
                    .is_some_and(|before| before.invalidated_at.is_none());
                if was_live && edge.invalidated_at.is_some() {
                    *dropped.entry(name_of(&edge.entity_id)).or_default() += 1;
                }
            }
            for (name, links) in dropped {
                println!("  dropped {links} link(s) to `{name}`");
            }
        }
    }
}
