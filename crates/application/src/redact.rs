//! Secret redaction at the ingest boundary (MVP-SPEC §13, PRIV-001).

use serde_json::Value;

/// Replacement written in place of a secret, identical to the adapter's.
pub const REDACTED: &str = "[REDACTED]";

/// Credential names whose assignment value is masked (compared case-insensitively
/// after removing `_` and `-`).
const ASSIGNMENT_NAMES: &[&str] = &[
    "token",
    "apikey",
    "secret",
    "password",
    "accesskey",
    "authorization",
];

/// A prefixed key shape: prefix, minimum run length after it, allowed run chars.
type KeyShape = (&'static str, usize, fn(char) -> bool);

/// Prefixed key shapes masked at a word boundary.
const KEY_SHAPES: &[KeyShape] = &[
    ("github_pat_", 20, |c| c.is_ascii_alphanumeric() || c == '_'),
    ("ghp_", 20, |c| c.is_ascii_alphanumeric()),
    ("sk-", 8, |c| {
        c.is_ascii_alphanumeric() || c == '_' || c == '-'
    }),
    ("xoxb-", 10, |c| c.is_ascii_alphanumeric() || c == '-'),
    ("xoxa-", 10, |c| c.is_ascii_alphanumeric() || c == '-'),
    ("xoxp-", 10, |c| c.is_ascii_alphanumeric() || c == '-'),
    ("xoxr-", 10, |c| c.is_ascii_alphanumeric() || c == '-'),
    ("xoxs-", 10, |c| c.is_ascii_alphanumeric() || c == '-'),
];

/// Masks common secret shapes in `text`.
pub fn redact_secrets(text: &str) -> String {
    let text = redact_private_keys(text);
    let text = redact_assignments(&text);
    redact_key_shapes(&text)
}

/// Redacts every string inside a JSON value, recursively. Keys are kept.
pub fn redact_json(value: &Value) -> Value {
    match value {
        Value::String(text) => Value::String(redact_secrets(text)),
        Value::Array(items) => Value::Array(items.iter().map(redact_json).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, item)| (key.clone(), redact_json(item)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Replaces `-----BEGIN … PRIVATE KEY----- … -----END … PRIVATE KEY-----`.
fn redact_private_keys(text: &str) -> String {
    const BEGIN: &str = "-----BEGIN ";
    const END: &str = "-----END ";
    const MARKER: &str = "PRIVATE KEY-----";

    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(BEGIN) {
        let header = &rest[start + BEGIN.len()..];
        let Some(header_end) = header.find("-----") else {
            break;
        };
        let label = &header[..header_end];
        let is_private_key = header[..header_end + 5].ends_with(MARKER)
            && label
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == ' ');
        if !is_private_key {
            output.push_str(&rest[..start + BEGIN.len()]);
            rest = header;
            continue;
        }
        output.push_str(&rest[..start]);
        output.push_str(REDACTED);
        let body = &header[header_end + 5..];
        rest = match find_block_end(body, END, MARKER) {
            Some(end) => &body[end..],
            None => "",
        };
    }
    output.push_str(rest);
    output
}

/// Returns the byte index right after the first `-----END … PRIVATE KEY-----`.
fn find_block_end(body: &str, end: &str, marker: &str) -> Option<usize> {
    let mut offset = 0;
    while let Some(found) = body[offset..].find(end) {
        let after = offset + found + end.len();
        let tail = &body[after..];
        if let Some(close) = tail.find("-----") {
            let label = &tail[..close];
            if tail[..close + 5].ends_with(marker)
                && label
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == ' ')
            {
                return Some(after + close + 5);
            }
        }
        offset = after;
    }
    None
}

/// Masks the value of credential assignment lines, keeping the name and the
/// separator: `API_KEY = abc` becomes `API_KEY = [REDACTED]`.
fn redact_assignments(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            output.push('\n');
        }
        match assignment_value_start(line) {
            Some(start) => {
                output.push_str(&line[..start]);
                output.push_str(REDACTED);
                if line.ends_with('\r') {
                    output.push('\r');
                }
            }
            None => output.push_str(line),
        }
    }
    output
}

/// Byte index where the masked value starts, when `line` is a credential
/// assignment with a non-empty value.
fn assignment_value_start(line: &str) -> Option<usize> {
    let trimmed_start = line.len() - line.trim_start().len();
    let rest = &line[trimmed_start..];
    let name_len = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .unwrap_or(rest.len());
    let name: String = rest[..name_len]
        .chars()
        .filter(|c| *c != '_' && *c != '-')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if !ASSIGNMENT_NAMES.contains(&name.as_str()) {
        return None;
    }
    let after_name = &rest[name_len..];
    let spaces = after_name.len() - after_name.trim_start().len();
    let separator = after_name[spaces..].chars().next()?;
    if separator != ':' && separator != '=' {
        return None;
    }
    let after_separator = &after_name[spaces + 1..];
    let value_offset = after_separator.len() - after_separator.trim_start().len();
    let value = after_separator[value_offset..].trim_end_matches('\r');
    if value.is_empty() || value == REDACTED {
        return None;
    }
    Some(trimmed_start + name_len + spaces + 1 + value_offset)
}

/// Masks prefixed key shapes that start at a word boundary.
fn redact_key_shapes(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    let bytes = text.as_bytes();
    while index < text.len() {
        let at_boundary = index == 0 || !is_word_byte(bytes[index - 1]);
        if at_boundary {
            if let Some(length) = key_shape_len(&text[index..]) {
                output.push_str(REDACTED);
                index += length;
                continue;
            }
        }
        let Some(character) = text[index..].chars().next() else {
            break;
        };
        output.push(character);
        index += character.len_utf8();
    }
    output
}

/// Length in bytes of a key shape at the start of `text`, if any.
fn key_shape_len(text: &str) -> Option<usize> {
    for (prefix, minimum, allowed) in KEY_SHAPES {
        let Some(rest) = text.strip_prefix(prefix) else {
            continue;
        };
        let mut run = rest
            .char_indices()
            .find(|(_, c)| !allowed(*c))
            .map(|(position, _)| position)
            .unwrap_or(rest.len());
        while run > 0 && rest.as_bytes()[run - 1] == b'-' {
            run -= 1;
        }
        let followed_by_word = rest.as_bytes().get(run).is_some_and(|b| is_word_byte(*b));
        if run >= *minimum && !followed_by_word {
            return Some(prefix.len() + run);
        }
    }
    None
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[cfg(test)]
mod tests {
    use super::{redact_json, redact_secrets, REDACTED};
    use serde_json::json;

    #[test]
    fn masks_private_key_blocks() {
        let text = "antes\n-----BEGIN RSA PRIVATE KEY-----\nMIIEow\nAAAA\n-----END RSA PRIVATE KEY-----\ndepois";
        assert_eq!(redact_secrets(text), format!("antes\n{REDACTED}\ndepois"));
        let open = "x -----BEGIN PRIVATE KEY-----\nMIIE";
        assert_eq!(redact_secrets(open), format!("x {REDACTED}"));
        let public = "-----BEGIN PUBLIC KEY-----\nMIIB\n-----END PUBLIC KEY-----";
        assert_eq!(
            redact_secrets(public),
            public,
            "public keys are not secrets"
        );
    }

    #[test]
    fn masks_assignment_values_and_keeps_names() {
        assert_eq!(
            redact_secrets("API_KEY=abc123\n  password: hunter2\r\nname=ok"),
            format!("API_KEY={REDACTED}\n  password: {REDACTED}\r\nname=ok")
        );
        assert_eq!(
            redact_secrets("Authorization = Bearer abc"),
            format!("Authorization = {REDACTED}")
        );
        assert_eq!(
            redact_secrets("access-key:x"),
            format!("access-key:{REDACTED}")
        );
        assert_eq!(redact_secrets("TOKEN="), "TOKEN=", "empty value is kept");
        assert_eq!(
            redact_secrets("tokens=5"),
            "tokens=5",
            "only the listed names"
        );
        assert_eq!(
            redact_secrets("let token = compute();"),
            "let token = compute();",
            "code that does not start with the name is untouched"
        );
    }

    #[test]
    fn masks_known_key_shapes_at_word_boundaries() {
        assert_eq!(
            redact_secrets("chave sk-abcdEFGH1234 fim"),
            format!("chave {REDACTED} fim")
        );
        assert_eq!(redact_secrets("ghp_abcdefghijklmnopqrstuvwxyz"), REDACTED);
        assert_eq!(
            redact_secrets("github_pat_ABCDEFGHIJKLMNOPQRSTUV_x"),
            REDACTED
        );
        assert_eq!(
            redact_secrets("slack xoxb-1234567890-abc."),
            format!("slack {REDACTED}.")
        );
        assert_eq!(
            redact_secrets("sk-abcdefgh-"),
            format!("{REDACTED}-"),
            "a trailing hyphen is not part of the key"
        );
        assert_eq!(redact_secrets("task-abcdefghij"), "task-abcdefghij");
        assert_eq!(redact_secrets("sk-short"), "sk-short");
        assert_eq!(
            redact_secrets("ghp_short"),
            "ghp_short",
            "below the minimum length"
        );
    }

    #[test]
    fn is_idempotent_and_keeps_unicode() {
        let text = "ação: sk-abcdefgh12345678\nSECRET=valor\n-----BEGIN EC PRIVATE KEY-----\nk\n-----END EC PRIVATE KEY-----";
        let once = redact_secrets(text);
        assert_eq!(redact_secrets(&once), once);
        assert!(once.starts_with("ação: "));
        assert!(!once.contains("sk-abcdefgh"));
        assert!(!once.contains("valor"));
    }

    #[test]
    fn plain_text_is_unchanged() {
        let text = "diff --git a/src/lib.rs b/src/lib.rs\n+fn main() {}\n";
        assert_eq!(redact_secrets(text), text);
    }

    #[test]
    fn redacts_strings_inside_json() {
        let value = json!({
            "file": "config.env",
            "nested": ["sk-abcdefgh12345678", {"note": "API_KEY=xyz"}],
            "count": 3
        });
        assert_eq!(
            redact_json(&value),
            json!({
                "file": "config.env",
                "nested": [REDACTED, {"note": format!("API_KEY={REDACTED}")}],
                "count": 3
            })
        );
    }
}
