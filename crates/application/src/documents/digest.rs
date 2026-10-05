//! The central parts of a document.
//!
//! Documentation is long and mostly not decisions: install steps, usage,
//! changelogs, plans. Sent whole, every file made candidates for Revisão and
//! most of them were noise. A digest keeps what states a choice (an ADR's
//! decision and consequences, a README's architecture or conventions section,
//! sentences that say "we decided", "never", "instead of") and drops the rest;
//! a document with no such part is not sent at all. It is deterministic and
//! free: no model reads anything to decide what is worth reading.

use super::DocumentKind;

/// Most characters of a digest sent to extraction.
pub const DIGEST_CHARS: usize = 6_000;
/// Most characters kept of one section.
const SECTION_CHARS: usize = 1_500;
/// Characters of the opening paragraph kept as context.
const OPENING_CHARS: usize = 400;

/// What is done with a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// It has central parts: send the digest.
    Central,
    /// It has none, or is not a source of decisions; why, in a few words.
    Skip(&'static str),
}

/// The central parts of one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digest {
    /// Whether it is worth sending.
    pub verdict: Verdict,
    /// What to send: the title, the opening and the central sections.
    pub text: String,
    /// Headings of the sections kept, in order.
    pub sections: Vec<String>,
    /// How strongly the document speaks in decisions; ranks documents of the
    /// same kind when only some can be sent.
    pub score: usize,
}

impl Digest {
    fn skip(reason: &'static str) -> Self {
        Self {
            verdict: Verdict::Skip(reason),
            text: String::new(),
            sections: Vec::new(),
            score: 0,
        }
    }
}

/// Headings that announce a choice, a rule or its reasons.
const CENTRAL_HEADINGS: [&str; 30] = [
    "decisao",
    "decisoes",
    "decision",
    "decidido",
    "arquitetura",
    "architecture",
    "principio",
    "principle",
    "restric",
    "constraint",
    "convenc",
    "convention",
    "regra",
    "rule",
    "rationale",
    "justificativa",
    "motivo",
    "consequenc",
    "consequence",
    "trade-off",
    "tradeoff",
    "alternativa",
    "alternative",
    "stack",
    "premissa",
    "assumption",
    "invariante",
    "invariant",
    "padroes",
    "politica",
];

/// Headings of ADR sections that carry the decision (an ADR's own "context"
/// counts, a README's does not).
const ADR_HEADINGS: [&str; 5] = ["contexto", "context", "escopo", "scope", "problema"];

/// Headings of sections that are about using the thing, not choosing it.
const PERIPHERAL_HEADINGS: [&str; 34] = [
    "instal",
    "getting started",
    "quickstart",
    "comeco",
    "como rodar",
    "how to run",
    "uso",
    "usage",
    "comandos",
    "commands",
    "scripts",
    "contribu",
    "changelog",
    "roadmap",
    "todo",
    "proximos passos",
    "next steps",
    "faq",
    "troubleshoot",
    "license",
    "licenca",
    "credits",
    "creditos",
    "agradec",
    "acknowledg",
    "sumario",
    "indice",
    "table of contents",
    "exemplo",
    "example",
    "referencia",
    "teste",
    "badge",
    "suporte",
];

/// Phrases that state a choice or a rule, folded (lowercase, no accents).
const DECISION_PHRASES: [&str; 26] = [
    "decidimos",
    "decidiu-se",
    "optamos",
    "escolhemos",
    "adotamos",
    "passamos a ",
    "nao usamos",
    "nao utilizamos",
    "em vez de",
    "ao inves de",
    "nunca ",
    "sempre ",
    "nao deve",
    "nao devem",
    "e proibido",
    "obrigatorio",
    "we decided",
    "we chose",
    "we do not",
    "we don't",
    "instead of",
    "must not",
    "must ",
    "never ",
    "should not",
    "is required",
];

/// Path pieces of documents that are not sources of decisions.
const SKIPPED_PATH_PARTS: [&str; 16] = [
    "changelog",
    "release-notes",
    "releasenotes",
    "roadmap",
    "todo",
    "research",
    "pesquisa",
    "meeting",
    "retro",
    "license",
    "licence",
    "copying",
    "code_of_conduct",
    "code-of-conduct",
    "contributors",
    "third_party",
];

/// ADR statuses that mean "not in force".
const NOT_IN_FORCE: [&str; 10] = [
    "proposto",
    "proposed",
    "rascunho",
    "draft",
    "rejeitado",
    "rejected",
    "obsoleto",
    "deprecated",
    "superado",
    "superseded",
];

/// Lowercase with the Portuguese accents folded, so one list serves both
/// "decisão" and "decisao".
fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

struct Section {
    heading: String,
    body: String,
}

/// Title, opening text and sections of a Markdown text; headings inside code
/// fences are text, and fenced code itself is replaced by a short note.
fn sections(content: &str) -> (String, Vec<Section>) {
    let mut title = String::new();
    let mut parts = vec![Section {
        heading: String::new(),
        body: String::new(),
    }];
    let mut fenced = false;
    let mut front_matter = content.trim_start().starts_with("---");
    let mut seen_front = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if front_matter {
            if trimmed == "---" {
                if seen_front {
                    front_matter = false;
                }
                seen_front = true;
            }
            continue;
        }
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            if !fenced {
                if let Some(section) = parts.last_mut() {
                    section.body.push_str("[código omitido]\n");
                }
            }
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let level = trimmed.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&level) && trimmed[level..].starts_with(' ') {
            let heading = trimmed[level..]
                .trim()
                .trim_end_matches('#')
                .trim()
                .to_owned();
            if level == 1 && title.is_empty() {
                title = heading;
            } else {
                parts.push(Section {
                    heading,
                    body: String::new(),
                });
            }
            continue;
        }
        if let Some(section) = parts.last_mut() {
            section.body.push_str(line);
            section.body.push('\n');
        }
    }
    (title, parts)
}

/// The status an ADR declares, folded: from a `Status:` line or a `Status`
/// section.
fn adr_status(title_and_parts: &[Section], content: &str) -> Option<String> {
    for line in content.lines().take(40) {
        let folded = fold(line.trim().trim_start_matches(['-', '*', ' ']));
        let folded = folded.replace("**", "");
        if let Some(rest) = folded.strip_prefix("status") {
            let rest = rest.trim_start_matches([':', ' ', '*']).trim();
            if !rest.is_empty() {
                return Some(rest.to_owned());
            }
        }
    }
    title_and_parts
        .iter()
        .find(|section| fold(&section.heading) == "status")
        .and_then(|section| section.body.lines().find(|line| !line.trim().is_empty()))
        .map(|line| fold(line.trim()))
}

fn decision_hits(text: &str) -> usize {
    let folded = fold(text);
    DECISION_PHRASES
        .iter()
        .map(|phrase| folded.matches(phrase).count())
        .sum::<usize>()
        .min(5)
}

fn mentions(heading: &str, words: &[&str]) -> bool {
    let folded = fold(heading);
    words.iter().any(|word| folded.contains(word))
}

/// Why a path is not a source of decisions, if it is not.
fn skipped_path(path: &str) -> Option<&'static str> {
    let folded = fold(path);
    let file = folded.rsplit('/').next().unwrap_or(&folded);
    if SKIPPED_PATH_PARTS
        .iter()
        .any(|part| folded.split('/').any(|piece| piece.contains(part)))
    {
        return Some("registro, plano ou nota, não decisão");
    }
    // README.pt-br.md, README.zh-CN.md: a translation of the README.
    if file.starts_with("readme.") && file.matches('.').count() >= 2 {
        return Some("tradução");
    }
    if folded
        .split('/')
        .any(|piece| matches!(piece, "i18n" | "locales" | "translations" | "node_modules"))
    {
        return Some("tradução");
    }
    None
}

fn clip(text: &str, max: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let cut: String = text.chars().take(max).collect();
    // End on a sentence or a line, not in the middle of a word.
    let end = cut.rfind(['.', '\n']).map_or(cut.len(), |at| at + 1);
    format!("{}…", cut[..end].trim_end())
}

/// The digest of a document: its central parts, or why it has none.
pub fn digest(kind: DocumentKind, path: &str, content: &str) -> Digest {
    if let Some(reason) = skipped_path(path) {
        return Digest::skip(reason);
    }
    let (title, parts) = sections(content);
    if kind == DocumentKind::Adr {
        if let Some(status) = adr_status(&parts, content) {
            if NOT_IN_FORCE.iter().any(|word| status.starts_with(word)) {
                return Digest::skip("ADR que ainda não vale");
            }
        }
    }
    let adr = kind == DocumentKind::Adr;
    let mut kept: Vec<(&Section, usize)> = Vec::new();
    for section in parts.iter().filter(|section| !section.heading.is_empty()) {
        let peripheral = mentions(&section.heading, &PERIPHERAL_HEADINGS);
        let central = mentions(&section.heading, &CENTRAL_HEADINGS)
            || (adr && mentions(&section.heading, &ADR_HEADINGS));
        let hits = decision_hits(&section.body);
        // A peripheral heading wins over a few "must"s: install steps say
        // "you must have", and are not decisions.
        let keep = (central && !(peripheral && !adr)) || (!peripheral && hits >= 2);
        if keep && !section.body.trim().is_empty() {
            kept.push((section, hits + if central { 2 } else { 0 }));
        }
    }
    let score: usize = kept.iter().map(|(_, score)| score).sum();
    let needed = match kind {
        DocumentKind::Adr | DocumentKind::Spec => 1,
        DocumentKind::Readme | DocumentKind::Guide => 3,
    };
    if kept.is_empty() || score < needed {
        return Digest::skip("sem decisões explícitas");
    }

    let mut text = String::new();
    if !title.is_empty() {
        text.push_str(&format!("# {title}\n\n"));
    }
    let opening = parts
        .first()
        .map(|part| clip(&part.body.replace("[código omitido]", ""), OPENING_CHARS))
        .unwrap_or_default();
    if !opening.is_empty() {
        text.push_str(&opening);
        text.push_str("\n\n");
    }
    let mut names = Vec::new();
    for (section, _) in kept {
        let block = format!(
            "## {}\n{}\n\n",
            section.heading,
            clip(&section.body, SECTION_CHARS)
        );
        if text.chars().count() + block.chars().count() > DIGEST_CHARS {
            break;
        }
        text.push_str(&block);
        names.push(section.heading.clone());
    }
    Digest {
        verdict: Verdict::Central,
        text: text.trim_end().to_owned(),
        sections: names,
        score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADR: &str =
        "# ADR-0007: Visão gerada pela IA\n\n- **Status:** aceito\n- **Data:** 2026-09-30\n\n\
        ## Contexto\n\nO app só conhecia diffs.\n\n## Decisão\n\nDecidimos gerar a visão com o \
        provedor ativo, nunca sem consentimento.\n\n## Consequências\n\nCusta tokens.\n\n\
        ## Alternativas rejeitadas\n\nGerar offline.\n";

    #[test]
    fn an_accepted_adr_keeps_its_decision_parts() {
        let digest = digest(DocumentKind::Adr, "docs/adr/0007-visao.md", ADR);
        assert_eq!(digest.verdict, Verdict::Central);
        assert_eq!(
            digest.sections,
            vec![
                "Contexto",
                "Decisão",
                "Consequências",
                "Alternativas rejeitadas"
            ]
        );
        assert!(digest.text.contains("Decidimos gerar a visão"));
        assert!(digest.score >= 3);
    }

    #[test]
    fn an_adr_that_is_not_in_force_is_skipped() {
        let text = ADR.replace("aceito", "proposto");
        let result = digest(DocumentKind::Adr, "docs/adr/0007-visao.md", &text);
        assert_eq!(result.verdict, Verdict::Skip("ADR que ainda não vale"));
        let text = ADR.replace("**Status:** aceito", "Status: Superseded by 0009");
        assert!(matches!(
            digest(DocumentKind::Adr, "adr/0007.md", &text).verdict,
            Verdict::Skip(_)
        ));
    }

    #[test]
    fn a_readme_keeps_architecture_and_drops_install_and_usage() {
        let readme = "# App\n\nUm app de decisões.\n\n## Instalação\n\nVocê deve ter Rust. \
            Sempre rode `cargo build`.\n\n## Uso\n\nRode o app.\n\n## Arquitetura\n\nDecidimos \
            usar SQLite em vez de Postgres: o app é local. Nunca grave segredos no banco.\n\n\
            ## Licença\n\nMIT\n";
        let digest = digest(DocumentKind::Readme, "README.md", readme);
        assert_eq!(digest.verdict, Verdict::Central);
        assert_eq!(digest.sections, vec!["Arquitetura"]);
        assert!(!digest.text.contains("cargo build"));
        assert!(digest.text.contains("em vez de Postgres"));
    }

    #[test]
    fn a_readme_without_decisions_is_skipped() {
        let readme = "# App\n\n## Instalação\n\nBaixe.\n\n## Uso\n\nRode.\n";
        assert_eq!(
            digest(DocumentKind::Readme, "README.md", readme).verdict,
            Verdict::Skip("sem decisões explícitas")
        );
    }

    #[test]
    fn logs_plans_and_translations_are_skipped_by_their_path() {
        for path in [
            "CHANGELOG.md",
            "docs/release-notes/1.2.md",
            "docs/roadmap/fase-4.md",
            "docs/pesquisas/graficos.md",
            "README.pt-br.md",
            "docs/i18n/en/guia.md",
        ] {
            assert!(
                matches!(
                    digest(DocumentKind::Guide, path, ADR).verdict,
                    Verdict::Skip(_)
                ),
                "{path}"
            );
        }
        assert_eq!(
            digest(DocumentKind::Readme, "README.md", ADR).verdict,
            Verdict::Central
        );
    }

    #[test]
    fn code_is_left_out_and_headings_in_fences_are_not_sections() {
        let text = "# Guia\n\n## Convenções\n\nUse sempre erros tipados.\n\n```rust\n\
            // nunca assim\n# Decisão falsa\nfn x() {}\n```\n\nNunca use unwrap.\n";
        let digest = digest(DocumentKind::Guide, "docs/guia.md", text);
        assert_eq!(digest.verdict, Verdict::Central);
        assert_eq!(digest.sections, vec!["Convenções"]);
        assert!(!digest.text.contains("fn x()"));
        assert!(digest.text.contains("[código omitido]"));
    }

    #[test]
    fn a_digest_never_exceeds_its_budget() {
        let long = "palavra ".repeat(400);
        let mut text = String::from("# Spec\n\n");
        for n in 0..20 {
            text.push_str(&format!("## Decisão {n}\n\nDecidimos {long}.\n\n"));
        }
        let digest = digest(DocumentKind::Spec, "specs/x.md", &text);
        assert!(digest.text.chars().count() <= DIGEST_CHARS);
        assert!(digest.sections.len() < 20);
    }
}
