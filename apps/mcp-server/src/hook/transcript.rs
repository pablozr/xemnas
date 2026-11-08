//! Incremental reader of a Claude Code transcript (JSONL, one entry per line).
//!
//! Only `user` and `assistant` entries outside sidechains matter. Parsing is
//! lenient: unparsable lines and unknown fields are skipped. Transcripts reach
//! tens of MB, so the reader streams from a byte offset and keeps only the
//! compact [`Turn`] summary, never the raw entries.

use std::borrow::Cow;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use application::extract::WAKE_SOURCE;
use application::speaker::{classify, opens_with_harness_tag, Speaker};
use serde_json::Value;

/// Bytes read from the end of the transcript to find recently touched files.
const TAIL_BYTES: u64 = 256 * 1024;

/// Tools whose `file_path` counts as a recently touched file.
const FILE_TOOLS: [&str; 5] = ["Edit", "Write", "MultiEdit", "NotebookEdit", "Read"];

/// Echoes of slash commands, which are not user prompts.
const COMMAND_PREFIXES: [&str; 3] = ["<command-name>", "<local-command-", "<command-message>"];

/// Longest answer to an `AskUserQuestion` kept, in characters.
const MAX_ANSWER_CHARS: usize = 2_000;

/// Longest prompt (chars, trimmed) of a trivial turn: "ok", "pode seguir", a quick question.
pub const TRIVIAL_PROMPT_CHARS: usize = 80;

/// Longest assistant text (chars) of a trivial turn; longer answers carry content worth analysing.
pub const TRIVIAL_ASSISTANT_CHARS: usize = 600;

/// What a user prompt sends to the server: its text and, when the hook took
/// the words out of an envelope, where they came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    /// The text as it goes in the artifact.
    pub text: String,
    /// `Some("wake")` when `text` is the human part of a wake.
    pub source: Option<&'static str>,
}

/// One `tool_use` block of the turn and how it ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    /// Tool name.
    pub name: String,
    /// Block id, matched by `tool_result.tool_use_id`.
    pub id: String,
    /// Whether the matching `tool_result` had `is_error`.
    pub is_error: bool,
}

/// A file change reported by a tool result, as unified hunks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEdit {
    /// Path as the tool reported it.
    pub path: String,
    /// `@@ -a,b +c,d @@` lines followed by their content lines.
    pub hunks: String,
}

/// A user prompt and every following entry until the next prompt.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Turn {
    /// `uuid` of the prompt entry.
    pub prompt_uuid: String,
    /// Byte offset of the prompt line in the transcript.
    pub start_offset: u64,
    /// The prompts of the turn, one per user message: the server classifies each.
    pub prompts: Vec<Prompt>,
    /// Trimmed length (chars) of what the user said in the prompts, as the
    /// turn's trivial check reads it: zero for the harness and other agents.
    pub said_chars: usize,
    /// Assistant `text` blocks, in order; `thinking` is never kept.
    pub assistant_texts: Vec<String>,
    /// Tool calls, in order.
    pub tools: Vec<ToolCall>,
    /// File changes, in order.
    pub edits: Vec<FileEdit>,
    /// Paths the file tools were called on (edits and reads), in order, without repeats.
    pub touched: Vec<String>,
    /// What the user answered to the questions the assistant asked with
    /// `AskUserQuestion`, one entry per question: where they chose.
    pub answers: Vec<String>,
}

impl Turn {
    /// A short exchange without tool use: it adds no decision of its own, only
    /// the confirmation of the turn before it.
    pub fn is_trivial(&self) -> bool {
        let answer: usize = self.assistant_texts.iter().map(|t| t.chars().count()).sum();
        self.tools.is_empty()
            && self.said_chars <= TRIVIAL_PROMPT_CHARS
            && answer <= TRIVIAL_ASSISTANT_CHARS
    }

    /// Records a file the turn touched, keeping first-seen order without repeats.
    fn touch(&mut self, path: &str) {
        if !self.touched.iter().any(|known| known == path) {
            self.touched.push(path.to_string());
        }
    }

    /// Appends `other`'s prompts and answer after this turn's. Each prompt stays
    /// its own message, so a notification and the "pode" after it never mix. The
    /// identity (uuid, offset) stays this turn's.
    pub fn fold(&mut self, other: &Turn) {
        self.answers.extend(other.answers.iter().cloned());
        for path in &other.touched {
            self.touch(path);
        }
        self.prompts.extend(other.prompts.iter().cloned());
        if other.assistant_texts.is_empty() {
            return;
        }
        let answer = other.assistant_texts.join("\n");
        if self.assistant_texts.is_empty() {
            self.assistant_texts.push(answer);
        } else {
            let joined = self.assistant_texts.join("\n");
            self.assistant_texts = vec![format!("{joined}\n\n{answer}")];
        }
    }
}

/// Result of [`scan`].
#[derive(Debug, Default)]
pub struct Scan {
    /// Turns found, oldest first.
    pub turns: Vec<Turn>,
    /// Bytes read from the file.
    pub bytes_read: u64,
}

/// Reads turns from `offset` (a line start), at most `max_turns`. A file
/// shorter than `offset` was rewritten, so reading restarts at 0. Content
/// before the first prompt is ignored.
pub fn scan(path: &Path, offset: u64, max_turns: usize) -> io::Result<Scan> {
    let mut file = File::open(path)?;
    let offset = if offset > file.metadata()?.len() {
        0
    } else {
        offset
    };
    file.seek(SeekFrom::Start(offset))?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);

    let mut scan = Scan::default();
    let mut position = offset;
    let mut line = Vec::new();
    loop {
        line.clear();
        let read = reader.read_until(b'\n', &mut line)?;
        if read == 0 {
            break;
        }
        let start = position;
        position += read as u64;
        scan.bytes_read += read as u64;

        let Ok(entry) = serde_json::from_slice::<Value>(&line) else {
            continue;
        };
        if entry["isSidechain"].as_bool() == Some(true) {
            continue;
        }
        match entry["type"].as_str() {
            Some("user") => {
                if let Some((uuid, prompt, said_chars)) = prompt_of(&entry) {
                    if scan.turns.len() == max_turns {
                        break;
                    }
                    scan.turns.push(Turn {
                        prompt_uuid: uuid,
                        start_offset: start,
                        prompts: prompt.into_iter().collect(),
                        said_chars,
                        ..Turn::default()
                    });
                } else if let Some(turn) = scan.turns.last_mut() {
                    add_results(turn, &entry);
                }
            }
            Some("assistant") => {
                if let Some(turn) = scan.turns.last_mut() {
                    add_assistant(turn, &entry);
                }
            }
            _ => {}
        }
    }
    Ok(scan)
}

/// `(uuid, prompt, said_chars)` when `entry` starts a turn: not meta, not the
/// summary Claude Code writes when it compacts a conversation, not made of tool
/// results and not a slash-command echo. The turn always counts (for its diffs
/// and tools); `prompt` is what goes to the server and `said_chars` what the
/// user said, for the trivial check. A prompt is dropped only by fact: a
/// `promptSource` of `system`, nothing but reminders, an interruption, a fixed
/// agent opening or a wake without a human. When only the shape of a tag made
/// the classifier say harness, the raw text goes and the server decides, so a
/// pasted `<div>` is never lost. A wake sends the human's words with their
/// source. It is decided per message, before turns are folded, so a
/// notification and the "pode" that follows it never mix.
fn prompt_of(entry: &Value) -> Option<(String, Option<Prompt>, usize)> {
    let flagged = |field: &str| entry[field].as_bool() == Some(true);
    if flagged("isMeta") || flagged("isCompactSummary") || flagged("isVisibleInTranscriptOnly") {
        return None;
    }
    let uuid = entry["uuid"].as_str()?;
    let text = match &entry["message"]["content"] {
        Value::String(text) => text.clone(),
        Value::Array(blocks) => {
            if blocks.iter().any(|block| block["type"] == "tool_result") {
                return None;
            }
            blocks
                .iter()
                .filter(|block| block["type"] == "text")
                .filter_map(|block| block["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => return None,
    };
    let head = text.trim_start();
    if COMMAND_PREFIXES
        .iter()
        .any(|prefix| head.starts_with(prefix))
    {
        return None;
    }
    let uuid = uuid.to_string();
    if entry["promptSource"].as_str() == Some("system") {
        return Some((uuid, None, 0));
    }
    let (prompt, said_chars) = match classify(&text) {
        Speaker::User(Cow::Owned(words)) => {
            let said_chars = words.trim().chars().count();
            let source = Some(WAKE_SOURCE);
            (
                Some(Prompt {
                    text: words,
                    source,
                }),
                said_chars,
            )
        }
        Speaker::User(Cow::Borrowed(_)) => {
            let said_chars = text.trim().chars().count();
            (Some(Prompt { text, source: None }), said_chars)
        }
        Speaker::Harness | Speaker::Agent => {
            let kept = opens_with_harness_tag(&text).then_some(Prompt { text, source: None });
            (kept, 0)
        }
    };
    Some((uuid, prompt, said_chars))
}

fn add_assistant(turn: &mut Turn, entry: &Value) {
    let Some(blocks) = entry["message"]["content"].as_array() else {
        return;
    };
    for block in blocks {
        match block["type"].as_str() {
            Some("text") => {
                if let Some(text) = block["text"].as_str().filter(|text| !text.is_empty()) {
                    turn.assistant_texts.push(text.to_string());
                }
            }
            Some("tool_use") => {
                if let Some(name) = block["name"].as_str() {
                    if let Some(path) = touched_path(name, &block["input"]) {
                        turn.touch(path);
                    }
                    turn.tools.push(ToolCall {
                        name: name.to_string(),
                        id: block["id"].as_str().unwrap_or_default().to_string(),
                        is_error: false,
                    });
                }
            }
            _ => {}
        }
    }
}

/// The file a file tool was called on.
fn touched_path<'a>(tool: &str, input: &'a Value) -> Option<&'a str> {
    if !FILE_TOOLS.contains(&tool) {
        return None;
    }
    input["file_path"]
        .as_str()
        .or_else(|| input["notebook_path"].as_str())
}

fn add_results(turn: &mut Turn, entry: &Value) {
    for block in entry["message"]["content"].as_array().into_iter().flatten() {
        if block["type"] != "tool_result" || block["is_error"].as_bool() != Some(true) {
            continue;
        }
        let id = block["tool_use_id"].as_str().unwrap_or_default();
        if let Some(tool) = turn.tools.iter_mut().find(|tool| tool.id == id) {
            tool.is_error = true;
        }
    }
    if let Some(edit) = edit_of(&entry["toolUseResult"]) {
        turn.edits.push(edit);
    }
    turn.answers.extend(answers_of(&entry["toolUseResult"]));
}

/// The answers in the `toolUseResult` of an `AskUserQuestion`: `answers` maps
/// each question to the label (or free text) the user picked and `questions`
/// carries the options. Each becomes the question, the choice with its
/// description and the options left out, so the extractor sees what was
/// weighed.
fn answers_of(result: &Value) -> Vec<String> {
    let Some(answers) = result["answers"].as_object() else {
        return Vec::new();
    };
    let questions = result["questions"].as_array();
    let mut entries = Vec::new();
    for (question, answer) in answers {
        let Some(answer) = answer.as_str().filter(|answer| !answer.trim().is_empty()) else {
            continue;
        };
        let options: &[Value] = questions
            .into_iter()
            .flatten()
            .find(|candidate| candidate["question"].as_str() == Some(question))
            .and_then(|candidate| candidate["options"].as_array())
            .map_or(&[], Vec::as_slice);
        let chosen = options
            .iter()
            .find(|option| option["label"].as_str() == Some(answer));
        let mut entry = format!("question: {question}\nanswer: {answer}");
        if let Some(description) = chosen
            .and_then(|option| option["description"].as_str())
            .filter(|description| !description.is_empty())
        {
            entry.push_str(" \u{2014} ");
            entry.push_str(description);
        }
        let left: Vec<&str> = options
            .iter()
            .filter_map(|option| option["label"].as_str())
            .filter(|label| *label != answer)
            .collect();
        if !left.is_empty() {
            entry.push_str("\nnot chosen: ");
            entry.push_str(&left.join("; "));
        }
        entries.push(entry.chars().take(MAX_ANSWER_CHARS).collect());
    }
    entries
}

/// The change in a `toolUseResult`: its `structuredPatch`, or for a `Write`
/// that created the file (empty patch) one all-added hunk from `content`.
fn edit_of(result: &Value) -> Option<FileEdit> {
    let path = result["filePath"].as_str()?;
    let mut hunks = String::new();
    for hunk in result["structuredPatch"].as_array().into_iter().flatten() {
        let number = |key: &str| hunk[key].as_u64().unwrap_or_default();
        hunks.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            number("oldStart"),
            number("oldLines"),
            number("newStart"),
            number("newLines")
        ));
        for line in hunk["lines"].as_array().into_iter().flatten() {
            if let Some(line) = line.as_str() {
                hunks.push_str(line);
                hunks.push('\n');
            }
        }
    }
    if hunks.is_empty() && result["type"] == "create" {
        let content = result["content"]
            .as_str()
            .filter(|content| !content.is_empty())?;
        hunks.push_str(&format!("@@ -0,0 +1,{} @@\n", content.lines().count()));
        for line in content.lines() {
            hunks.push('+');
            hunks.push_str(line);
            hunks.push('\n');
        }
    }
    (!hunks.is_empty()).then(|| FileEdit {
        path: path.to_string(),
        hunks,
    })
}

/// Paths of the last `limit` distinct files the session touched, most recent
/// first, read from the tail of the transcript only.
pub fn recent_files(path: &Path, limit: usize) -> Vec<String> {
    let Ok(tail) = read_tail(path) else {
        return Vec::new();
    };
    let tail = String::from_utf8_lossy(&tail);
    let mut files: Vec<String> = Vec::new();
    for line in tail.lines().rev() {
        let Ok(entry) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if entry["type"] != "assistant" || entry["isSidechain"].as_bool() == Some(true) {
            continue;
        }
        let blocks = entry["message"]["content"].as_array();
        for block in blocks.into_iter().flatten().rev() {
            let touched = block["type"] == "tool_use"
                && block["name"]
                    .as_str()
                    .is_some_and(|name| FILE_TOOLS.contains(&name));
            let file = block["input"]["file_path"]
                .as_str()
                .or_else(|| block["input"]["notebook_path"].as_str());
            if let (true, Some(file)) = (touched, file) {
                if !files.iter().any(|known| known == file) {
                    files.push(file.to_string());
                }
            }
            if files.len() == limit {
                return files;
            }
        }
    }
    files
}

/// Last [`TAIL_BYTES`] of the file, starting at a line boundary.
fn read_tail(path: &Path) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let start = file.metadata()?.len().saturating_sub(TAIL_BYTES);
    file.seek(SeekFrom::Start(start))?;
    let mut tail = Vec::new();
    file.take(TAIL_BYTES).read_to_end(&mut tail)?;
    if start > 0 {
        // The first line is probably cut in half.
        let skip = tail
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(tail.len(), |at| at + 1);
        tail.drain(..skip);
    }
    Ok(tail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tool(name: &str, file: &str) -> Value {
        json!({ "type": "assistant", "message": { "content": [
            { "type": "tool_use", "id": "t", "name": name, "input": { "file_path": file } }
        ] } })
    }

    fn turn(prompt: &str, answer: &str) -> Turn {
        Turn {
            prompt_uuid: prompt.to_string(),
            prompts: vec![Prompt {
                text: prompt.to_string(),
                source: None,
            }],
            said_chars: prompt.trim().chars().count(),
            assistant_texts: vec![answer.to_string()],
            ..Turn::default()
        }
    }

    #[test]
    fn the_answer_to_an_ask_user_question_is_read_from_the_tool_result() {
        let path = std::env::temp_dir().join(format!("xemnas-ask-{}.jsonl", std::process::id()));
        let lines = [
            json!({ "type": "user", "uuid": "p1", "message": { "content": "planeja" } }),
            json!({ "type": "assistant", "message": { "content": [
                { "type": "tool_use", "id": "q1", "name": "AskUserQuestion", "input": {} }
            ] } }),
            json!({ "type": "user", "uuid": "r1", "message": { "content": [
                { "type": "tool_result", "tool_use_id": "q1", "content": "Your questions have been answered" }
            ] }, "toolUseResult": {
                "questions": [{ "question": "Qual cache?", "header": "Cache", "multiSelect": false,
                    "options": [
                        { "label": "Redis", "description": "rapido, exige servico" },
                        { "label": "Memoria", "description": "simples" },
                        { "label": "Disco", "description": "lento" }
                    ] }],
                "answers": { "Qual cache?": "Redis" }
            } }),
        ];
        let text: Vec<String> = lines.iter().map(Value::to_string).collect();
        std::fs::write(&path, text.join("\n") + "\n").expect("write");

        let scan = scan(&path, 0, 8).expect("scan");
        let _ = std::fs::remove_file(&path);
        assert_eq!(scan.turns.len(), 1);
        assert_eq!(
            scan.turns[0].answers,
            ["question: Qual cache?\nanswer: Redis \u{2014} rapido, exige servico\nnot chosen: Memoria; Disco"]
        );
        let mut folded = turn("a", "b");
        folded.fold(&scan.turns[0]);
        assert_eq!(folded.answers.len(), 1);
    }

    #[test]
    fn a_free_text_answer_has_no_option_to_describe() {
        let result = json!({
            "questions": [{ "question": "Qual?", "options": [{ "label": "A", "description": "d" }] }],
            "answers": { "Qual?": "outra coisa", "Vazia?": " " }
        });
        assert_eq!(
            answers_of(&result),
            ["question: Qual?\nanswer: outra coisa\nnot chosen: A"]
        );
        assert!(answers_of(&json!({ "filePath": "a.rs" })).is_empty());
    }

    #[test]
    fn trivial_turns_are_short_and_without_tools() {
        assert!(turn("  ok  ", "certo").is_trivial());
        assert!(turn(&"a".repeat(TRIVIAL_PROMPT_CHARS), "").is_trivial());
        assert!(!turn(&"a".repeat(TRIVIAL_PROMPT_CHARS + 1), "certo").is_trivial());
        assert!(!turn("ok", &"a".repeat(TRIVIAL_ASSISTANT_CHARS + 1)).is_trivial());
        let mut with_tool = turn("ok", "certo");
        with_tool.tools.push(ToolCall {
            name: "Bash".to_string(),
            id: "t".to_string(),
            is_error: false,
        });
        assert!(!with_tool.is_trivial());
    }

    #[test]
    fn folding_keeps_each_prompt_and_the_identity() {
        let mut head = turn("proposta", "faço X?");
        head.start_offset = 7;
        head.fold(&turn("pode", "feito"));

        assert_eq!(head.prompt_uuid, "proposta");
        assert_eq!(head.start_offset, 7);
        let texts: Vec<&str> = head.prompts.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, ["proposta", "pode"]);
        assert_eq!(head.assistant_texts.join("\n"), "faço X?\n\nfeito");
    }

    #[test]
    fn touched_files_come_from_the_file_tools_in_order_without_repeats() {
        let mut turn = Turn::default();
        for (name, file) in [
            ("Read", "/a/x.rs"),
            ("Bash", "/a/no.rs"),
            ("Edit", "/b/y.rs"),
            ("Edit", "/a/x.rs"),
        ] {
            add_assistant(&mut turn, &tool(name, file));
        }
        let notebook = json!({ "type": "assistant", "message": { "content": [
            { "type": "tool_use", "id": "n", "name": "NotebookEdit", "input": { "notebook_path": "/c/n.ipynb" } }
        ] } });
        add_assistant(&mut turn, &notebook);
        assert_eq!(turn.touched, ["/a/x.rs", "/b/y.rs", "/c/n.ipynb"]);

        let mut head = Turn {
            touched: vec!["/b/y.rs".to_string()],
            ..Turn::default()
        };
        head.fold(&turn);
        assert_eq!(head.touched, ["/b/y.rs", "/a/x.rs", "/c/n.ipynb"]);
    }

    #[test]
    fn recent_files_come_from_the_tail_deduplicated_and_capped() {
        let path = std::env::temp_dir().join(format!("xemnas-tail-{}.jsonl", std::process::id()));
        let mut lines = vec![tool("Edit", "old-head.rs").to_string()];
        let filler = json!({ "type": "attachment", "text": "x".repeat(1_000) }).to_string();
        lines.extend(std::iter::repeat_n(filler, 600));
        lines.extend([
            tool("Read", "a.rs").to_string(),
            tool("Bash", "ignored.rs").to_string(),
            "not json".to_string(),
            tool("Write", "b.rs").to_string(),
            tool("Edit", "a.rs").to_string(),
            tool("NotebookEdit", "n.ipynb").to_string(),
        ]);
        std::fs::write(&path, lines.join("\n") + "\n").expect("write");

        assert_eq!(recent_files(&path, 8), ["n.ipynb", "a.rs", "b.rs"]);
        assert_eq!(recent_files(&path, 2), ["n.ipynb", "a.rs"]);
        assert!(recent_files(Path::new("nao-existe.jsonl"), 8).is_empty());
        let _ = std::fs::remove_file(&path);
    }
}
