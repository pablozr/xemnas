//! Explicitly invoked: asks the live, consented provider's turn triage about
//! every capture of the relevance evaluation set that the relevance step
//! sends to the triage today, and prints how many labelled decisions it
//! recognizes, how many non-decisions it bars, the calls and the characters
//! sent. Only case names and numbers are printed, never the speech.
//! Double opt-in: `#[ignore]` and `XEMNAS_LIVE_TRIAGE=1`.
//!
//!     $env:XEMNAS_LIVE_TRIAGE = "1"
//!     cargo test --locked -p ai-provider --test triage_live -- --ignored --nocapture

#[path = "../../application/tests/fixtures/relevance_cases.rs"]
mod cases;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use application::analysis::ExtractorFactory;
use application::extract::{assess_relevance, triage_input_chars, CandidateExtractor, Relevance};
use application::paths::AppPaths;
use application::profile::{AiSettings, FileProfileStore};

/// Calls after which a round where every call failed stops, sparing the rest.
const FAIL_FAST_CALLS: usize = 3;

#[test]
#[ignore = "calls the live provider; requires explicit user authorization"]
fn live_turn_triage_accuracy() {
    if std::env::var("XEMNAS_LIVE_TRIAGE").as_deref() != Ok("1") {
        println!("live triage skipped: set XEMNAS_LIVE_TRIAGE=1 to authorize the calls");
        return;
    }
    let paths = AppPaths::from_env();
    let authorized = Path::new(r"C:\Users\Pablo\AppData\Local\xemnas\settings\ai-profile.json");
    assert_eq!(paths.ai_profile, authorized, "live profile path only");

    let settings = AiSettings::new(
        FileProfileStore::new(paths.ai_profile),
        ai_provider::KeyringSecretStore::new(),
    );
    let profile = settings.load_or_seed().expect("profile");
    let secret = settings
        .secret(&profile.credential_account())
        .expect("secret store")
        .unwrap_or_default();
    let factory =
        ai_provider::ProviderFactory::new(Arc::new(ai_provider::ChatGptSession::default()));
    let model = factory
        .authorized(&profile, secret, &settings)
        .expect("authorized provider");

    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../application/tests/fixtures/extraction");
    let (mut structural, mut empty, mut calls, mut chars) = (0usize, 0usize, 0usize, 0usize);
    let (mut decisions, mut recognized) = (0usize, 0usize);
    let (mut non_decisions, mut barred) = (0usize, 0usize);
    let mut errors: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (name, evidence, relevant) in cases::corpus(&dir) {
        let input = match assess_relevance(&evidence) {
            Relevance::Pass(_) => {
                structural += 1;
                continue;
            }
            Relevance::Empty => {
                empty += 1;
                continue;
            }
            Relevance::Triage(input) => input,
        };
        calls += 1;
        chars += triage_input_chars(&input);
        match model.triage(&input) {
            Ok(Some(verdict)) => {
                let passed = verdict.signal().is_some();
                if relevant {
                    decisions += 1;
                    if passed {
                        recognized += 1;
                    } else {
                        println!("missed decision: {name}");
                    }
                } else {
                    non_decisions += 1;
                    if passed {
                        println!("false decision: {name} kind={}", verdict.kind.as_str());
                    } else {
                        barred += 1;
                    }
                }
            }
            Ok(None) => panic!("the live provider must triage"),
            Err(error) => {
                let seen = errors.entry(error.code()).or_default();
                *seen += 1;
                // The first message of each code names the cause; later ones repeat it.
                if *seen == 1 {
                    println!("triage error: {name} code={} message={error}", error.code());
                } else {
                    println!("triage error: {name} code={}", error.code());
                }
                if error.is_deferred() {
                    println!("provider paused; stopping");
                    break;
                }
                if calls == FAIL_FAST_CALLS && errors.values().sum::<usize>() == calls {
                    println!("the first {calls} calls failed; stopping");
                    break;
                }
            }
        }
    }
    println!(
        "live triage model={:?} calls={calls} input_chars={chars} est_tokens={} \
         recognized={recognized}/{decisions} ({:.3}) barred={barred}/{non_decisions} ({:.3}) \
         structural={structural} empty={empty} errors={errors:?}",
        profile.model,
        chars / 4,
        recognized as f64 / decisions.max(1) as f64,
        barred as f64 / non_decisions.max(1) as f64,
    );
    assert!(calls > 0, "the evaluation set sends nothing to the triage");
    assert!(
        errors.values().sum::<usize>() < calls,
        "every triage call failed: {errors:?}"
    );
}
