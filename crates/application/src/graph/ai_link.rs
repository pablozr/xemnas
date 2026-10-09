//! Reason of a suggested tie proposed by the AI from the decision's text.
//!
//! Like a mention, it is a weak link: the quote the model copied from the
//! decision and its one-sentence reason are kept in the edge's `reason`, so
//! the review (manual or automatic) sees what the model saw. The prefix tells
//! it apart from mention, file and dependency reasons.

/// Prefix of the reason of a suggestion proposed by the AI; the quote follows
/// between double quotes and, after a line break, the model's reason.
pub const AI_LINK_REASON: &str = "sugerido pela IA: ";

/// The reason of a suggestion that came from the extractor, not from the
/// separate call: it named the component together with the decision itself.
pub const EXTRACTED_LINK_WHY: &str = "indicado pelo extrator junto com a decisão";

/// Longest reason or quote kept, in characters.
const MAX_PART_CHARS: usize = 240;

fn one_line(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_PART_CHARS)
        .collect()
}

/// The reason stored on a suggestion proposed from `quote`.
pub fn ai_link_reason(quote: &str, reason: &str) -> String {
    let (quote, reason) = (one_line(quote), one_line(reason));
    if reason.is_empty() {
        format!("{AI_LINK_REASON}\"{quote}\"")
    } else {
        format!("{AI_LINK_REASON}\"{quote}\"\n{reason}")
    }
}

fn body(reason: &str) -> Option<(&str, &str)> {
    let rest = reason.strip_prefix(AI_LINK_REASON)?.strip_prefix('"')?;
    Some(match rest.split_once("\"\n") {
        Some((quote, why)) => (quote, why),
        None => (rest.strip_suffix('"').unwrap_or(rest), ""),
    })
}

/// The quote of a suggestion proposed by the AI, `None` for any other reason.
pub fn ai_link_quote(reason: &str) -> Option<&str> {
    body(reason).map(|(quote, _)| quote)
}

/// The model's one-sentence reason of such a suggestion (possibly empty).
pub fn ai_link_why(reason: &str) -> Option<&str> {
    body(reason).map(|(_, why)| why)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_quote_and_the_reason_round_trip_and_stay_apart_from_other_reasons() {
        let stored = ai_link_reason("o core grava\npela outbox", "A decisão rege o \"core\".");
        assert_eq!(ai_link_quote(&stored), Some("o core grava pela outbox"));
        assert_eq!(ai_link_why(&stored), Some("A decisão rege o \"core\"."));
        let bare = ai_link_reason("só a citação", "");
        assert_eq!(ai_link_quote(&bare), Some("só a citação"));
        assert_eq!(ai_link_why(&bare), Some(""));
        assert_eq!(ai_link_quote(&crate::graph::mention_reason("x")), None);
        assert_eq!(ai_link_quote("crates/core/src/lib.rs"), None);
        assert!(crate::graph::mention_quote(&stored).is_none());
    }
}
