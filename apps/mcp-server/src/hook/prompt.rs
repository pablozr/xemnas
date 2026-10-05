//! `hook prompt`: asks the running app for a context block for the prompt.

use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};

use super::envelope::relative_inside;
use super::transcript::recent_files;
use crate::backend::endpoint;

/// Total time for the context call; the turn proceeds without context after it.
const CONTEXT_TIMEOUT: Duration = Duration::from_millis(300);

/// Files sent with the prompt (the server accepts 50).
const MAX_FILES: usize = 8;

/// Hook output line for the `UserPromptSubmit` event, or `None` when there is
/// nothing to inject (app closed, no context). `Err` carries a sanitized reason.
pub fn execute(event: &Value, runtime_dir: &Path) -> Result<Option<String>, &'static str> {
    let (Some(cwd), Some(session_id)) = (event["cwd"].as_str(), event["session_id"].as_str())
    else {
        return Err("invalid input");
    };
    let Some((port, token)) = endpoint(runtime_dir) else {
        return Ok(None);
    };

    let files: Vec<String> = event["transcript_path"]
        .as_str()
        .map(|path| recent_files(Path::new(path), MAX_FILES))
        .unwrap_or_default()
        .iter()
        .map(|file| relative_inside(file, cwd).unwrap_or_else(|| file.replace('\\', "/")))
        .collect();
    let body = json!({
        "canonical_path": cwd,
        "session_id": session_id,
        "prompt": event["prompt"].as_str().unwrap_or_default(),
        "files": files,
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(CONTEXT_TIMEOUT)
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
