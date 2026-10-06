//! Claude Code hooks: `xemnas-mcp hook prompt` injects context on every user
//! prompt, `xemnas-mcp hook stop` captures the finished exchanges and
//! `xemnas-mcp hook session-end` flushes the one held back.
//!
//! A hook must never break or block Claude Code: every path exits 0, prints
//! nothing on failure and writes at most one sanitized line (ids and counts,
//! never prompt or content text) to stderr.

pub mod deliver;
pub mod envelope;
pub mod prompt;
pub mod stop;
pub mod transcript;

use std::io::Read;

use serde_json::Value;

/// Largest hook payload read from stdin.
const MAX_INPUT_BYTES: u64 = 8 * 1024 * 1024;

/// Runs the hook `subcommand` (`prompt`, `stop` or `session-end`) over the JSON on stdin.
pub fn run(subcommand: &str) {
    // A panic message could carry content; hooks stay silent instead.
    std::panic::set_hook(Box::new(|_| {}));
    let outcome = std::panic::catch_unwind(|| handle(subcommand));
    if outcome.is_err() {
        eprintln!("xemnas hook: internal error");
    }
}

fn handle(subcommand: &str) {
    let mut input = String::new();
    let read = std::io::stdin()
        .take(MAX_INPUT_BYTES)
        .read_to_string(&mut input);
    let Some(event) = read
        .ok()
        .and_then(|_| serde_json::from_str::<Value>(&input).ok())
    else {
        eprintln!("xemnas hook: invalid input");
        return;
    };
    let paths = application::AppPaths::from_env();
    match subcommand {
        "prompt" => match prompt::execute(&event, &paths.runtime_dir) {
            Ok(Some(line)) => println!("{line}"),
            Ok(None) => {}
            Err(reason) => eprintln!("xemnas hook: prompt {reason}"),
        },
        "stop" | "session-end" => {
            if let Err(reason) = stop::execute(&event, &paths, subcommand == "session-end") {
                eprintln!("xemnas hook: {subcommand} {reason}");
            }
        }
        _ => eprintln!("xemnas hook: unknown subcommand"),
    }
}
