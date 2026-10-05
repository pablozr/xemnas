//! Explicitly invoked, read-only preflight of the authorized live AI profile.

use application::paths::AppPaths;
use application::profile::{
    build_preview, consent_status, grant_consent, preview_hash, FileProfileStore, ProfileKind,
    ProfileStore,
};

#[test]
#[ignore = "reads the live profile; requires explicit user authorization"]
fn live_profile_preflight() {
    let paths = AppPaths::from_env();
    // Fail closed if an inherited test-data override redirects the live read.
    let authorized =
        std::path::Path::new(r"C:\Users\Pablo\AppData\Local\xemnas\settings\ai-profile.json");
    if paths.ai_profile != authorized {
        println!("providerkind=unknown model=unknown enabled=unknown consentstatus=path_mismatch");
        return;
    }

    let store = FileProfileStore::new(paths.ai_profile);
    let profile = match store.load() {
        Ok(Some(profile)) => profile,
        Ok(None) => {
            println!("providerkind=absent model=unknown enabled=false consentstatus=absent");
            return;
        }
        Err(_) => {
            // Storage errors may contain filesystem details: never display them.
            println!("providerkind=unknown model=unknown enabled=unknown consentstatus=unreadable");
            return;
        }
    };
    let kind = match profile.kind {
        ProfileKind::Fake => "fake",
        ProfileKind::OpenAiCompatible => "open_ai_compatible",
        ProfileKind::ChatGptPlan => "chat_gpt_plan",
        ProfileKind::OpenCode => "open_code",
    };
    let status = match consent_status(&profile) {
        Ok(()) => "valid",
        Err(reason) => reason,
    };
    let preview = build_preview(&profile);
    let matches = |preview: &application::profile::ConsentPreview| {
        profile
            .consent
            .as_ref()
            .is_some_and(|consent| consent.preview_hash == preview_hash(preview))
    };
    println!("stored_matches_current={}", matches(&preview));
    let mut without_routing = preview.clone();
    without_routing
        .categories
        .retain(|category| category.kind != "context_routing");
    without_routing.total_approximate_chars = profile
        .max_input_chars
        .saturating_mul(without_routing.categories.len());
    println!(
        "stored_matches_without_context_routing={}",
        matches(&without_routing)
    );
    let mut old_four = preview.clone();
    old_four.categories.truncate(4);
    old_four.total_approximate_chars = profile.max_input_chars.saturating_mul(4);
    println!("stored_matches_old_four={}", matches(&old_four));
    // Pure in-memory reproduction only: never persists or checks credentials.
    let grant_valid = grant_consent(&profile, &preview, "diagnostic-only", true)
        .is_ok_and(|granted| consent_status(&granted).is_ok());
    println!("in_memory_grant_valid={grant_valid}");
    if !matches(&preview) && (matches(&without_routing) || matches(&old_four)) {
        println!("mismatched_fields=categories,total_approximate_chars");
    }
    // Escaping the allowed model field prevents multiline/control-character output.
    println!(
        "providerkind={kind} model={:?} enabled={} consentstatus={status}",
        profile.model, profile.external_calls_enabled,
    );
}
