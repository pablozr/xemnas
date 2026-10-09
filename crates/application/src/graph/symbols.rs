//! Where the code defines its names: a line scanner in the style of ctags,
//! without a parser. A decision that cites `FLUSH_INTERVAL` between backticks
//! is about the component whose code defines it. The scan reads only files
//! with a code extension of at most [`MAX_FILE_BYTES`], stops at
//! [`MAX_TOTAL_BYTES`], and runs once per listing (see
//! [`super::repo_files::RepoFiles::definitions`]).
//!
//! It is a heuristic, not a parser: a name defined in two components, or in
//! none, links to nothing.

use std::collections::BTreeMap;
use std::path::Path;

use super::repo_files::RepoFiles;

/// Largest file read, in bytes.
const MAX_FILE_BYTES: u64 = 256 * 1024;
/// Bytes read in all; beyond it the scan is `partial`.
const MAX_TOTAL_BYTES: u64 = 32 * 1024 * 1024;
/// Shortest symbol worth indexing.
pub(crate) const MIN_SYMBOL_CHARS: usize = 4;

/// Extensions of the files scanned.
const CODE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "go", "java", "kt", "cs", "swift", "rb",
    "php", "c", "h", "cc", "cpp", "hpp",
];

/// Words that come before a definition and say nothing about it.
const MODIFIERS: &[&str] = &[
    "pub",
    "export",
    "default",
    "async",
    "public",
    "private",
    "protected",
    "final",
    "abstract",
    "unsafe",
    "extern",
];

/// Words that introduce a definition: the identifier after one is defined.
const KEYWORDS: &[&str] = &[
    "fn",
    "struct",
    "enum",
    "trait",
    "type",
    "const",
    "static",
    "mod",
    "class",
    "interface",
    "function",
    "def",
    "func",
    "record",
    "object",
];

/// Words between the keyword and the name that are not the name.
const FILLERS: &[&str] = &["mut", "fn", "unsafe", "async"];

/// Whether `name` is a file the scan reads.
pub(crate) fn is_code_file(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(stem, extension)| !stem.is_empty() && CODE_EXTENSIONS.contains(&extension))
}

/// Whether `word` is specific enough to be looked up: long enough, and
/// written with an uppercase letter or an underscore, so common words such as
/// `state` or `config` are never taken for a symbol.
pub(crate) fn is_symbol(word: &str) -> bool {
    word.chars().count() >= MIN_SYMBOL_CHARS
        && word
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
        && word
            .chars()
            .any(|character| character.is_uppercase() || character == '_')
}

/// The names the code defines, with the files that define them.
#[derive(Debug, Default)]
pub(crate) struct Definitions {
    by_symbol: BTreeMap<String, Vec<usize>>,
    /// The scan stopped at its byte budget: a name may be defined elsewhere.
    partial: bool,
}

impl Definitions {
    /// Positions (in the listing) of the files that define `symbol`.
    pub(crate) fn files_defining(&self, symbol: &str) -> &[usize] {
        self.by_symbol.get(symbol).map_or(&[], Vec::as_slice)
    }

    /// Whether the scan was cut.
    pub(crate) fn partial(&self) -> bool {
        self.partial
    }

    /// Number of distinct names found.
    pub(crate) fn count(&self) -> usize {
        self.by_symbol.len()
    }
}

/// Scans the code files of `repo` under `root`.
pub(crate) fn scan(root: &Path, repo: &RepoFiles) -> Definitions {
    let mut definitions = Definitions::default();
    let mut read = 0u64;
    for (position, file) in repo.paths().iter().enumerate() {
        let name = file.rsplit('/').next().unwrap_or("");
        if !is_code_file(name) {
            continue;
        }
        let path = root.join(file);
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        if metadata.len() > MAX_FILE_BYTES {
            continue;
        }
        if read + metadata.len() > MAX_TOTAL_BYTES {
            definitions.partial = true;
            break;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        read += bytes.len() as u64;
        let text = String::from_utf8_lossy(&bytes);
        for line in text.lines() {
            if let Some(symbol) = defined_name(line) {
                if is_symbol(symbol) {
                    let files = definitions.by_symbol.entry(symbol.to_string()).or_default();
                    if files.last() != Some(&position) {
                        files.push(position);
                    }
                }
            }
        }
    }
    definitions
}

/// The name a line defines: after modifiers, the first word is a keyword and
/// the identifier after it is the name.
fn defined_name(line: &str) -> Option<&str> {
    let mut words = line.split_whitespace().peekable();
    // Modifiers, which may carry a visibility group (`pub(crate)`).
    while let Some(word) = words.peek() {
        let head = word.split('(').next().unwrap_or(word);
        if MODIFIERS.contains(&head) {
            words.next();
        } else {
            break;
        }
    }
    let keyword = words.next()?;
    if !KEYWORDS.contains(&keyword) {
        return None;
    }
    let mut name = words.next()?;
    while FILLERS.contains(&name) {
        name = words.next()?;
    }
    let end = name
        .find(|character: char| !(character.is_alphanumeric() || character == '_'))
        .unwrap_or(name.len());
    let name = &name[..end];
    (!name.is_empty()).then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_are_read_from_the_first_words_of_a_line() {
        for (line, name) in [
            ("pub const FLUSH_INTERVAL: u64 = 5;", Some("FLUSH_INTERVAL")),
            ("pub(crate) struct SessionTable {", Some("SessionTable")),
            ("    pub async fn run_all() {", Some("run_all")),
            ("pub const fn max_len() -> usize {", Some("max_len")),
            ("static mut COUNTER: u8 = 0;", Some("COUNTER")),
            ("export default class Widget extends Base {", Some("Widget")),
            ("export interface UserRecord {", Some("UserRecord")),
            ("def build_index(root):", Some("build_index")),
            ("func ParseFlags(args []string) {", Some("ParseFlags")),
            ("public abstract class Handler {", Some("Handler")),
            ("type Alias<T> = Vec<T>;", Some("Alias")),
            ("let value = 1;", None),
            ("// struct Commented", None),
            ("use std::fmt;", None),
            ("", None),
        ] {
            assert_eq!(defined_name(line), name, "{line}");
        }
    }

    #[test]
    fn only_specific_words_are_symbols() {
        assert!(is_symbol("FLUSH_INTERVAL"));
        assert!(is_symbol("SessionTable"));
        assert!(is_symbol("run_all"));
        assert!(!is_symbol("state"));
        assert!(!is_symbol("Api"));
        assert!(!is_symbol("a-b_c"));
        assert!(is_code_file("state.rs") && is_code_file("App.tsx"));
        assert!(!is_code_file("notes.md") && !is_code_file(".rs"));
    }

    #[test]
    fn the_scan_maps_names_to_the_files_that_define_them() {
        let root = std::env::temp_dir().join(format!("xemnas-symbols-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (file, text) in [
            (
                "a/src/lib.rs",
                "pub struct Config {}\npub const KEEP_ALIVE: u8 = 1;\n",
            ),
            ("b/src/lib.rs", "pub struct Config {}\n"),
            ("b/notes.md", "pub struct NotCode {}\n"),
        ] {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let repo = RepoFiles::from_paths(
            vec![
                "a/src/lib.rs".into(),
                "b/src/lib.rs".into(),
                "b/notes.md".into(),
            ],
            false,
        );
        let definitions = scan(&root, &repo);
        let defining = |symbol: &str| -> Vec<&str> {
            definitions
                .files_defining(symbol)
                .iter()
                .map(|position| repo.file(*position))
                .collect()
        };
        assert_eq!(defining("Config"), ["a/src/lib.rs", "b/src/lib.rs"]);
        assert_eq!(defining("KEEP_ALIVE"), ["a/src/lib.rs"]);
        assert!(definitions.files_defining("NotCode").is_empty());
        assert_eq!(definitions.count(), 2);
        assert!(!definitions.partial());
        let _ = std::fs::remove_dir_all(&root);
    }
}
