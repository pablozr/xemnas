//! Who wrote a message that reached the agent as a user turn.
//!
//! Claude Code and other harnesses deliver more than the user's words as user
//! messages: the end of a background subagent, a slash command and its output,
//! `<system-reminder>` blocks, an interruption notice, what another agent sent.
//! This is the one place that tells them apart, by the shape of the text and
//! never by a list of tags, so the triage, the extractor and the injection
//! agree. The one heuristic is the shape of a harness tag: a standard HTML
//! element never has a `-` or `_` in its name, so a prompt that opens with
//! `<div>`, `<template>` or `<br>` is still the user. No regex, and no
//! allocation unless a wake is unwrapped.

use std::borrow::Cow;

const SYSTEM_REMINDER_OPEN: &str = "<system-reminder>";
const SYSTEM_REMINDER_CLOSE: &str = "</system-reminder>";
const INTERRUPTED_PREFIX: &str = "[Request interrupted by user";
const WAKE_OPEN: &str = "<wake";
const MESSAGE_OPEN: &str = "<message";
const MESSAGE_CLOSE: &str = "</message>";
const FROM_HUMAN: &str = "from=\"human\"";

/// Longest tag name a harness block opens with.
const MAX_TAG_NAME: usize = 64;

/// Openings of messages another agent writes (OpenBot hand-offs and replays).
/// Fixed texts, listed in `docs/arquitetura/qualidade-do-nucleo.md`.
const AGENT_PREFIXES: &[&str] = &[
    "Message from OpenBot teammate",
    "You have one assignment in a shared OpenBot channel",
    "Continue this OpenBot conversation.",
    "<openbot_",
];

/// The author of a user-role message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Speaker<'a> {
    /// The user. Carries their words: without the leading reminders and, for a
    /// wake, the human text taken out of the envelope.
    User(Cow<'a, str>),
    /// The harness: notification, command echo, reminder, interruption.
    Harness,
    /// Another agent: a teammate's message, an assignment, a wake without a human.
    Agent,
}

/// Classifies `text` by its shape.
///
/// 1. Nothing left after the leading reminders, or an interruption notice: harness.
/// 2. A fixed agent opening: agent.
/// 3. `<wake ...>`: the human message inside it is the user's; without one, agent.
/// 4. An opening tag with a harness-shaped name (`<command-name>`,
///    `<task-notification>`: no attributes, lowercase, with a `-` or `_`):
///    harness. A standard element (`<div>`, `<br>`), a tag with attributes
///    (`<pasted_content id=...>`) or a long name is prose the user pasted.
/// 5. Anything else: the user.
pub fn classify(text: &str) -> Speaker<'_> {
    let rest = strip_leading_reminders(text);
    if rest.is_empty() || rest.starts_with(INTERRUPTED_PREFIX) {
        return Speaker::Harness;
    }
    if opens_agent(rest) {
        return Speaker::Agent;
    }
    if is_wake(rest) {
        return human_of_wake(rest)
            .map_or(Speaker::Agent, |human| Speaker::User(Cow::Owned(human)));
    }
    if starts_with_harness_tag(rest) {
        return Speaker::Harness;
    }
    Speaker::User(Cow::Borrowed(rest))
}

/// `prompt` without the `<system-reminder>` blocks at its start; an unclosed
/// block swallows the rest.
fn strip_leading_reminders(prompt: &str) -> &str {
    let mut rest = prompt.trim_start();
    while let Some(after) = rest.strip_prefix(SYSTEM_REMINDER_OPEN) {
        let Some(end) = after.find(SYSTEM_REMINDER_CLOSE) else {
            return "";
        };
        rest = after[end + SYSTEM_REMINDER_CLOSE.len()..].trim_start();
    }
    rest
}

/// `<wake>` or `<wake ...>`, and not a longer name such as `<wakeup>`.
fn is_wake(rest: &str) -> bool {
    rest.strip_prefix(WAKE_OPEN)
        .is_some_and(|after| after.starts_with(['>', ' ', '\n', '\t']))
}

/// The text of the `<message ... from="human" ...>` blocks of a wake, joined by
/// a line break; `None` when there is none or the wake is malformed.
fn human_of_wake(wake: &str) -> Option<String> {
    let mut humans: Vec<&str> = Vec::new();
    let mut rest = wake;
    while let Some(start) = rest.find(MESSAGE_OPEN) {
        let after = &rest[start + MESSAGE_OPEN.len()..];
        let open_end = after.find('>')?;
        let attributes = &after[..open_end];
        let body = &after[open_end + 1..];
        let close = body.find(MESSAGE_CLOSE)?;
        if attributes.starts_with([' ', '\n', '\t']) && attributes.contains(FROM_HUMAN) {
            let text = body[..close].trim();
            if !text.is_empty() {
                humans.push(text);
            }
        }
        rest = &body[close + MESSAGE_CLOSE.len()..];
    }
    (!humans.is_empty()).then(|| humans.join("\n"))
}

/// Whether `rest` opens with `<name>`, a name that looks like the harness's:
/// ascii lowercase starting with a letter, only `a-z`, digits, `_` and `-`, at
/// least one `-` or `_` (a standard HTML element never has one), at most
/// [`MAX_TAG_NAME`], closed directly by `>`. One pass, no allocation.
///
/// Accepted risk: a custom element or XML tag with a separator and no
/// attributes at the very start (`<my-card>`, `<log_entry>`) counts as harness.
fn starts_with_harness_tag(rest: &str) -> bool {
    let Some(after) = rest.strip_prefix('<') else {
        return false;
    };
    let bytes = after.as_bytes();
    if !bytes.first().is_some_and(u8::is_ascii_lowercase) {
        return false;
    }
    let mut separators = 0usize;
    let mut name = 0usize;
    for &byte in bytes.iter().take(MAX_TAG_NAME + 1) {
        match byte {
            b'a'..=b'z' | b'0'..=b'9' => {}
            b'_' | b'-' => separators += 1,
            _ => break,
        }
        name += 1;
    }
    (1..=MAX_TAG_NAME).contains(&name) && separators > 0 && bytes.get(name) == Some(&b'>')
}

/// Whether the text opens, after the reminders, with a harness-shaped tag.
/// The Harness verdict of [`classify`] then came from the shape of the tag (a
/// heuristic), not from a fact such as an interruption or a fixed agent opening.
pub fn opens_with_harness_tag(text: &str) -> bool {
    let rest = strip_leading_reminders(text);
    starts_with_harness_tag(rest) && !opens_agent(rest)
}

fn opens_agent(rest: &str) -> bool {
    AGENT_PREFIXES.iter().any(|prefix| rest.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_user(text: &str) -> bool {
        matches!(classify(text), Speaker::User(_))
    }

    #[test]
    fn a_harness_shaped_tag_is_the_harness() {
        for text in [
            "  <task-notification><status>done</status></task-notification>",
            "<command-name>/compact</command-name>",
            "<local-command-stdout>ok</local-command-stdout>",
            "<local-command-caveat>x</local-command-caveat>",
            "<user-prompt-submit-hook>x</user-prompt-submit-hook>",
            "<bash-input>ls</bash-input>",
            "<bash-stdout>ok</bash-stdout>",
            // Accepted risk: a custom element with a separator looks like the harness.
            "<foo-bar>",
        ] {
            assert_eq!(classify(text), Speaker::Harness, "{text}");
        }
    }

    #[test]
    fn a_common_tag_opening_is_the_user() {
        for text in [
            "<div>oi</div> por que quebra?",
            "<template>\n  <p>x</p>\n</template>\nesse componente",
            "<br>",
            "<log><entry>timeout</entry></log>\ntroca o redis",
            "<foo>",
            "<wakeup>x",
            "<Task-Notification>x",
            "<-x>",
            "<1-a>",
            "<?xml version=\"1.0\"?><a/>",
            "<!-- c -->",
        ] {
            assert!(is_user(text), "{text}");
        }
    }

    #[test]
    fn openbot_memories_are_an_agent() {
        for text in [
            "<openbot_saved_memories>m</openbot_saved_memories>\nbrief",
            "<openbot_memories_update>x</openbot_memories_update>",
        ] {
            assert_eq!(classify(text), Speaker::Agent, "{text}");
        }
    }

    #[test]
    fn reminders_are_skipped_and_an_empty_rest_is_the_harness() {
        assert_eq!(
            classify("<system-reminder>x</system-reminder>"),
            Speaker::Harness
        );
        assert_eq!(
            classify(
                "<system-reminder>x</system-reminder>\n<task-notification>y</task-notification>"
            ),
            Speaker::Harness
        );
        assert_eq!(
            classify("<system-reminder>x</system-reminder>\nusa redis"),
            Speaker::User(Cow::Borrowed("usa redis"))
        );
        assert_eq!(classify("<system-reminder>unclosed"), Speaker::Harness);
        assert_eq!(classify("   "), Speaker::Harness);
    }

    #[test]
    fn an_interruption_is_the_harness() {
        assert_eq!(
            classify("[Request interrupted by user for tool use]"),
            Speaker::Harness
        );
    }

    #[test]
    fn what_a_person_writes_is_the_user() {
        assert_eq!(classify("cache"), Speaker::User(Cow::Borrowed("cache")));
        for text in [
            "sim pode",
            "veja <task-notification>",
            "se a < b usa x",
            "<pasted_content id=\"1\">log</pasted_content>",
            "<a href=\"x\">link</a>",
            "<>",
            "</closing>",
        ] {
            assert!(is_user(text), "{text}");
        }
    }

    #[test]
    fn a_long_tag_name_is_prose() {
        let name = format!("a-{}", "a".repeat(MAX_TAG_NAME - 2));
        assert_eq!(classify(&format!("<{name}>")), Speaker::Harness);
        let name = format!("a-{}", "a".repeat(MAX_TAG_NAME - 1));
        assert!(is_user(&format!("<{name}>")));
    }

    #[test]
    fn only_the_shape_of_the_tag_is_a_heuristic_verdict() {
        assert!(opens_with_harness_tag(
            "<system-reminder>r</system-reminder>\n<bash-stdout>x</bash-stdout>"
        ));
        for text in [
            "[Request interrupted by user for tool use]",
            "Message from OpenBot teammate Agente A: oi",
            "<openbot_saved_memories>m</openbot_saved_memories>",
            "<div>",
        ] {
            assert!(!opens_with_harness_tag(text), "{text}");
        }
    }

    #[test]
    fn other_agents_are_recognized_by_their_fixed_openings() {
        for text in [
            "Message from OpenBot teammate Agente A: segue o resultado",
            "You have one assignment in a shared OpenBot channel.\n[{\"author\":{\"kind\":\"member\"}}]",
            "Continue this OpenBot conversation.\nhistory",
        ] {
            assert_eq!(classify(text), Speaker::Agent, "{text}");
        }
    }

    #[test]
    fn a_wake_with_a_human_message_carries_the_human_words() {
        let wake =
            "<wake reason=\"message\">\n<message id=\"00000000-0000-0000-0000-000000000001\" \
                    from=\"human\" at=\"2026-01-01\">usa sqlite local</message>\n</wake>";
        assert_eq!(
            classify(wake),
            Speaker::User(Cow::Owned("usa sqlite local".to_string()))
        );
    }

    #[test]
    fn a_wake_without_a_human_or_malformed_is_an_agent() {
        for wake in [
            "<wake reason=\"timer\"><message from=\"agent\">oi</message></wake>",
            "<wake reason=\"timer\">nothing</wake>",
            "<wake><message from=\"human\">sem fechar",
            "<wake><message from=\"human\" ",
            "<wake><message from=\"human\">   </message></wake>",
        ] {
            assert_eq!(classify(wake), Speaker::Agent, "{wake}");
        }
        assert!(is_user("<wakeup>x"));
    }

    #[test]
    fn a_wake_human_text_that_opens_with_a_harness_tag_stays_the_users() {
        let wake = "<wake><message from=\"human\"><task-notification>x</task-notification>\
                    </message></wake>";
        assert_eq!(
            classify(wake),
            Speaker::User(Cow::Owned(
                "<task-notification>x</task-notification>".to_string()
            ))
        );
    }
}
