//! `hook prompt`: asks the running app for a context block for the prompt.

use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::envelope::relative_inside;
use super::root::Roots;
use super::transcript::recent_files;
use crate::backend::endpoint;

/// Total time for the whole hook, the root included; the turn proceeds without context after it.
const CONTEXT_TIMEOUT: Duration = Duration::from_millis(300);

/// Files sent with the prompt (the server accepts 50).
const MAX_FILES: usize = 8;

/// Folders resolved for the root, inside the same time budget.
const MAX_ROOT_LOOKUPS: usize = 16;

/// Hook output line for the `UserPromptSubmit` event, or `None` when there is
/// nothing to inject (app closed, no context). `Err` carries a sanitized reason.
pub fn execute(event: &Value, runtime_dir: &Path) -> Result<Option<String>, &'static str> {
    let (Some(cwd), Some(session_id)) = (event["cwd"].as_str(), event["session_id"].as_str())
    else {
        return Err("invalid input");
    };
    let deadline = Instant::now() + CONTEXT_TIMEOUT;
    let Some((port, token)) = endpoint(runtime_dir) else {
        return Ok(None);
    };

    let recent = event["transcript_path"]
        .as_str()
        .map(|path| recent_files(Path::new(path), MAX_FILES))
        .unwrap_or_default();
    let root = Roots::new(cwd, MAX_ROOT_LOOKUPS)
        .until(deadline)
        .root_for(&[], &recent);
    let files: Vec<String> = recent
        .iter()
        .map(|file| relative_inside(file, &root).unwrap_or_else(|| file.replace('\\', "/")))
        .collect();
    let body = json!({
        "canonical_path": root,
        "session_id": session_id,
        "prompt": event["prompt"].as_str().unwrap_or_default(),
        "files": files,
    });

    // One deadline for the whole hook: the HTTP call gets what the root left.
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err("timeout");
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(remaining)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "client unavailable")?;
    let response = match client
        .post(format!("http://127.0.0.1:{port}/v1/context"))
        .bearer_auth(token)
        .json(&body)
        .send()
    {
        Ok(response) => response,
        // A stale discovery file means the app is closed.
        Err(error) if error.is_connect() => return Ok(None),
        Err(error) if error.is_timeout() => return Err("timeout"),
        Err(_) => return Err("request failed"),
    };
    if !response.status().is_success() {
        return Err("rejected by the app");
    }

    let context = response
        .json::<Value>()
        .map_err(|_| "unreadable response")?["context"]
        .as_str()
        .filter(|context| !context.trim().is_empty())
        .map(str::to_string);
    Ok(context.map(|context| {
        json!({
            "hookSpecificOutput": {
                "hookEventName": "UserPromptSubmit",
                "additionalContext": context,
            }
        })
        .to_string()
    }))
}
