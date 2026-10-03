//! The writing style every text the AI produces for people follows: 80% of
//! ASD-STE100 (Simplified Technical English), the controlled language made
//! for aircraft manuals. Short, direct sentences are read faster and
//! misread less, which matters when the reader must decide what to confirm.
//!
//! The rules shorten and clarify; they never remove or add content. The
//! guard rails (keep every fact and every hedge, add nothing, leave code and
//! quotes alone) are part of the text, so a clearer sentence cannot become a
//! more convincing but wrong one.

/// The style rules, as a literal for prompts (`concat!`-compatible).
#[macro_export]
macro_rules! plain_rules {
    () => {
        "\nStyle of every sentence you write for people (80% of ASD-STE100, Simplified \
Technical English): one idea per sentence, up to 20 words; active voice with the subject \
first; one word for one meaning, and the same word every time for the same thing; no \
figurative language, no idioms, no stacked clauses; the result first, then the reason; a \
verb for each action instead of a noun phrase. Guard rails: keep every fact and every \
qualification or doubt (\"may\", \"probably\", \"unknown\"); add no fact; never change \
code, identifiers, names, ids or text quoted from the records, which stay verbatim."
    };
}

/// The same rules for text that is not a prompt (tests and docs).
pub const PLAIN_RULES: &str = plain_rules!();

#[cfg(test)]
mod tests {
    use super::PLAIN_RULES;

    #[test]
    fn the_rules_keep_their_guard_rails() {
        for needle in ["ASD-STE100", "keep every fact", "add no fact", "verbatim"] {
            assert!(PLAIN_RULES.contains(needle), "{needle}");
        }
    }
}
