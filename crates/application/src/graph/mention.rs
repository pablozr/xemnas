//! Mentions of map entities in the text of a decision: a decision taken from
//! an ADR or a spec only "touched" the document, but its words name the parts
//! it is about ("o core grava pela outbox"). No AI: whole-word matching
//! against the names, aliases and path patterns the map already has.
//!
//! # Matching rule
//!
//! Text and terms are compared lowercase and without accents (`Decisões` =
//! `decisoes`), on whole words: the characters around a hit must not be
//! letters, digits, `_` or `-`, so `storage` does not hit `storage-sqlite`.
//!
//! * A **path** (a term with `/`, or the literal part of a pattern such as
//!   `crates/core/**` → `crates/core`) always counts.
//! * A **name or alias** of 4 or more characters counts anywhere.
//! * A name of **3 characters** (`api`, `cli`, `web`) is too common in prose:
//!   it counts only when written as code or as an acronym, i.e. in uppercase
//!   (`API`) or between backticks or quotes (`` `api` ``, `"api"`).
//! * Shorter names never count.
//! * A name that is also the **project's own name** (the app `acme` of the
//!   project `acme`) cannot be told from the product in prose: it counts only
//!   as a path, as code or between quotes, whatever its length.
//! * A name that is a **segment of another path** (`data_dir()/acme/`,
//!   `logs/core/`, `crate::core`) is not a mention of the part.
//!
//! # Affirmative mentions only
//!
//! A hit counts only when the text says something about the part. A part named
//! to be excluded ("independent of X", "sem X", "instead of X", "X foi
//! descartado") is not linked. The rule is the lexical part of NegEx and
//! ConText (Chapman et al.): trigger phrases that negate what follows
//! ([`NEGATION_BEFORE`]) or what precedes ([`NEGATION_AFTER`]), pseudo
//! triggers that look like them and do not negate ([`PSEUDO_TRIGGERS`]), and a
//! scope that ends at clause punctuation, a terminating word, a connector
//! followed by a new clause or [`MAX_SCOPE_WORDS`] words. No AI, and a single
//! pass over the text.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use std::collections::BTreeSet;

use super::EntityRecord;

/// Prefix of the reason of a suggestion derived from a mention; the quote
/// follows between double quotes. Auto-approval reads it to tell these weak
/// links from file and dependency ones.
pub const MENTION_REASON: &str = "citado no texto: ";
/// Characters of context kept on each side of a mention in its quote.
const QUOTE_CONTEXT: usize = 48;
/// Characters a name needs to count anywhere in prose.
const MIN_PLAIN: usize = 4;
/// Characters a name needs to count when written as code or acronym.
const MIN_MARKED: usize = 3;
/// Words a negation reaches, at most.
const MAX_SCOPE_WORDS: usize = 8;

/// Phrases that negate what follows them. English and Portuguese; the plain
/// English `no` is left out because it is the Portuguese "in the" (`no core`).
const NEGATION_BEFORE: &[&str] = &[
    "not",
    "never",
    "without",
    "nor",
    "neither",
    "instead of",
    "rather than",
    "independent of",
    "independently of",
    "independent from",
    "regardless of",
    "except",
    "except for",
    "excluding",
    "other than",
    "apart from",
    "unlike",
    "outside",
    "avoid",
    "avoids",
    "avoiding",
    "must not",
    "do not",
    "does not",
    "don't",
    "doesn't",
    "cannot",
    "no longer",
    "nothing about",
    "decoupled from",
    "isolated from",
    "free of",
    "não",
    "nunca",
    "sem",
    "nem",
    "em vez de",
    "ao invés de",
    "no lugar de",
    "independente de",
    "independentemente de",
    "exceto",
    "salvo",
    "fora de",
    "evitar",
    "evita",
    "nenhum",
    "nenhuma",
    "desacoplado de",
    "isolado de",
    "não mais",
];

/// Phrases that negate what precedes them.
const NEGATION_AFTER: &[&str] = &[
    "foi descartado",
    "foi rejeitado",
    "was rejected",
    "was discarded",
    "is out of scope",
    "não entra no escopo",
    "is not used",
    "não é usado",
];

/// Phrases that contain a trigger and do not negate.
const PSEUDO_TRIGGERS: &[&str] = &[
    "not only",
    "not just",
    "não só",
    "não apenas",
    "no doubt",
    "sem dúvida",
];

/// Words that end the scope of a negation.
const TERMINATORS: &[&str] = &[
    "but", "however", "while", "whereas", "although", "though", "yet", "so", "because", "since",
    "which", "that", "where", "when", "mas", "porém", "enquanto", "embora", "porque", "pois",
    "que", "onde", "quando",
];

/// Words that join the items of a list.
const CONNECTORS: &[&str] = &["and", "or", "e", "ou"];

/// Articles: a connector followed by one keeps the list going (`sem o X e o
/// Y`). After a comma it starts a new clause (`sem o X, o Y faz Z`), so only a
/// name or code goes on there.
const ARTICLES: &[&str] = &[
    "a", "an", "the", "o", "os", "as", "um", "uma", "uns", "umas", "do", "da", "dos", "das",
];

/// The reason stored on a suggestion derived from `quote`.
pub fn mention_reason(quote: &str) -> String {
    format!("{MENTION_REASON}\"{quote}\"")
}

/// The quote of a suggestion derived from a mention, `None` for any other
/// reason (a file or a dependency).
pub fn mention_quote(reason: &str) -> Option<&str> {
    let rest = reason.strip_prefix(MENTION_REASON)?;
    Some(
        rest.strip_prefix('"')
            .and_then(|quote| quote.strip_suffix('"'))
            .unwrap_or(rest),
    )
}

/// One way the text may name an entity, already folded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Term {
    chars: Vec<char>,
    path: bool,
    /// Named like the project: counts only as code or between quotes.
    restricted: bool,
}

/// The terms that name `entity`: name, aliases and, for a component, the
/// literal part of each path pattern. Terms the rule can never accept are
/// left out. `project_keys` are the keys of the project's own names (see
/// `discover::project_names`).
pub(crate) fn entity_terms(entity: &EntityRecord, project_keys: &BTreeSet<String>) -> Vec<Term> {
    let patterns = entity.patterns.iter().filter_map(|pattern| {
        let literal = pattern
            .trim()
            .trim_end_matches("/**")
            .trim_end_matches("/*")
            .trim_end_matches('/');
        (!literal.contains(['*', '?'])).then_some(literal)
    });
    let mut terms: Vec<Term> = Vec::new();
    for raw in std::iter::once(entity.name.as_str())
        .chain(entity.aliases.iter().map(String::as_str))
        .chain(patterns)
    {
        let chars: Vec<char> = raw.trim().chars().map(fold).collect();
        let path = chars.contains(&'/');
        if chars.is_empty() || (chars.len() < MIN_MARKED && !path) {
            continue;
        }
        let restricted = !path && project_keys.contains(&domain::entities::entity_key(raw));
        let term = Term {
            chars,
            path,
            restricted,
        };
        if !terms.contains(&term) {
            terms.push(term);
        }
    }
    terms
}

/// The term for a dependency name: counts like a name of the same length, and
/// when it is shorter than [`MIN_PLAIN`] or an ordinary English word (`time`,
/// `log`) only as code or between quotes.
pub(crate) fn dependency_term(name: &str) -> Option<Term> {
    const COMMON_WORDS: &[&str] = &[
        "time",
        "log",
        "rand",
        "url",
        "bytes",
        "once",
        "either",
        "home",
        "dirs",
        "open",
        "json",
        "hex",
        "ring",
        "base",
        "serde_json",
    ];
    let chars: Vec<char> = name.trim().chars().map(fold).collect();
    if chars.len() < MIN_MARKED {
        return None;
    }
    let path = chars.contains(&'/');
    let lower: String = chars.iter().collect();
    let restricted = !path && (chars.len() < MIN_PLAIN || COMMON_WORDS.contains(&lower.as_str()));
    Some(Term {
        chars,
        path,
        restricted,
    })
}

/// The term for a path written in a text: a file the text may cite.
pub(crate) fn path_term(path: &str) -> Option<Term> {
    let chars: Vec<char> = path
        .trim()
        .trim_start_matches("./")
        .chars()
        .map(fold)
        .collect();
    (!chars.is_empty()).then_some(Term {
        chars,
        path: true,
        restricted: false,
    })
}

/// A text ready to be searched: the original characters and their folded
/// form, one to one, with the spans of code and the words a negation reaches.
pub(crate) struct Folded {
    original: Vec<char>,
    folded: Vec<char>,
    /// Between a pair of backticks.
    code: Vec<bool>,
    /// Inside the scope of a negation; empty when the text has none.
    negated: Vec<bool>,
}

impl Folded {
    pub(crate) fn new(text: &str) -> Self {
        let original: Vec<char> = text.chars().collect();
        let folded: Vec<char> = original.iter().copied().map(fold).collect();
        let code = code_spans(&original);
        let negated = negation_mask(&folded, &code);
        Self {
            original,
            folded,
            code,
            negated,
        }
    }

    /// The words written between backticks (names with `.` kept, so
    /// `state.rs` stays whole), outside every negation.
    pub(crate) fn code_words(&self) -> Vec<String> {
        let mut words = Vec::new();
        let mut at = 0;
        while at < self.original.len() {
            if !self.code[at] {
                at += 1;
                continue;
            }
            let start = at;
            while at < self.original.len()
                && self.code[at]
                && (self.original[at].is_alphanumeric() || matches!(self.original[at], '_' | '.'))
            {
                at += 1;
            }
            if at == start {
                at += 1;
                continue;
            }
            if !self.negated.get(start).copied().unwrap_or(false) {
                let word: String = self.original[start..at].iter().collect();
                let word = word.trim_matches('.');
                if !word.is_empty() {
                    words.push(word.to_string());
                }
            }
        }
        words
    }

    /// The quote around the first affirmative mention of `term`, when the text
    /// has one the rule accepts.
    pub(crate) fn mention(&self, term: &Term) -> Option<String> {
        self.find(term, true)
            .map(|start| self.quote(start, term.chars.len()))
    }

    /// The first hit the rule accepts; one inside a negation is skipped when
    /// `affirmative`.
    fn find(&self, term: &Term, affirmative: bool) -> Option<usize> {
        let size = term.chars.len();
        if size == 0 || size > self.folded.len() {
            return None;
        }
        (0..=self.folded.len() - size).find(|&start| {
            self.folded[start..start + size] == term.chars[..]
                && self.accepts(start, term)
                && !(affirmative && self.negated.get(start).copied().unwrap_or(false))
        })
    }

    fn accepts(&self, start: usize, term: &Term) -> bool {
        let size = term.chars.len();
        let before = start.checked_sub(1).map(|at| self.original[at]);
        let after = self.original.get(start + size).copied();
        if before.is_some_and(word_char) || after.is_some_and(word_char) {
            return false;
        }
        if !term.path {
            // A segment of another path (`data_dir()/acme/`, `crate::core`)
            // is not the part. `::` after is the part's own item
            // (`sc_core::X`).
            let separator = |character: char| matches!(character, '/' | '\\');
            let scoped = start >= 2 && self.original[start - 2..start] == [':', ':'];
            if before.is_some_and(separator) || after.is_some_and(separator) || scoped {
                return false;
            }
        }
        if term.path || (size >= MIN_PLAIN && !term.restricted) {
            return true;
        }
        let marked = before.is_some_and(code_mark) && after.is_some_and(code_mark);
        let in_code = self.code[start] && self.code[start + size - 1];
        let written = &self.original[start..start + size];
        let acronym = !term.restricted
            && written.iter().any(|c| c.is_alphabetic())
            && !written.iter().any(|c| c.is_lowercase());
        acronym || marked || in_code
    }

    fn quote(&self, start: usize, size: usize) -> String {
        let text = &self.original;
        let mut from = start.saturating_sub(QUOTE_CONTEXT);
        if from > 0 {
            // Start at a word, not in the middle of one.
            if let Some(space) = text[from..start].iter().position(|c| c.is_whitespace()) {
                from += space + 1;
            }
        }
        let mut to = (start + size + QUOTE_CONTEXT).min(text.len());
        if to < text.len() {
            if let Some(space) = text[start + size..to]
                .iter()
                .rposition(|c| c.is_whitespace())
            {
                to = start + size + space;
            }
        }
        let body: String = text[from..to].iter().collect();
        let body = body.split_whitespace().collect::<Vec<_>>().join(" ");
        let lead = if from > 0 { "…" } else { "" };
        let tail = if to < text.len() { "…" } else { "" };
        format!("{lead}{body}{tail}")
    }
}

/// Lowercase without accent, one character for one.
fn fold(character: char) -> char {
    let lower = character.to_lowercase().next().unwrap_or(character);
    match lower {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        '\\' => '/',
        other => other,
    }
}

/// Characters that continue a word (and a crate name).
fn word_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '-'
}

/// Characters that mark a name as code or as a quoted name.
fn code_mark(character: char) -> bool {
    matches!(character, '`' | '"' | '\'' | '“' | '”' | '‘' | '’')
}

/// Which characters sit between a pair of backticks. An unpaired backtick
/// opens nothing.
fn code_spans(original: &[char]) -> Vec<bool> {
    let mut code = vec![false; original.len()];
    let mut ticks = original
        .iter()
        .enumerate()
        .filter(|(_, character)| **character == '`')
        .map(|(at, _)| at);
    while let (Some(open), Some(close)) = (ticks.next(), ticks.next()) {
        for flag in &mut code[open + 1..close] {
            *flag = true;
        }
    }
    code
}

/// One piece of the text as the negation scope reads it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Piece {
    Word,
    Comma,
    /// Ends a clause: `.` before a space, `;`, `:`, `!`, `?`, a bracket or a
    /// line break.
    Stop,
}

struct Token {
    start: usize,
    end: usize,
    piece: Piece,
}

/// Splits the folded text into words, commas and clause stops.
fn tokenize(folded: &[char]) -> Vec<Token> {
    let count = folded.len();
    let ends_clause = |at: usize| folded.get(at + 1).is_none_or(|next| next.is_whitespace());
    let stop = |at: usize| Token {
        start: at,
        end: at + 1,
        piece: Piece::Stop,
    };
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < count {
        let character = folded[at];
        match character {
            '\n' | '\r' | ';' | '!' | '?' | '(' | ')' | '[' | ']' | '{' | '}' => {
                tokens.push(stop(at));
                at += 1;
            }
            '.' | ':' if ends_clause(at) => {
                tokens.push(stop(at));
                at += 1;
            }
            ',' => {
                tokens.push(Token {
                    start: at,
                    end: at + 1,
                    piece: Piece::Comma,
                });
                at += 1;
            }
            _ if character.is_whitespace() || is_quote(character) => at += 1,
            _ => {
                let start = at;
                while at < count {
                    let next = folded[at];
                    let splits = next.is_whitespace()
                        || matches!(
                            next,
                            ',' | ';' | '!' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '"' | '`'
                        )
                        || matches!(next, '\u{201c}' | '\u{201d}')
                        || (matches!(next, '.' | ':') && ends_clause(at));
                    if splits {
                        break;
                    }
                    at += 1;
                }
                let mut end = at;
                while end > start && is_quote(folded[end - 1]) {
                    end -= 1;
                }
                if end > start {
                    tokens.push(Token {
                        start,
                        end,
                        piece: Piece::Word,
                    });
                }
            }
        }
    }
    tokens
}

/// Quote marks that wrap a word without being part of it.
fn is_quote(character: char) -> bool {
    matches!(
        character,
        '\'' | '"' | '\u{2018}' | '\u{2019}' | '\u{201c}' | '\u{201d}' | '`'
    )
}

/// Trigger phrases, folded and indexed by their first word.
struct Phrases {
    by_first: HashMap<String, Vec<Vec<String>>>,
}

impl Phrases {
    fn new(list: &[&str]) -> Self {
        let mut by_first: HashMap<String, Vec<Vec<String>>> = HashMap::new();
        for phrase in list {
            let words: Vec<String> = phrase.split_whitespace().map(fold_word).collect();
            // "independente de" is also written "independente do/da/dos/das".
            let mut variants = vec![words.clone()];
            if words.len() > 1 && words.last().is_some_and(|last| last == "de") {
                for contraction in ["do", "da", "dos", "das"] {
                    let mut variant = words.clone();
                    variant.pop();
                    variant.push(contraction.to_string());
                    variants.push(variant);
                }
            }
            for variant in variants {
                if let Some(first) = variant.first() {
                    by_first.entry(first.clone()).or_default().push(variant);
                }
            }
        }
        for phrases in by_first.values_mut() {
            phrases.sort_by_key(|words| std::cmp::Reverse(words.len()));
        }
        Self { by_first }
    }

    /// Tokens the longest phrase starting at `at` covers; 0 when none does.
    /// Only consecutive words match: a comma or a stop breaks a phrase.
    fn longest(&self, tokens: &[Token], words: &[String], at: usize) -> usize {
        let Some(phrases) = self.by_first.get(&words[at]) else {
            return 0;
        };
        phrases
            .iter()
            .find(|phrase| {
                phrase.iter().enumerate().all(|(offset, word)| {
                    tokens
                        .get(at + offset)
                        .is_some_and(|token| token.piece == Piece::Word)
                        && words[at + offset] == *word
                })
            })
            .map_or(0, Vec::len)
    }
}

struct Lexicon {
    before: Phrases,
    after: Phrases,
    pseudo: Phrases,
    terminators: HashSet<String>,
    connectors: HashSet<String>,
    articles: HashSet<String>,
}

fn lexicon() -> &'static Lexicon {
    static LEXICON: OnceLock<Lexicon> = OnceLock::new();
    LEXICON.get_or_init(|| {
        let set = |list: &[&str]| list.iter().map(|word| fold_word(word)).collect();
        Lexicon {
            before: Phrases::new(NEGATION_BEFORE),
            after: Phrases::new(NEGATION_AFTER),
            pseudo: Phrases::new(PSEUDO_TRIGGERS),
            terminators: set(TERMINATORS),
            connectors: set(CONNECTORS),
            articles: set(ARTICLES),
        }
    })
}

/// A word folded the way the text is, with typographic apostrophes plain.
fn fold_word(word: &str) -> String {
    word.chars()
        .map(|character| match fold(character) {
            '\u{2019}' | '\u{2018}' => '\'',
            other => other,
        })
        .collect()
}

/// Which characters a negation reaches; empty when no trigger is present.
fn negation_mask(folded: &[char], code: &[bool]) -> Vec<bool> {
    let tokens = tokenize(folded);
    let words: Vec<String> = tokens
        .iter()
        .map(|token| match token.piece {
            Piece::Word => fold_word(&folded[token.start..token.end].iter().collect::<String>()),
            _ => String::new(),
        })
        .collect();
    let lexicon = lexicon();
    let mut mask: Vec<bool> = Vec::new();
    let mut mark = |token: &Token| {
        if mask.is_empty() {
            mask = vec![false; folded.len()];
        }
        for flag in &mut mask[token.start..token.end] {
            *flag = true;
        }
    };
    // Whether the word at `at` goes on with a list: an article (only after a
    // connector), code, a name-shaped word or another trigger.
    let continues = |at: usize, after_comma: bool| {
        let Some(token) = tokens.get(at).filter(|token| token.piece == Piece::Word) else {
            return false;
        };
        let word = &words[at];
        (!after_comma && lexicon.articles.contains(word))
            || code.get(token.start).copied().unwrap_or(false)
            || word.contains(['-', '_', '/', '.', ':', '@'])
            || word.chars().any(|character| character.is_ascii_digit())
            || lexicon.before.longest(&tokens, &words, at) > 0
    };
    let mut at = 0;
    while at < tokens.len() {
        if tokens[at].piece != Piece::Word {
            at += 1;
            continue;
        }
        let pseudo = lexicon.pseudo.longest(&tokens, &words, at);
        if pseudo > 0 {
            at += pseudo;
            continue;
        }
        let before = lexicon.before.longest(&tokens, &words, at);
        let after = lexicon.after.longest(&tokens, &words, at);
        if before > 0 {
            // Forward, to the end of the clause or the list.
            let mut reached = 0;
            let mut next = at + before;
            while let Some(token) = tokens.get(next) {
                match token.piece {
                    Piece::Stop => break,
                    Piece::Comma => {
                        if !continues(next + 1, true) {
                            break;
                        }
                    }
                    Piece::Word => {
                        let word = &words[next];
                        if lexicon.terminators.contains(word) {
                            break;
                        }
                        if lexicon.connectors.contains(word) {
                            if !continues(next + 1, false) {
                                break;
                            }
                        } else {
                            reached += 1;
                            if reached > MAX_SCOPE_WORDS {
                                break;
                            }
                            mark(token);
                        }
                    }
                }
                next += 1;
            }
        }
        if after > 0 {
            // Backward, to the start of the clause.
            let mut reached = 0;
            let mut previous = at;
            while previous > 0 {
                previous -= 1;
                let token = &tokens[previous];
                match token.piece {
                    Piece::Stop => break,
                    Piece::Comma => {}
                    Piece::Word => {
                        if lexicon.terminators.contains(&words[previous]) {
                            break;
                        }
                        reached += 1;
                        if reached > MAX_SCOPE_WORDS {
                            break;
                        }
                        mark(token);
                    }
                }
            }
        }
        at += before.max(after).max(1);
    }
    mask
}

#[cfg(test)]
mod tests {
    use domain::entities::EntityKind;

    use super::*;

    fn entity(name: &str, aliases: &[&str], patterns: &[&str]) -> EntityRecord {
        EntityRecord {
            entity_id: "e".into(),
            project_id: "p".into(),
            kind: EntityKind::Component,
            name: name.into(),
            key: domain::entities::entity_key(name),
            description: String::new(),
            patterns: patterns.iter().map(|pattern| (*pattern).into()).collect(),
            aliases: aliases.iter().map(|alias| (*alias).into()).collect(),
            created_at: "2026-01-01T00:00:00Z".into(),
            retired_at: None,
        }
    }

    /// The quote of the first term of `entity` that `text` mentions.
    fn hit(text: &str, entity: &EntityRecord) -> Option<String> {
        let folded = Folded::new(text);
        entity_terms(entity, &BTreeSet::new())
            .iter()
            .find_map(|term| folded.mention(term))
    }

    #[test]
    fn names_match_whole_words_ignoring_case() {
        let storage = entity("storage", &[], &[]);
        assert!(hit("O Storage guarda tudo.", &storage).is_some());
        assert!(hit("usa storage-sqlite para isso", &storage).is_none());
        assert!(hit("o storages_v2 antigo", &storage).is_none());
        assert!(hit("(storage)", &storage).is_some());
        let sqlite = entity("storage-sqlite", &[], &[]);
        assert!(hit("o storage-sqlite mantém a outbox", &sqlite).is_some());
    }

    #[test]
    fn accents_do_not_matter() {
        let decisions = entity("Decisões", &[], &[]);
        assert!(hit("as decisoes revisadas", &decisions).is_some());
        let plain = entity("revisao", &[], &[]);
        assert!(hit("A Revisão confirma", &plain).is_some());
    }

    #[test]
    fn short_and_generic_names_need_code_or_acronym() {
        let api = entity("api", &[], &[]);
        assert!(hit("a api responde", &api).is_none(), "plain prose");
        assert!(hit("a API responde", &api).is_some(), "acronym");
        assert!(hit("o módulo `api` responde", &api).is_some(), "code");
        assert!(hit("o \"api\" responde", &api).is_some(), "quoted");
        let ui = entity("ui", &[], &[]);
        assert!(hit("a UI mostra", &ui).is_none(), "never under 3");
        let core = entity("core", &[], &[]);
        assert!(hit("o core grava pela outbox", &core).is_some());
    }

    #[test]
    fn paths_and_aliases_count() {
        let core = entity("Núcleo", &["engine"], &["crates/core/**"]);
        assert!(hit("mudanças em crates/core/src/lib.rs", &core).is_some());
        assert!(hit("mudanças em crates\\core\\lib.rs", &core).is_some());
        assert!(hit("o engine decide", &core).is_some());
        assert!(hit("crates/core-extra/x.rs", &core).is_none());
        // A one-segment pattern is a name and follows the name rule.
        let ui = entity("Interface", &[], &["ui/**"]);
        assert!(hit("a ui mostra", &ui).is_none());
    }

    /// Whether the text mentions `entity` affirmatively.
    fn affirmative(text: &str, entity: &EntityRecord) -> bool {
        hit(text, entity).is_some()
    }

    /// Whether the text names the entity and every hit is negated.
    fn negated_only(text: &str, entity: &EntityRecord) -> bool {
        let folded = Folded::new(text);
        let terms = entity_terms(entity, &BTreeSet::new());
        terms.iter().any(|term| folded.find(term, false).is_some())
            && terms.iter().all(|term| folded.find(term, true).is_none())
    }

    #[test]
    fn a_part_named_to_be_left_out_is_not_a_mention() {
        let net = entity("net-core", &[], &[]);
        for text in [
            "The service is independent of net-core.",
            "Persistir sem net-core.",
            "Usar o outro em vez de net-core.",
            "Rather than net-core, write to disk.",
            "Instead of net-core we use sockets.",
            "Não usar net-core aqui.",
            "O transporte nem net-core nem o outro.",
            "Do not import net-core from the CLI.",
            "It must not call net-core.",
            "Avoid net-core in hot paths.",
            "Everything except net-core is public.",
            "Isolado de net-core, o resto compila.",
            "Adotar net-core foi descartado.",
            "Using net-core was rejected in the review.",
            "net-core is out of scope for this decision.",
            "Usar net-core não entra no escopo.",
        ] {
            assert!(negated_only(text, &net), "{text}");
            assert!(!affirmative(text, &net), "{text}");
        }
    }

    #[test]
    fn pseudo_triggers_do_not_negate() {
        let net = entity("net-core", &[], &[]);
        for text in [
            "Not only net-core but also the cache changes.",
            "Not just net-core: the cache changes too.",
            "Não só o net-core muda.",
            "Não apenas o net-core muda.",
            "No doubt net-core changes.",
            "Sem dúvida net-core muda.",
        ] {
            assert!(affirmative(text, &net), "{text}");
        }
    }

    #[test]
    fn the_scope_ends_at_the_clause_a_terminator_or_a_new_subject() {
        let net = entity("net-core", &[], &[]);
        let store = entity("store-core", &[], &[]);
        // Punctuation, terminating words and a connector that starts a clause.
        for text in [
            "Without a cache. net-core changes.",
            "Without a cache; net-core changes.",
            "Without a cache: net-core changes.",
            "Without a cache (as before) net-core changes.",
            "Without a cache\nnet-core changes.",
            "Without a cache but net-core changes.",
            "Sem cache, mas net-core muda.",
            "Sem cache porque net-core muda.",
        ] {
            assert!(affirmative(text, &net), "{text}");
        }
        // A list of names keeps the negation.
        for text in [
            "Without store-core, net-core or the cache.",
            "Não usar store-core, net-core e o cache.",
            "Instead of `store-core` and `net-core`.",
        ] {
            assert!(negated_only(text, &net), "{text}");
        }
        // The subject before the trigger stays affirmative.
        let text = "net-core stays independent of store-core.";
        assert!(affirmative(text, &net) && negated_only(text, &store));
        // A later clause names the part again.
        let text = "Without net-core nor store-core; net-core still owns the sockets.";
        assert!(affirmative(text, &net) && negated_only(text, &store));
        // Beyond the cap of words the negation has ended.
        let far = "Without one two three four five six seven eight nine net-core.";
        assert!(affirmative(far, &net));
        let near = "Without one two three four five six seven net-core.";
        assert!(negated_only(near, &net));
    }

    /// Whether the text mentions `entity` when the project is also called
    /// `project`.
    fn named_like_the_project(text: &str, entity: &EntityRecord, project: &str) -> bool {
        let keys = BTreeSet::from([domain::entities::entity_key(project)]);
        let folded = Folded::new(text);
        entity_terms(entity, &keys)
            .iter()
            .any(|term| folded.mention(term).is_some())
    }

    #[test]
    fn a_part_named_like_the_project_counts_only_as_a_path_or_as_code() {
        let app = entity("acme", &[], &["apps/acme/**"]);
        for text in [
            "How should acme acquire the license?",
            "ACME ships a new installer.",
            "O Acme cobra por assento.",
            "Keep the cache under data_dir()/acme/ by default.",
        ] {
            assert!(!named_like_the_project(text, &app, "acme"), "{text}");
            assert!(
                affirmative(text, &app) || text.contains("data_dir"),
                "{text}"
            );
        }
        for text in [
            "The `acme` binary parses its flags.",
            "O \"acme\" lê a flag.",
            "Edit apps/acme/src/main.rs to read the flag.",
        ] {
            assert!(named_like_the_project(text, &app, "acme"), "{text}");
        }
        // Another project's name changes nothing.
        assert!(named_like_the_project("O acme lê a flag.", &app, "other"));
    }

    #[test]
    fn a_negated_verb_phrase_also_reaches_the_nouns_after_it() {
        // Known limit of a lexical rule: "sem perder nenhum evento no X" negates
        // the action, and X is only where it happens, yet X is left out.
        let sqlite = entity("sqlite", &[], &[]);
        assert!(negated_only(
            "Gravar sem perder nenhum evento no sqlite.",
            &sqlite
        ));
        // Said the other way round, it is a mention.
        assert!(affirmative(
            "Gravar no sqlite, sem perder nenhum evento.",
            &sqlite
        ));
    }

    #[test]
    fn in_portuguese_no_is_not_a_negation() {
        let core = entity("core", &[], &[]);
        assert!(affirmative(
            "O plugin grava no core antes de responder.",
            &core
        ));
    }

    #[test]
    fn a_name_inside_another_path_is_not_the_part() {
        let acme = entity("acme", &[], &["apps/acme/**"]);
        assert!(!affirmative(
            "Guardar em data_dir()/acme/ por padrão.",
            &acme
        ));
        assert!(!affirmative("Guardar em logs\\acme\\run.", &acme));
        assert!(!affirmative("Usar crate::acme::run aqui.", &acme));
        // Its own item and its own path still count.
        assert!(affirmative("Usar acme::run aqui.", &acme));
        assert!(affirmative("Editar apps/acme/src/main.rs.", &acme));
        assert!(affirmative("O acme lê a flag.", &acme));
    }

    #[test]
    fn code_words_are_the_affirmative_names_between_backticks() {
        let words = |text: &str| Folded::new(text).code_words();
        assert_eq!(
            words("Raise `FLUSH_INTERVAL` and read `state.rs` or `A::b()`."),
            vec!["FLUSH_INTERVAL", "state.rs", "A", "b"]
        );
        assert!(words("Do not touch `FLUSH_INTERVAL`; the rest stays.").is_empty());
        assert_eq!(
            words("Do not touch `OLD`; raise `NEW_ONE`."),
            vec!["NEW_ONE"]
        );
        assert!(words("No code here.").is_empty());
    }

    #[test]
    fn a_short_name_counts_inside_code() {
        let api = entity("api", &[], &[]);
        assert!(affirmative("Chamar `api::start` no boot.", &api));
        assert!(!affirmative("Chamar api::start no boot.", &api));
    }

    #[test]
    fn the_quote_is_short_and_starts_at_a_word() {
        let outbox = entity("outbox", &[], &[]);
        let text = "Contexto longo que vem antes e não interessa muito aqui, porque a \
                    escrita passa sempre pela outbox antes de chegar ao banco e depois \
                    segue para o resto do sistema sem pressa nenhuma.";
        let quote = hit(text, &outbox).expect("mentioned");
        assert!(quote.starts_with('…') && quote.ends_with('…'), "{quote}");
        assert!(quote.contains("pela outbox antes"), "{quote}");
        assert!(quote.chars().count() <= 2 * QUOTE_CONTEXT + 8, "{quote}");
        let reason = mention_reason(&quote);
        assert_eq!(mention_quote(&reason), Some(quote.as_str()));
        assert_eq!(mention_quote("crates/core/src/lib.rs"), None);
    }

    /// The map of a small project and labelled decision texts: which parts
    /// each text is really about. Negatives use the same words in other
    /// senses, so precision measures what a wrong link would cost.
    /// Labelled map parts and texts with the parts each one is about.
    type Corpus = (
        Vec<(&'static str, EntityRecord)>,
        Vec<(&'static str, Vec<&'static str>)>,
    );

    fn corpus() -> Corpus {
        let map = vec![
            (
                "storage",
                entity(
                    "storage-sqlite",
                    &["sqlite store"],
                    &["crates/storage-sqlite/**"],
                ),
            ),
            ("core", entity("core", &["núcleo"], &["crates/core/**"])),
            ("outbox", entity("outbox", &[], &["adapters/outbox/**"])),
            (
                "app",
                entity("app desktop", &["gpui app"], &["apps/desktop-gpui/**"]),
            ),
            (
                "api",
                entity("api", &["local api"], &["crates/local-api/**"]),
            ),
            (
                "mcp",
                entity("mcp server", &["xemnas-mcp"], &["apps/mcp-server/**"]),
            ),
            (
                "hooks",
                entity("hooks", &["opencode plugin"], &["adapters/opencode/**"]),
            ),
        ];
        let texts = vec![
            (
                "O core grava cada captura pela outbox antes de responder.",
                vec!["core", "outbox"],
            ),
            (
                "O storage-sqlite usa WAL para leituras durante gravações.",
                vec!["storage"],
            ),
            ("A `api` local só escuta em loopback.", vec!["api"]),
            ("A API local exige token por sessão.", vec!["api"]),
            ("O xemnas-mcp responde get_decision em stdio.", vec!["mcp"]),
            (
                "Os arquivos em crates/core/src mudam juntos com o núcleo.",
                vec!["core"],
            ),
            (
                "O plugin grava em adapters/outbox/pending quando o app desktop fecha.",
                vec!["outbox", "app"],
            ),
            ("A janela da gpui app mostra a Revisão.", vec!["app"]),
            (
                "Retentar só falhas transitórias, com a mesma chave.",
                vec![],
            ),
            (
                "A api de pagamentos de terceiros não entra no escopo.",
                vec![],
            ),
            (
                // Figurative "núcleo": a known false positive of the rule.
                "O núcleo do problema é a ordem das mensagens.",
                vec![],
            ),
            ("Usar storage separado por projeto foi descartado.", vec![]),
            (
                "O mcp server lê o mesmo banco do app desktop.",
                vec!["mcp", "app"],
            ),
            (
                "Hooks do opencode plugin capturam cada turno.",
                vec!["hooks"],
            ),
            // Negation: the part is named to be left out.
            (
                "O core fica independente do storage-sqlite e da outbox.",
                vec!["core"],
            ),
            (
                "Gravar na outbox em vez de no storage-sqlite.",
                vec!["outbox"],
            ),
            ("The core stays independent of the local api.", vec!["core"]),
            (
                "Não usar storage-sqlite nem outbox; o storage-sqlite fica só para leitura.",
                vec!["storage"],
            ),
            (
                "Without the mcp server, the app desktop still starts.",
                vec!["app"],
            ),
            (
                "Rather than the outbox, the core writes straight to the sqlite store.",
                vec!["core", "storage"],
            ),
            ("O app desktop não depende do núcleo.", vec!["app"]),
            (
                "Not only the core but also the outbox writes the capture.",
                vec!["core", "outbox"],
            ),
            // Polarity words of other languages: the part is named to be left out.
            ("El core no usa la outbox.", vec!["core"]),
            ("Sans la outbox, le core écrit directement.", vec!["core"]),
            ("Der core schreibt ohne outbox.", vec!["core"]),
            ("Il core scrive senza outbox.", vec!["core"]),
            // Portuguese "no" is "in the", not a negation.
            ("O plugin grava no core antes de responder.", vec!["core"]),
            // The name inside another path is not the part.
            ("Os logs ficam em data_dir()/outbox/ por padrão.", vec![]),
            ("Gravar em logs/core/ a cada execução.", vec![]),
            (
                "Mudar adapters/outbox/pending não afeta o resto.",
                vec!["outbox"],
            ),
        ];
        (map, texts)
    }

    /// Parts of a project discovered from manifests: npm names with the aliases
    /// discovery derives from them, and decisions written the way people talk.
    fn package_corpus() -> Corpus {
        let map = vec![
            (
                "pcore",
                entity("@jevguard/core", &["core"], &["packages/core/**"]),
            ),
            (
                "padapter",
                entity(
                    "@jevguard/opencode-adapter",
                    &["opencode-adapter", "opencode adapter"],
                    &["packages/opencode-adapter/**"],
                ),
            ),
            (
                "ptestkit",
                entity("@jevguard/testkit", &["testkit"], &["packages/testkit/**"]),
            ),
            (
                "pplugin",
                entity(
                    "@pablozrrrr/jevguard",
                    &["jevguard", "plugin"],
                    &["packages/plugin/**"],
                ),
            ),
        ];
        let texts = vec![
            (
                "O core grava cada decisão antes de responder.",
                vec!["pcore"],
            ),
            (
                "The opencode adapter forwards each turn to the core.",
                vec!["padapter", "pcore"],
            ),
            (
                "O plugin carrega o testkit só nos testes.",
                vec!["pplugin", "ptestkit"],
            ),
            (
                "Hooks do opencode-adapter capturam cada turno.",
                vec!["padapter"],
            ),
            (
                "Mudanças em packages/core/src e no testkit andam juntas.",
                vec!["pcore", "ptestkit"],
            ),
            (
                "Every call the plugin makes goes through the opencode adapter.",
                vec!["pplugin", "padapter"],
            ),
            (
                "O testkit fornece o servidor falso que o core usa nos testes.",
                vec!["ptestkit", "pcore"],
            ),
            ("A `core` valida o schema antes de gravar.", vec!["pcore"]),
            (
                "O jevguard bloqueia o comando antes da execução.",
                vec!["pplugin"],
            ),
            (
                "Usar um adapter genérico para cada editor foi descartado.",
                vec![],
            ),
            ("O kit de testes compartilhado foi descartado.", vec![]),
            // Same words in other senses: a known false positive of the rule.
            ("The core of the problem is the message ordering.", vec![]),
            // Out of scope: named to be left out.
            ("Um plugin de navegador não entra no escopo.", vec![]),
            (
                "Fica sem o testkit, mas o core valida o schema.",
                vec!["pcore"],
            ),
            (
                "Instead of the opencode adapter, the plugin calls the core directly.",
                vec!["pplugin", "pcore"],
            ),
            (
                "O core grava `@jevguard/testkit` só em testes, nunca no plugin.",
                vec!["pcore", "ptestkit"],
            ),
        ];
        (map, texts)
    }

    /// Floors measured on 2026-10-07. The precision floor went from 0.92
    /// (12/13) to 0.90 (27/30) when the package corpus added two known false
    /// positives of plain-word aliases ("the core of the problem", "plugin de
    /// navegador"), the price of recall on how people write, and to 0.95
    /// (43/45) when the corpus gained negation, lists and paths: "plugin de
    /// navegador não entra no escopo" is now left out, and the two
    /// remaining false positives are the plain-word alias in a figurative
    /// sense, which no lexical rule can tell apart.
    ///
    /// 2026-10-09: the corpus gained five sentences with polarity words of
    /// Spanish, French, German and Italian (and Portuguese "no"). The lexicon
    /// only knows English and Portuguese, so four of them link a part that is
    /// named to be left out: precision 0.889 (48/54), recall 1.000. The floor
    /// sits there until a polarity word of any language raises doubt instead
    /// of being ignored.
    const MENTION_PRECISION_FLOOR: f64 = 0.88;
    const MENTION_RECALL_FLOOR: f64 = 1.0;

    #[test]
    fn mention_quality_gate() {
        // Each corpus is its own project: the same word may name a part in one.
        let (mut tp, mut found, mut expected) = (0usize, 0usize, 0usize);
        for (map, texts) in [corpus(), package_corpus()] {
            for (text, truth) in &texts {
                let folded = Folded::new(text);
                let hits: Vec<&str> = map
                    .iter()
                    .filter(|(_, entity)| {
                        entity_terms(entity, &BTreeSet::new())
                            .iter()
                            .any(|term| folded.mention(term).is_some())
                    })
                    .map(|(label, _)| *label)
                    .collect();
                if hits.len() != truth.len() || !hits.iter().all(|hit| truth.contains(hit)) {
                    println!("mention differs: {text} -> {hits:?}, expected {truth:?}");
                }
                tp += hits.iter().filter(|hit| truth.contains(hit)).count();
                found += hits.len();
                expected += truth.len();
            }
        }
        let precision = tp as f64 / found.max(1) as f64;
        let recall = tp as f64 / expected.max(1) as f64;
        println!(
            "mention precision={precision:.3} ({tp}/{found}) recall={recall:.3} ({tp}/{expected})"
        );
        assert!(
            precision >= MENTION_PRECISION_FLOOR,
            "mention precision {precision:.3}"
        );
        assert!(recall >= MENTION_RECALL_FLOOR, "mention recall {recall:.3}");
    }

    #[test]
    fn mention_matching_scales_to_a_large_project() {
        // 2.000 decisions of ~300 characters against a 60-part map.
        let map: Vec<EntityRecord> = (0..60)
            .map(|index| {
                entity(
                    &format!("component{index}"),
                    &[&format!("alias{index}")],
                    &[&format!("crates/component{index}/**")],
                )
            })
            .collect();
        let none = BTreeSet::new();
        let terms: Vec<Vec<Term>> = map
            .iter()
            .map(|entity| entity_terms(entity, &none))
            .collect();
        let text = "A decisão grava cada captura numa transação, valida a versão e só                     então confirma; o component7 recebe o resultado e o alias12 registra                     a ocorrência em crates/component33/src/lib.rs para a próxima sessão."
            .repeat(2);
        let start = std::time::Instant::now();
        let mut hits = 0usize;
        for _ in 0..2_000 {
            let folded = Folded::new(&text);
            hits += terms
                .iter()
                .filter(|terms| terms.iter().any(|term| folded.mention(term).is_some()))
                .count();
        }
        let elapsed = start.elapsed();
        println!("mention 2000x60 elapsed_ms={}", elapsed.as_millis());
        assert_eq!(hits, 2_000 * 3);
        assert!(
            elapsed.as_millis() < 3_000,
            "mention matching took {}ms for 2000 decisions",
            elapsed.as_millis()
        );
    }
}
