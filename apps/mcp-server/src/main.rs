//! `xemnas-mcp`: started by the agent; stdout carries only the protocol.

use std::io::{BufRead, Write};

use xemnas_mcp::backend::HttpBackend;
use xemnas_mcp::protocol::{handle, parse_error};

fn main() {
    let directory = project_directory();
    let backend = HttpBackend::new(application::AppPaths::from_env().runtime_dir, directory);
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str(&line) {
            Ok(message) => handle(&message, &backend),
            Err(_) => Some(parse_error()),
        };
        if let Some(response) = response {
            if writeln!(stdout, "{response}")
                .and_then(|()| stdout.flush())
                .is_err()
            {
                break;
            }
        }
    }
}

/// `--project <dir>`, or the directory the agent started the server in.
fn project_directory() -> String {
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--project" {
            if let Some(directory) = arguments.next() {
                return directory;
            }
        }
    }
    std::env::current_dir()
        .map(|directory| directory.to_string_lossy().into_owned())
        .unwrap_or_default()
}
