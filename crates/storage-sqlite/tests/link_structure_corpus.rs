//! Decision -> component links on projects the way people really have them:
//! a Rust workspace whose app has the project's name, a pnpm monorepo, an npm
//! workspaces project and a single-package project. Each project is a small
//! tree on disk (without `.git`, so the file index uses its walk), decisions
//! are labelled with the components they are really about, and the gate
//! compares them with the `affects` edges `assemble` + `refresh_suggestions`
//! derive. Nothing is project specific: the cases cover negation, the
//! project's own name, files cited in documents, dependencies, symbols, the
//! workspace root and CI.
//!
//! Metrics, printed as `gate structural links: ...`:
//! * precision / recall of the links against the labels;
//! * `negated_linked`: links to a part the text only names to exclude it;
//! * `homonym_linked`: links to the part named like the project when the text
//!   speaks of the product (or the name sits inside another path);
//! * `ghost_proposals`: component proposals whose folder does not exist.

mod support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use application::captures::{
    CaptureArtifactRecord, CaptureCheckpointRecord, CaptureReceiptRecord, CaptureRepository,
    CaptureWrite,
};
use application::extract::{DecisionCandidateRecord, ExtractionStore};
use application::graph::{GraphStore, KnowledgeGraph};
use application::inbox::{CandidateEdits, Inbox};
use application::jobs::{JobRecord, JobState, ANALYZE_CAPTURE_KIND};
use application::projects::{ProjectRecord, ProjectRepository};
use domain::entities::{EdgeKind, NodeKind};
use storage_sqlite::SqliteStore;

/// Floors measured on the corpus; raise them when the rule improves. History
/// (2026-10-07): before the structural rules precision 0.487, recall 0.613,
/// 11 negated, 4 homonym, 3 ghost; with affirmative mentions only 0.783
/// (18/23), 0.581 (18/31), 0, 2, 3.
/// with the project's own name restricted: precision 0.857 (18/21), recall 0.581, homonym_linked 0.
/// with the file index: precision 1.000 (20/20), recall 0.645 (20/31), ghost 0; listing of 5,000 files 69 ms.
const PRECISION_FLOOR: f64 = 0.95;
const RECALL_FLOOR: f64 = 0.64;
/// Ceilings: the count of wrong links of each kind.
const NEGATED_CEILING: usize = 0;
const HOMONYM_CEILING: usize = 0;
const GHOST_CEILING: usize = 0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Plain,
    /// The text names a part only to exclude it.
    Negated,
    /// The text speaks of the product, or the name sits inside another path.
    Homonym,
}

/// One labelled decision.
struct Case {
    key: &'static str,
    kind: Kind,
    /// The decision's choice: the text the links are derived from.
    text: &'static str,
    /// `diff_summary.files` of its candidate.
    files: &'static [&'static str],
    /// Taken from a document (an ADR), not from a diff.
    document: bool,
    /// Names of the components it is really about.
    expect: &'static [&'static str],
}

const fn case(
    key: &'static str,
    kind: Kind,
    text: &'static str,
    expect: &'static [&'static str],
) -> Case {
    Case {
        key,
        kind,
        text,
        files: &[],
        document: false,
        expect,
    }
}

const fn code(
    key: &'static str,
    text: &'static str,
    files: &'static [&'static str],
    expect: &'static [&'static str],
) -> Case {
    Case {
        key,
        kind: Kind::Plain,
        text,
        files,
        document: false,
        expect,
    }
}

const fn document(
    key: &'static str,
    text: &'static str,
    files: &'static [&'static str],
    expect: &'static [&'static str],
) -> Case {
    Case {
        key,
        kind: Kind::Plain,
        text,
        files,
        document: true,
        expect,
    }
}

/// A project: its files and its labelled decisions.
struct Fixture {
    folder: &'static str,
    files: &'static [(&'static str, &'static str)],
    /// The component named like the project, when there is one.
    homonym: Option<&'static str>,
    cases: Vec<Case>,
}

/// What the cited paths of the ADR of project `acme` look like: relative to
/// the crate that holds them, with `../`, and a full one.
const ACME_DOCUMENT_FILES: &[&str] = &[
    "docs/adr/0011.md",
    "examples/chat.rs",
    "../core/src/lib.rs",
    "crates/store/src/flush.rs",
];

fn fixtures() -> Vec<Fixture> {
    vec![acme(), shop(), toolkit(), solo()]
}

fn acme() -> Fixture {
    Fixture {
        folder: "acme",
        homonym: Some("acme"),
        files: &[
            (
                "Cargo.toml",
                "[workspace]\nresolver = \"2\"\nmembers = [\"apps/acme\", \"crates/*\", \"tests/arch\"]\n",
            ),
            ("rust-toolchain.toml", "[toolchain]\nchannel = \"1.80\"\n"),
            (".github/workflows/release.yml", "name: release\non: push\n"),
            (
                "apps/acme/Cargo.toml",
                "[package]\nname = \"acme\"\n\n[dependencies]\nclap = \"4\"\nacme-core = { path = \"../../crates/core\" }\n",
            ),
            ("apps/acme/src/main.rs", "fn main() {}\n"),
            (
                "crates/core/Cargo.toml",
                "[package]\nname = \"acme-core\"\n\n[dependencies]\nserde = \"1\"\n",
            ),
            (
                "crates/core/src/lib.rs",
                "pub mod config;\npub mod state;\n",
            ),
            ("crates/core/src/config.rs", "pub struct Config {}\n"),
            (
                "crates/core/src/state.rs",
                "pub struct SessionTable {}\nimpl SessionTable {}\n",
            ),
            (
                "crates/net/Cargo.toml",
                "[package]\nname = \"acme-net\"\n\n[dependencies]\nquinn = \"0.11\"\nserde = \"1\"\n",
            ),
            ("crates/net/src/lib.rs", "pub mod config;\n"),
            ("crates/net/src/config.rs", "pub struct Config {}\n"),
            ("crates/net/examples/chat.rs", "fn main() {}\n"),
            (
                "crates/store/Cargo.toml",
                "[package]\nname = \"acme-store\"\n\n[dependencies]\nrusqlite = \"0.40\"\nserde = \"1\"\n",
            ),
            (
                "crates/store/src/flush.rs",
                "pub const FLUSH_INTERVAL: u64 = 5;\npub fn flush_all() {}\n",
            ),
            (
                "tests/arch/Cargo.toml",
                "[package]\nname = \"acme-arch\"\n\n[dev-dependencies]\ntokio = \"1\"\n",
            ),
            ("tests/arch/src/lib.rs", "// layer rules\n"),
            ("docs/adr/0011.md", "# 0011\n"),
        ],
        cases: vec![
            // Mentions, affirmative.
            case(
                "a01",
                Kind::Plain,
                "O core grava cada captura pela outbox antes de responder.",
                &["acme-core"],
            ),
            case(
                "a02",
                Kind::Negated,
                "The core stays independent of acme-net.",
                &["acme-core"],
            ),
            case(
                "a03",
                Kind::Negated,
                "O core grava direto, sem passar pelo acme-store.",
                &["acme-core"],
            ),
            case(
                "a04",
                Kind::Negated,
                "Use the acme-core validator rather than acme-net parsing.",
                &["acme-core"],
            ),
            case(
                "a05",
                Kind::Negated,
                "Não usar acme-net nem acme-store, mas acme-core valida tudo.",
                &["acme-core"],
            ),
            case(
                "a06",
                Kind::Negated,
                "Não usar acme-net nem acme-store; o acme-net continua dono dos sockets.",
                &["acme-net"],
            ),
            case(
                "a07",
                Kind::Negated,
                "Persistir no acme-store em vez de acme-net.",
                &["acme-store"],
            ),
            case(
                "a08",
                Kind::Negated,
                "Instead of acme-store, write through acme-core.",
                &["acme-core"],
            ),
            // The project's own name.
            case(
                "a09",
                Kind::Homonym,
                "How should acme acquire the license on first run?",
                &[],
            ),
            case(
                "a10",
                Kind::Plain,
                "The `acme` binary parses its flags with clap.",
                &["acme"],
            ),
            case(
                "a11",
                Kind::Homonym,
                "Keep the cache under data_dir()/acme/ by default.",
                &[],
            ),
            case(
                "a12",
                Kind::Plain,
                "Edit apps/acme/src/main.rs to read the flag.",
                &["acme"],
            ),
            case(
                "a13",
                Kind::Plain,
                "Keep the run logs in logs/acme-core/ per run.",
                &[],
            ),
            // Dependencies cited.
            case(
                "a14",
                Kind::Plain,
                "Switch the transport to quinn for 0-RTT.",
                &["acme-net"],
            ),
            case(
                "a15",
                Kind::Plain,
                "Use rusqlite with WAL for the cache.",
                &["acme-store"],
            ),
            case(
                "a16",
                Kind::Negated,
                "Avoid quinn; fall back to TCP.",
                &[],
            ),
            case(
                "a17",
                Kind::Plain,
                "Use serde for all the payloads.",
                &[],
            ),
            // Files cited by a document, and only by the decision that cites them.
            document(
                "a18",
                "O exemplo examples/chat.rs mostra a reconexão.",
                ACME_DOCUMENT_FILES,
                &["acme-net"],
            ),
            document(
                "a19",
                "Reconectar com backoff exponencial.",
                ACME_DOCUMENT_FILES,
                &[],
            ),
            document(
                "a20",
                "Manter ../core/src/lib.rs como único ponto de validação.",
                ACME_DOCUMENT_FILES,
                &["acme-core"],
            ),
            document(
                "a21",
                "Registrar crates/store/src/flush.rs no relatório.",
                ACME_DOCUMENT_FILES,
                &["acme-store"],
            ),
            // Symbols and file names in code.
            case(
                "a22",
                Kind::Plain,
                "Raise `FLUSH_INTERVAL` to ten seconds.",
                &["acme-store"],
            ),
            case(
                "a23",
                Kind::Negated,
                "Do not touch `FLUSH_INTERVAL`; the interval stays.",
                &[],
            ),
            case(
                "a24",
                Kind::Plain,
                "`state.rs` holds the table of open sessions.",
                &["acme-core"],
            ),
            case(
                "a25",
                Kind::Plain,
                "`SessionTable` grows by one row per connection.",
                &["acme-core"],
            ),
            case(
                "a26",
                Kind::Plain,
                "Rename `Config` to Settings.",
                &[],
            ),
            case(
                "a27",
                Kind::Plain,
                "The flush interval is too short for slow disks.",
                &[],
            ),
            // The workspace root and CI.
            code(
                "a28",
                "Fixar a toolchain e subir o MSRV.",
                &["rust-toolchain.toml", "Cargo.toml"],
                &["workspace"],
            ),
            code(
                "a29",
                "Publicar o binário na tag.",
                &[".github/workflows/release.yml"],
                &["CI"],
            ),
            // A rule checked by a test component.
            case(
                "a30",
                Kind::Plain,
                "A regra de camadas é verificada em `acme-arch`.",
                &["acme-arch"],
            ),
            // The same vocabulary in other senses.
            case(
                "a31",
                Kind::Plain,
                "Keep a nets-and-ports table per host.",
                &[],
            ),
            case(
                "a32",
                Kind::Plain,
                "Use coreutils-style flag parsing.",
                &[],
            ),
            case(
                "a33",
                Kind::Plain,
                "The storage engine is chosen per tenant.",
                &[],
            ),
        ],
    }
}

fn shop() -> Fixture {
    Fixture {
        folder: "shop",
        homonym: Some("shop"),
        files: &[
            ("package.json", "{\"name\": \"shop\", \"private\": true}"),
            (
                "pnpm-workspace.yaml",
                "packages:\n  - 'packages/*'\n  - 'apps/*'\n",
            ),
            ("packages/web/package.json", "{\"name\": \"@shop/web\"}"),
            ("packages/web/src/index.ts", "export const page = 1;\n"),
            (
                "packages/api/package.json",
                "{\"name\": \"@shop/api\", \"dependencies\": {\"fastify\": \"^5.0.0\"}}",
            ),
            ("packages/api/src/server.ts", "export function start() {}\n"),
            ("packages/ui/package.json", "{\"name\": \"@shop/ui\"}"),
            ("packages/ui/src/tokens.ts", "export const gap = 8;\n"),
            ("apps/shop/package.json", "{\"name\": \"shop\"}"),
            ("apps/shop/src/main.ts", "export {};\n"),
        ],
        cases: vec![
            case(
                "b01",
                Kind::Plain,
                "The checkout page in `web` calls the cart endpoint.",
                &["@shop/web"],
            ),
            case(
                "b02",
                Kind::Plain,
                "Use fastify schemas to validate request bodies.",
                &["@shop/api"],
            ),
            case(
                "b03",
                Kind::Homonym,
                "How should shop price its bundles?",
                &[],
            ),
            case(
                "b04",
                Kind::Plain,
                "The `shop` shell registers routes lazily.",
                &["shop"],
            ),
            case(
                "b05",
                Kind::Homonym,
                "Serve the assets from public/shop/ in the CDN bucket.",
                &[],
            ),
            case(
                "b06",
                Kind::Negated,
                "`api` must not call `web`.",
                &["@shop/api"],
            ),
            case(
                "b07",
                Kind::Plain,
                "The design tokens live in packages/ui.",
                &["@shop/ui"],
            ),
            case(
                "b08",
                Kind::Negated,
                "Do not edit packages/web directly; go through the endpoint.",
                &[],
            ),
            code(
                "b09",
                "Ligar o workspace pnpm aos pacotes.",
                &["pnpm-workspace.yaml"],
                &["workspace"],
            ),
        ],
    }
}

fn toolkit() -> Fixture {
    Fixture {
        folder: "toolkit",
        homonym: None,
        files: &[
            (
                "package.json",
                "{\"name\": \"toolkit\", \"workspaces\": [\"libs/*\"]}",
            ),
            ("tsconfig.base.json", "{}"),
            ("libs/core/package.json", "{\"name\": \"@x/core\"}"),
            ("libs/core/src/index.ts", "export const one = 1;\n"),
            (
                "libs/cli/package.json",
                "{\"name\": \"@x/cli\", \"dependencies\": {\"commander\": \"^12.0.0\"}}",
            ),
            ("libs/cli/src/index.ts", "export const two = 2;\n"),
        ],
        cases: vec![
            case(
                "c01",
                Kind::Plain,
                "The core exposes a plain-object API.",
                &["@x/core"],
            ),
            case("c02", Kind::Plain, "Core-js polyfills are not needed.", &[]),
            case(
                "c03",
                Kind::Plain,
                "Parse the flags with commander.",
                &["@x/cli"],
            ),
            case(
                "c04",
                Kind::Negated,
                "Independent of core; the `cli` prints help.",
                &["@x/cli"],
            ),
            code(
                "c05",
                "Compartilhar a configuração do compilador.",
                &["tsconfig.base.json"],
                &["workspace"],
            ),
        ],
    }
}

fn solo() -> Fixture {
    Fixture {
        folder: "solo",
        homonym: None,
        files: &[
            (
                "package.json",
                "{\"name\": \"solo\", \"dependencies\": {\"left-pad\": \"1.3.0\"}}",
            ),
            ("src/index.ts", "export const solo = 1;\n"),
            ("src/util.ts", "export const util = 2;\n"),
        ],
        cases: vec![
            code(
                "d01",
                "Usar left-pad para alinhar a saída.",
                &["package.json"],
                &[],
            ),
            document(
                "d02",
                "Rode examples/demo.ts para ver o fluxo.",
                &["docs/adr/0001.md", "examples/demo.ts", "src/index.ts"],
                &[],
            ),
            code("d03", "Dividir o utilitário.", &["src/util.ts"], &[]),
        ],
    }
}

fn write_tree(root: &Path, files: &[(&str, &str)]) {
    for (relative, content) in files {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create folder");
        std::fs::write(path, content).expect("write file");
    }
}

/// A decision as the extraction leaves it: a capture with one artifact (a
/// diff hunk or a document), a candidate whose summary lists `files`, and the
/// confirmation that makes it a decision.
fn confirm_decision(store: &SqliteStore, project: &str, location: &str, case: &Case) -> String {
    seed_decision(
        store,
        project,
        location,
        &Seed {
            key: case.key,
            text: case.text,
            files: case.files,
            document: case.document,
        },
    )
}

/// The parts of a decision the store needs.
struct Seed<'a, F: AsRef<str>> {
    key: &'a str,
    text: &'a str,
    files: &'a [F],
    document: bool,
}

fn seed_decision<F: AsRef<str>>(
    store: &SqliteStore,
    project: &str,
    location: &str,
    case: &Seed<'_, F>,
) -> String {
    let key = case.key;
    let capture = format!("capture-{key}");
    let at = "2026-01-02T00:00:00Z".to_string();
    let (kind, content) = if case.document {
        ("document", format!("# Document\n\n{}\n", case.text))
    } else {
        ("diff_hunk", "diff --git a/x b/x\n+x\n".to_string())
    };
    store
        .insert_capture(&CaptureWrite {
            receipt: CaptureReceiptRecord {
                capture_id: capture.clone(),
                idempotency_key: format!("key-{capture}"),
                canonical_path: location.to_string(),
                received_at: at.clone(),
                artifact_count: 1,
            },
            artifacts: vec![CaptureArtifactRecord {
                capture_id: capture.clone(),
                artifact_id: format!("art-{key}"),
                kind: kind.to_string(),
                content,
                metadata: "{}".to_string(),
                fingerprint: format!("{key:0>64}"),
            }],
            job: JobRecord {
                id: format!("job-{capture}"),
                kind: ANALYZE_CAPTURE_KIND.to_string(),
                payload: capture.clone(),
                state: JobState::Queued,
                idempotent: true,
                attempts: 0,
                last_error: None,
                created_at: at.clone(),
                updated_at: at.clone(),
            },
            checkpoint: CaptureCheckpointRecord {
                adapter: "opencode".to_string(),
                adapter_version: "0.1.0".to_string(),
                session_id: format!("session-{key}"),
                message_id: format!("message-{key}"),
                capture_id: capture.clone(),
                observed_at: at.clone(),
                updated_at: at.clone(),
            },
        })
        .expect("seed capture");
    let id = format!("cand-{key}");
    store
        .insert_candidates(&[DecisionCandidateRecord {
            qualifiers: "[]".into(),
            id: id.clone(),
            project_id: project.to_string(),
            capture_id: capture,
            status: "pending".to_string(),
            question: format!("Qual abordagem seguir no caso {key}?"),
            choice: case.text.to_string(),
            rationale: "Motivo registrado na revisão.".to_string(),
            signals: "[\"public_contract\"]".to_string(),
            confidence: 0.7,
            confidence_reason: "sintético".to_string(),
            evidence_refs: format!("[\"art-{key}\"]"),
            diff_summary: serde_json::json!({
                "files": case.files.iter().map(AsRef::as_ref).collect::<Vec<&str>>(),
                "artifacts": 1
            })
            .to_string(),
            dedup_hash: format!("dedup-{key}"),
            created_at: at.clone(),
            updated_at: at,
            kind: "decision".to_string(),
            significance: 1.0,
            criteria: "[]".to_string(),
        }])
        .expect("insert candidate");
    let edits: Option<CandidateEdits> = None;
    Inbox::new(store.clone())
        .confirm(&id, edits)
        .unwrap_or_else(|error| panic!("confirm {key}: {error:?}"))
        .decision_id
}

fn register(store: &SqliteStore, project: &str, location: &Path) {
    store
        .insert(&ProjectRecord::new(
            project.to_string(),
            location.to_string_lossy().to_string(),
            "2026-01-01T00:00:00Z".to_string(),
        ))
        .expect("register project");
}

/// Whether the folder of a proposal's literal pattern exists.
fn folder_exists(root: &Path, pattern: &str) -> bool {
    let literal = pattern
        .trim_end_matches("/**")
        .trim_end_matches("/*")
        .trim_end_matches('/');
    literal.is_empty()
        || literal.contains('*')
        || (!literal.split('/').any(|part| part == "..") && root.join(literal).exists())
}

/// Whether a count stays within its ceiling (a ceiling of zero is a plain
/// equality, which the compiler lints when written inline).
fn at_most(count: usize, ceiling: usize) -> bool {
    count <= ceiling
}

#[derive(Default)]
struct Tally {
    found: usize,
    expected: usize,
    correct: usize,
    negated_linked: usize,
    homonym_linked: usize,
    ghost_proposals: usize,
}

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("xemnas-linkstruct-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch folder");
    root
}

#[test]
fn link_structure_quality_gate() {
    let base = scratch("gate");
    let test = support::open("link-structure", &[]);
    let graph = KnowledgeGraph::new(test.store.clone());
    let mut tally = Tally::default();
    for (index, fixture) in fixtures().into_iter().enumerate() {
        let project = format!("p{index}");
        let root = base.join(fixture.folder);
        write_tree(&root, fixture.files);
        register(&test.store, &project, &root);
        let location = root.to_string_lossy().to_string();
        let mut decisions: Vec<(String, &Case)> = Vec::new();
        for case in &fixture.cases {
            decisions.push((
                confirm_decision(&test.store, &project, &location, case),
                case,
            ));
        }
        graph.assemble(&project).expect("assemble");
        let report = graph.refresh_suggestions(&project).expect("refresh");
        tally.ghost_proposals += report
            .components
            .iter()
            .filter(|proposal| proposal.declared.is_none())
            .filter(|proposal| !folder_exists(&root, &proposal.pattern))
            .inspect(|proposal| println!("  ghost {} {}", fixture.folder, proposal.pattern))
            .count();

        let entities = test.store.project_entities(&project).expect("entities");
        let name_of = |entity_id: &str| {
            entities
                .iter()
                .find(|entity| entity.entity_id == entity_id)
                .map(|entity| entity.name.clone())
                .unwrap_or_default()
        };
        let edges = test.store.project_edges(&project).expect("edges");
        for (decision_id, case) in &decisions {
            let linked: BTreeSet<String> = edges
                .iter()
                .filter(|edge| {
                    edge.kind == EdgeKind::Affects
                        && edge.source_kind == NodeKind::Decision
                        && edge.source_id == *decision_id
                        && edge.is_live()
                })
                .map(|edge| name_of(&edge.entity_id))
                .collect();
            let expect: BTreeSet<String> =
                case.expect.iter().map(|name| name.to_string()).collect();
            tally.found += linked.len();
            tally.expected += expect.len();
            tally.correct += linked.intersection(&expect).count();
            let wrong: Vec<&String> = linked.difference(&expect).collect();
            let missed: Vec<&String> = expect.difference(&linked).collect();
            if !wrong.is_empty() || !missed.is_empty() {
                println!("  case {}: wrong {wrong:?} missed {missed:?}", case.key);
            }
            if case.kind == Kind::Negated {
                tally.negated_linked += wrong.len();
            }
            if case.kind == Kind::Homonym {
                tally.homonym_linked += wrong
                    .iter()
                    .filter(|name| Some(name.as_str()) == fixture.homonym)
                    .count();
            }
        }
    }
    let _ = std::fs::remove_dir_all(&base);
    let precision = tally.correct as f64 / tally.found.max(1) as f64;
    let recall = tally.correct as f64 / tally.expected.max(1) as f64;
    println!(
        "gate structural links: precision {precision:.3} ({}/{}) recall {recall:.3} ({}/{}) \
         negated_linked {} homonym_linked {} ghost_proposals {}",
        tally.correct,
        tally.found,
        tally.correct,
        tally.expected,
        tally.negated_linked,
        tally.homonym_linked,
        tally.ghost_proposals,
    );
    assert!(precision >= PRECISION_FLOOR, "precision {precision:.3}");
    assert!(recall >= RECALL_FLOOR, "recall {recall:.3}");
    assert!(
        at_most(tally.negated_linked, NEGATED_CEILING),
        "negated_linked {}",
        tally.negated_linked
    );
    assert!(
        at_most(tally.homonym_linked, HOMONYM_CEILING),
        "homonym_linked {}",
        tally.homonym_linked
    );
    assert!(
        at_most(tally.ghost_proposals, GHOST_CEILING),
        "ghost_proposals {}",
        tally.ghost_proposals
    );
}

/// A decision of the scale project.
struct Bulk {
    key: String,
    text: String,
    file: String,
    document: bool,
}

/// Writes confirmed decisions in one transaction, only in the tables the
/// graph reads (the review bookkeeping that `Inbox::confirm` also writes
/// grows with the number of decisions and would take minutes at this scale).
fn bulk_decisions(database: &Path, project: &str, location: &str, decisions: &[Bulk]) {
    let mut connection = rusqlite::Connection::open(database).expect("open the database");
    connection
        .execute_batch("PRAGMA synchronous = OFF; PRAGMA foreign_keys = ON;")
        .expect("pragmas");
    let at = "2026-01-02T00:00:00Z";
    let transaction = connection.transaction().expect("transaction");
    for decision in decisions {
        let key = &decision.key;
        let capture = format!("capture-{key}");
        let artifact = format!("art-{key}");
        let kind = if decision.document {
            "document"
        } else {
            "diff_hunk"
        };
        let summary = serde_json::json!({ "files": [decision.file], "artifacts": 1 }).to_string();
        let question = format!("Qual abordagem seguir no caso {key}?");
        transaction
            .execute(
                "INSERT INTO capture_receipts VALUES (?1, ?2, ?3, ?4, 1)",
                rusqlite::params![capture, format!("key-{capture}"), location, at],
            )
            .expect("receipt");
        transaction
            .execute(
                "INSERT INTO capture_artifacts VALUES (?1, ?2, ?3, ?4, '{}', ?5)",
                rusqlite::params![
                    capture,
                    artifact,
                    kind,
                    decision.text,
                    format!("{key:0>64}")
                ],
            )
            .expect("artifact");
        transaction
            .execute(
                "INSERT INTO decision_candidates (id, project_id, capture_id, status, question,                  choice, rationale, signals, confidence, confidence_reason, evidence_refs,                  diff_summary, dedup_hash, created_at, updated_at)                  VALUES (?1, ?2, ?3, 'accepted', ?4, ?5, 'Motivo.', '[]', 0.7, 'sintético',                  ?6, ?7, ?8, ?9, ?9)",
                rusqlite::params![
                    format!("cand-{key}"),
                    project,
                    capture,
                    question,
                    decision.text,
                    format!("[\"{artifact}\"]"),
                    summary,
                    format!("dedup-{key}"),
                    at,
                ],
            )
            .expect("candidate");
        transaction
            .execute(
                "INSERT INTO engineering_decisions (decision_id, candidate_id, project_id,                  capture_id, status, question, choice, rationale, created_at, confirmed_at,                  updated_at) VALUES (?1, ?2, ?3, ?4, 'accepted', ?5, ?6, 'Motivo.', ?7, ?7, ?7)",
                rusqlite::params![
                    format!("dec-{key}"),
                    format!("cand-{key}"),
                    project,
                    capture,
                    question,
                    decision.text,
                    at,
                ],
            )
            .expect("decision");
        transaction
            .execute(
                "INSERT INTO evidence_links VALUES (?1, ?2, 0, ?3)",
                rusqlite::params![format!("dec-{key}"), artifact, at],
            )
            .expect("evidence");
    }
    transaction.commit().expect("commit");
}

/// Components in the scale project.
const SCALE_COMPONENTS: usize = 60;
/// Decisions in the scale project.
const SCALE_DECISIONS: usize = 2_000;
/// Files in the scale tree, 2,000 of them code of about 8 KB.
const SCALE_FILES: usize = 5_000;
const SCALE_CODE_FILES: usize = 2_000;

/// Ceilings, about twice the baseline (cold 2.1 s, warm 1.3 s).
const REFRESH_COLD_CEILING_MS: u128 = 4_500;
const REFRESH_WARM_CEILING_MS: u128 = 3_000;
/// The listing of the 5,000 files.
const INDEX_CEILING_MS: u128 = 300;

#[test]
#[ignore = "scale gate (seeds 2,000 decisions and 5,000 files); run by tools/core-quality.py"]
fn link_structure_scales() {
    let base = scratch("scale");
    let root = base.join("big");
    let members: Vec<String> = (0..SCALE_COMPONENTS)
        .map(|index| format!("\"crates/part{index:02}\""))
        .collect();
    std::fs::create_dir_all(&root).expect("root");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[workspace]\nmembers = [{}]\n", members.join(", ")),
    )
    .expect("root manifest");
    let filler: String = (0..250)
        .map(|line| format!("pub fn generated_{line}(input: u64) -> u64 {{ input + {line} }}\n"))
        .collect();
    let mut written = 0;
    for part in 0..SCALE_COMPONENTS {
        let dir = root.join(format!("crates/part{part:02}"));
        std::fs::create_dir_all(dir.join("src")).expect("crate folder");
        std::fs::write(
            dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"part{part:02}\"\n\n[dependencies]\ndep{part:02} = \"1\"\n"
            ),
        )
        .expect("manifest");
        written += 1;
    }
    let mut code = 0;
    let mut index = 0;
    while written < SCALE_FILES {
        let part = index % SCALE_COMPONENTS;
        let dir = root.join(format!(
            "crates/part{part:02}/src/m{}",
            index / SCALE_COMPONENTS / 20
        ));
        std::fs::create_dir_all(&dir).expect("module folder");
        if code < SCALE_CODE_FILES {
            std::fs::write(dir.join(format!("file{index}.rs")), &filler).expect("code file");
            code += 1;
        } else {
            std::fs::write(dir.join(format!("note{index}.md")), "# note\n").expect("note");
        }
        written += 1;
        index += 1;
    }

    let test = support::open("link-structure-scale", &[]);
    register(&test.store, "big", &root);
    let location = root.to_string_lossy().to_string();
    let texts = [
        "O part{a} grava pela outbox antes de responder.",
        "Usar dep{a} para a compressão.",
        "Sem part{a}, o fluxo continua igual.",
        "Subir `generated_{a}` para o próximo lote.",
        "Revisar crates/part{a}/src/m0/file{a}.rs com cuidado.",
    ];
    let decisions: Vec<Bulk> = (0..SCALE_DECISIONS)
        .map(|number| {
            let a = format!("{:02}", number % SCALE_COMPONENTS);
            Bulk {
                key: format!("s{number:04}"),
                text: format!(
                    "{} Lote {number}.",
                    texts[number % texts.len()].replace("{a}", &a)
                ),
                file: format!("crates/part{a}/src/m0/file{}.rs", number % 60),
                document: number % 2 == 0,
            }
        })
        .collect();
    bulk_decisions(&test.root.join("app.db"), "big", &location, &decisions);

    let graph = KnowledgeGraph::new(test.store.clone());
    graph.assemble("big").expect("assemble");
    let started = Instant::now();
    let cold = graph.refresh_suggestions("big").expect("refresh");
    let cold_ms = started.elapsed().as_millis();
    let started = Instant::now();
    graph.refresh_suggestions("big").expect("refresh again");
    let warm_ms = started.elapsed().as_millis();
    // The listing alone, without the cache.
    let started = Instant::now();
    let (listed, partial) = application::graph::index_repository(&root).expect("listing");
    let index_ms = started.elapsed().as_millis();
    println!(
        "latency refresh_ms={cold_ms} refresh_warm_ms={warm_ms} index_ms={index_ms}          files={listed} new_edges={}",
        cold.new_edges
    );
    assert!(!partial && listed >= SCALE_FILES, "listed {listed} files");
    assert!(index_ms <= INDEX_CEILING_MS, "listing {index_ms}ms");
    let _ = std::fs::remove_dir_all(&base);
    assert!(
        cold_ms <= REFRESH_COLD_CEILING_MS,
        "cold refresh {cold_ms}ms"
    );
    assert!(
        warm_ms <= REFRESH_WARM_CEILING_MS,
        "warm refresh {warm_ms}ms"
    );
}
