//! Context selection measured on the user's own data, run by hand on their
//! machine. Nothing it reads or writes is versioned: the database is copied
//! with `VACUUM INTO` (the live file is opened read-only) and the cases, the
//! decision sheet and the per-case report stay beside the cases file. Stdout
//! carries only counts, so the run can be shared without its content.
//!
//! ```text
//! XEMNAS_DOGFOOD_DB=%LOCALAPPDATA%\xemnas\state\app.db
//! XEMNAS_DOGFOOD_CASES=<folder outside the repo>\cases.json
//! cargo test -p storage-sqlite --test dogfood_context -- --ignored --nocapture
//! ```
//!
//! The first run writes `decisions.tsv` (the decisions in force, to label
//! with) and a `cases.json` template. Each case is a real task as typed to
//! the agent, with the ids of the decisions or claims the agent needed.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use application::context::{ContextPack, ContextPacks, ContextProvider, ContextRequest};
use application::injection::render_compact;
use rusqlite::{Connection, OpenFlags};
use storage_sqlite::SqliteStore;

const TEMPLATE: &str = r#"{
  "cases": [
    {
      "project_id": "<project id from decisions.tsv>",
      "task": "<a real task, as typed to the agent>",
      "files": [],
      "required": ["<decision or claim id the agent needed; none for an unrelated task>"]
    }
  ]
}
"#;

fn env_path(name: &str) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("set {name}; see the header of this file"))
}

fn copy_database(live: &Path, copy: &Path) {
    let _ = std::fs::remove_file(copy);
    let source = Connection::open_with_flags(live, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("open the live database read-only");
    source
        .execute("VACUUM INTO ?1", [copy.to_string_lossy()])
        .expect("copy the database");
}

fn write_decision_sheet(copy: &Path, sheet: &Path) {
    let connection = Connection::open(copy).expect("open the copy");
    let mut statement = connection
        .prepare(
            "SELECT project_id, decision_id, question, choice FROM engineering_decisions \
             WHERE status = 'accepted' ORDER BY project_id, confirmed_at",
        )
        .expect("decisions query");
    let rows = statement
        .query_map([], |row| {
            Ok([row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?])
        })
        .expect("decisions")
        .collect::<Result<Vec<[String; 4]>, _>>()
        .expect("decision rows");
    let mut text = String::from("project_id\tdecision_id\tquestion\tchoice\n");
    for row in rows {
        let row = row.map(|field| field.replace(['\t', '\n', '\r'], " "));
        text.push_str(&row.join("\t"));
        text.push('\n');
    }
    std::fs::write(sheet, text).expect("write decisions.tsv");
}

struct Case {
    project_id: String,
    task: String,
    files: Vec<String>,
    required: Vec<String>,
}

fn read_cases(path: &Path) -> Vec<Case> {
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("read cases"))
            .expect("cases.json");
    let text = |value: &serde_json::Value, field: &str| {
        value[field]
            .as_str()
            .unwrap_or_else(|| panic!("case without {field}"))
            .to_string()
    };
    let list = |value: &serde_json::Value, field: &str| {
        value[field]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };
    json["cases"]
        .as_array()
        .expect("a cases array")
        .iter()
        .map(|case| Case {
            project_id: text(case, "project_id"),
            task: text(case, "task"),
            files: list(case, "files"),
            required: list(case, "required"),
        })
        .collect()
}

#[derive(Default)]
struct Totals {
    tp: usize,
    retrieved: usize,
    expected: usize,
    negative_cases: usize,
    contaminated: usize,
}

impl Totals {
    fn add(&mut self, required: &BTreeSet<String>, selected: &BTreeSet<String>) {
        self.tp += selected.intersection(required).count();
        self.retrieved += selected.len();
        self.expected += required.len();
        if required.is_empty() {
            self.negative_cases += 1;
            self.contaminated += usize::from(!selected.is_empty());
        }
    }

    fn line(&self, stage: &str) -> String {
        let ratio = |n: usize, d: usize| {
            if d == 0 {
                "N/A".to_string()
            } else {
                format!("{:.4}", n as f64 / d as f64)
            }
        };
        format!(
            "summary {stage} precision={}/{}={} recall={}/{}={} contaminated={}/{}",
            self.tp,
            self.retrieved,
            ratio(self.tp, self.retrieved),
            self.tp,
            self.expected,
            ratio(self.tp, self.expected),
            self.contaminated,
            self.negative_cases
        )
    }
}

/// Topical items: decisions and the claims that matched the task. Standing
/// rules go with every task and are left out of the score, as in the corpus.
fn topical(pack: &ContextPack) -> BTreeSet<String> {
    pack.decisions
        .iter()
        .map(|d| d.decision_id.clone())
        .chain(
            pack.claims
                .iter()
                .filter(|c| c.matched)
                .map(|c| c.claim_id.clone()),
        )
        .collect()
}

#[test]
#[ignore = "reads the user's local database; run by hand on their machine"]
fn dogfood_context_quality() {
    let live = env_path("XEMNAS_DOGFOOD_DB");
    let cases_path = env_path("XEMNAS_DOGFOOD_CASES");
    let folder = cases_path
        .parent()
        .expect("cases file inside a folder")
        .to_path_buf();
    std::fs::create_dir_all(&folder).expect("dogfood folder");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root");
    assert!(
        !folder
            .canonicalize()
            .expect("dogfood folder")
            .starts_with(repository),
        "keep the dogfood folder outside the repository"
    );
    let copy = folder.join("dogfood-copy.db");
    copy_database(&live, &copy);
    // Opening migrates the copy to the schema this build reads.
    let store = SqliteStore::open(&copy).expect("open the copy");
    write_decision_sheet(&copy, &folder.join("decisions.tsv"));
    if !cases_path.exists() {
        std::fs::write(&cases_path, TEMPLATE).expect("write the template");
        println!("wrote decisions.tsv and a cases.json template; label cases and run again");
        return;
    }

    let cases = read_cases(&cases_path);
    let packs = ContextPacks::new(store);
    let mut pack_totals = Totals::default();
    let mut compact_totals = Totals::default();
    let mut report = String::new();
    println!("case,positive,tp,retrieved,expected,compact_tp,compact_retrieved");
    for (index, case) in cases.iter().enumerate() {
        let pack = packs
            .build_pack(ContextRequest {
                project_id: case.project_id.clone(),
                task: case.task.clone(),
                as_of: None,
                budget_chars: None,
                files: case.files.clone(),
            })
            .unwrap_or_else(|error| panic!("case {index}: {error:?}"));
        let required: BTreeSet<String> = case.required.iter().cloned().collect();
        let selected = topical(&pack);
        let compact: BTreeSet<String> = render_compact(&pack, 300, &BTreeSet::new())
            .map(|block| block.items.into_iter().map(|item| item.id).collect())
            .unwrap_or_default();
        let compact: BTreeSet<String> = compact.intersection(&selected).cloned().collect();
        pack_totals.add(&required, &selected);
        compact_totals.add(&required, &compact);
        println!(
            "{index},{},{},{},{},{},{}",
            !required.is_empty(),
            selected.intersection(&required).count(),
            selected.len(),
            required.len(),
            compact.intersection(&required).count(),
            compact.len()
        );

        report.push_str(&format!("## case {index}: {}\n", case.task));
        for decision in &pack.decisions {
            let mark = if required.contains(&decision.decision_id) {
                "ok   "
            } else {
                "extra"
            };
            report.push_str(&format!(
                "  {mark} {} {} -> {}\n",
                decision.decision_id, decision.question, decision.choice
            ));
        }
        for claim in pack.claims.iter().filter(|c| c.matched) {
            let mark = if required.contains(&claim.claim_id) {
                "ok   "
            } else {
                "extra"
            };
            report.push_str(&format!(
                "  {mark} {} {}\n",
                claim.claim_id, claim.statement
            ));
        }
        for missing in required.difference(&selected) {
            report.push_str(&format!("  miss  {missing}\n"));
        }
    }
    std::fs::write(folder.join("report.txt"), report).expect("write report.txt");
    println!("{}", pack_totals.line("pack"));
    println!("{}", compact_totals.line("compact300"));
}
