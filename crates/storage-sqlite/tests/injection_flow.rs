//! Context injection end to end: project by directory, compact block,
//! per-session deduplication, shadow mode and silent no-ops.

mod support;

use application::claims::{Claims, NewClaim};
use application::context_settings::{ContextMode, ContextSettings};
use application::decisions::Decisions;
use application::injection::{ContextInjection, InjectionRequest};
use application::projects::canonicalize_location;
use domain::claims::ClaimKind;
use rusqlite::Connection;

/// Points project `p1` at the real temporary directory so it can be resolved.
fn register_directory(test: &support::TestStore) -> String {
    let directory = test.root.join("workspace");
    std::fs::create_dir_all(&directory).expect("workspace");
    let location = canonicalize_location(&directory.to_string_lossy()).expect("canonical");
    Connection::open(test.root.join("app.db"))
        .expect("raw")
        .execute(
            "UPDATE projects SET location = ?1 WHERE id = 'p1'",
            [&location],
        )
        .expect("relocate project");
    directory.to_string_lossy().into_owned()
}

fn request(path: &str, session: &str, prompt: &str) -> InjectionRequest {
    InjectionRequest {
        canonical_path: path.to_string(),
        session_id: session.to_string(),
        prompt: prompt.to_string(),
    }
}

fn set_mode(test: &support::TestStore, mode: ContextMode, budget: Option<usize>) {
    ContextSettings::new(test.store.clone())
        .set("p1", mode, budget)
        .expect("set mode");
}

fn seed(test: &support::TestStore) {
    support::decision(
        &test.store,
        "p1",
        "cache",
        "Como fazer cache da API?",
        "Cache em memória",
    );
    Claims::new(test.store.clone())
        .create(NewClaim {
            project_id: "p1".to_string(),
            kind: ClaimKind::Convention,
            statement: "Mensagens de erro em português".to_string(),
            valid_from: Some("2020-01-01".to_string()),
            valid_until: None,
            source_decision_id: None,
        })
        .expect("claim");
}

fn injections(test: &support::TestStore) -> i64 {
    Connection::open(test.root.join("app.db"))
        .expect("raw")
        .query_row("SELECT COUNT(*) FROM context_injections", [], |row| {
            row.get(0)
        })
        .expect("count")
}

#[test]
fn injects_once_per_session_and_again_when_a_decision_changes() {
    let test = support::open("inject-flow", &["p1"]);
    let path = register_directory(&test);
    seed(&test);
    set_mode(&test, ContextMode::Inject, None);
    let injection = ContextInjection::new(test.store.clone());

    let first = injection
        .prepare(request(&path, "s1", "melhorar o cache da API"))
        .expect("first turn");
    assert_eq!(first.mode, ContextMode::Inject);
    let block = first.block.expect("block");
    assert!(block.starts_with("<xemnas-context"));
    assert!(block.contains("Como fazer cache da API? → Cache em memória"));
    assert!(
        block.contains("regra:"),
        "standing conventions come on the first turn"
    );
    assert_eq!(first.items, 2);
    assert!(first.tokens < 100, "tokens: {}", first.tokens);

    let second = injection
        .prepare(request(&path, "s1", "cache da API de novo"))
        .expect("second turn");
    assert_eq!(second.block, None, "nothing new in the same session");
    assert_eq!(second.tokens, 0);

    let other_session = injection
        .prepare(request(&path, "s2", "cache da API"))
        .expect("other session");
    assert!(other_session.block.is_some());

    let decision = Decisions::new(test.store.clone())
        .list(&Default::default())
        .expect("list")
        .decisions[0]
        .decision_id
        .clone();
    Decisions::new(test.store.clone())
        .revise(&decision, support::rationale("Novo motivo."))
        .expect("revise");
    let after_revision = injection
        .prepare(request(&path, "s1", "cache da API"))
        .expect("after revision");
    let block = after_revision.block.expect("revised decision is delivered");
    assert!(block.contains(" v2 "));
    assert!(
        !block.contains("regra:"),
        "the convention was already delivered"
    );
    assert_eq!(injections(&test), 3);
}

#[test]
fn off_by_default_computes_and_records_nothing() {
    let test = support::open("inject-off", &["p1"]);
    let path = register_directory(&test);
    seed(&test);
    let outcome = ContextInjection::new(test.store.clone())
        .prepare(request(&path, "s1", "cache da API"))
        .expect("off");
    assert_eq!(outcome.mode, ContextMode::Off);
    assert_eq!((outcome.block, outcome.items, outcome.tokens), (None, 0, 0));
    assert_eq!(injections(&test), 0);
}

#[test]
fn shadow_mode_records_but_returns_no_block() {
    let test = support::open("inject-shadow", &["p1"]);
    let path = register_directory(&test);
    seed(&test);
    set_mode(&test, ContextMode::Shadow, None);
    let injection = ContextInjection::new(test.store.clone());

    let shadow = injection
        .prepare(request(&path, "s1", "cache da API"))
        .expect("shadow");
    assert_eq!(shadow.mode, ContextMode::Shadow);
    assert_eq!(shadow.block, None);
    assert_eq!(shadow.items, 2);
    assert!(shadow.tokens > 0);
    assert_eq!(injections(&test), 1);

    set_mode(&test, ContextMode::Inject, None);
    let live = injection
        .prepare(request(&path, "s1", "cache da API"))
        .expect("inject after shadow");
    assert!(
        live.block.is_some(),
        "shadow deliveries do not count as delivered"
    );
}

#[test]
fn the_project_budget_limits_the_block() {
    let test = support::open("inject-budget", &["p1"]);
    let path = register_directory(&test);
    seed(&test);
    set_mode(&test, ContextMode::Inject, Some(50));
    let outcome = ContextInjection::new(test.store.clone())
        .prepare(request(&path, "s1", "cache da API"))
        .expect("budgeted");
    assert!(outcome.tokens <= 50, "tokens: {}", outcome.tokens);
    assert_eq!(outcome.items + outcome.omitted, 2);
}

#[test]
fn unknown_directories_and_empty_prompts_cost_nothing() {
    let test = support::open("inject-noop", &["p1"]);
    let path = register_directory(&test);
    seed(&test);
    set_mode(&test, ContextMode::Inject, None);
    let injection = ContextInjection::new(test.store.clone());
    let elsewhere = test.root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("dir");

    for request in [
        request(&elsewhere.to_string_lossy(), "s1", "cache da API"),
        request("/does/not/exist", "s1", "cache da API"),
        request(&path, "s1", "   "),
    ] {
        let outcome = injection.prepare(request).expect("no-op");
        assert_eq!((outcome.block, outcome.items), (None, 0));
    }
    assert_eq!(injections(&test), 0);
    assert_eq!(
        injection
            .prepare(request(&path, " ", "api"))
            .map(|_| ())
            .map_err(|error| error.code()),
        Err("invalid_request")
    );
}
