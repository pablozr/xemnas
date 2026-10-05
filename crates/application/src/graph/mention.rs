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
}

/// The terms that name `entity`: name, aliases and, for a component, the
/// literal part of each path pattern. Terms the rule can never accept are
/// left out.
pub(crate) fn entity_terms(entity: &EntityRecord) -> Vec<Term> {
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
        let term = Term { chars, path };
        if !terms.contains(&term) {
            terms.push(term);
        }
    }
    terms
}

/// A text ready to be searched: the original characters and their folded
/// form, one to one.
pub(crate) struct Folded {
    original: Vec<char>,
    folded: Vec<char>,
}

impl Folded {
    pub(crate) fn new(text: &str) -> Self {
        let original: Vec<char> = text.chars().collect();
        let folded = original.iter().copied().map(fold).collect();
        Self { original, folded }
    }

    /// The quote around the first mention of `term`, when the text has one
    /// the rule accepts.
    pub(crate) fn mention(&self, term: &Term) -> Option<String> {
        let size = term.chars.len();
        if size == 0 || size > self.folded.len() {
            return None;
        }
        (0..=self.folded.len() - size)
            .find(|&start| {
                self.folded[start..start + size] == term.chars[..] && self.accepts(start, term)
            })
            .map(|start| self.quote(start, size))
    }

    fn accepts(&self, start: usize, term: &Term) -> bool {
        let size = term.chars.len();
        let before = start.checked_sub(1).map(|at| self.original[at]);
        let after = self.original.get(start + size).copied();
        if before.is_some_and(word_char) || after.is_some_and(word_char) {
            return false;
        }
        if term.path || size >= MIN_PLAIN {
            return true;
        }
        let written = &self.original[start..start + size];
        let acronym =
            written.iter().any(|c| c.is_alphabetic()) && !written.iter().any(|c| c.is_lowercase());
        let marked = before.is_some_and(code_mark) && after.is_some_and(code_mark);
        acronym || marked
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
        entity_terms(entity)
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
    fn corpus() -> (
        Vec<(&'static str, EntityRecord)>,
        Vec<(&'static str, Vec<&'static str>)>,
    ) {
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
        ];
        (map, texts)
    }

    /// Floors measured on 2026-10-05; raise them when the rule improves.
    const MENTION_PRECISION_FLOOR: f64 = 0.92;
    const MENTION_RECALL_FLOOR: f64 = 1.0;

    #[test]
    fn mention_quality_gate() {
        let (map, texts) = corpus();
        let (mut tp, mut found, mut expected) = (0usize, 0usize, 0usize);
        for (text, truth) in &texts {
            let folded = Folded::new(text);
            let hits: Vec<&str> = map
                .iter()
                .filter(|(_, entity)| {
                    entity_terms(entity)
                        .iter()
                        .any(|term| folded.mention(term).is_some())
                })
                .map(|(label, _)| *label)
                .collect();
            tp += hits.iter().filter(|hit| truth.contains(hit)).count();
            found += hits.len();
            expected += truth.len();
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
        let terms: Vec<Vec<Term>> = map.iter().map(entity_terms).collect();
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
