//! Telemetry bootstrap and sanitized log formatting.
#![warn(missing_docs)]

use std::fmt;

use tracing_core::field::{Field, Visit};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::EnvFilter;

/// Replacement written in place of a sensitive field value.
const REDACTED: &str = "[redacted]";

/// Field-name fragments that mark a value as sensitive.
const SENSITIVE_FIELD_NEEDLES: &[&str] = &[
    "token",
    "authorization",
    "bearer",
    "auth",
    "secret",
    "password",
    "api_key",
    "apikey",
    "credential",
    "cookie",
    "session",
    "prompt",
    "completion",
    "reasoning",
    "content",
    "body",
    "diff",
    "conversation",
    "message_content",
];

/// Returns `true` if a field with this name must have its value redacted.
fn is_sensitive(field_name: &str) -> bool {
    let lower = field_name.to_ascii_lowercase();
    SENSITIVE_FIELD_NEEDLES
        .iter()
        .any(|needle| lower.contains(needle))
}

/// Case-insensitive authorization scheme whose following token is redacted.
const BEARER_SCHEME: &str = "bearer";

/// Header prefix of a JWT (`{"` in base64url).
const JWT_PREFIX: [char; 3] = ['e', 'y', 'J'];

/// Minimum length for an `sk-` / `sk_` key to be treated as a credential.
const API_KEY_MIN_LEN: usize = 12;

/// Returns `true` when a character belongs to an opaque value token.
fn is_value_char(character: char) -> bool {
    !character.is_whitespace()
        && !matches!(
            character,
            '"' | '\'' | ',' | ';' | ')' | ']' | '}' | '&' | '=' | '?'
        )
}

/// Returns `true` for characters that continue a word on the left edge.
fn is_word_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '-'
}

/// Returns `true` when `position` starts a new word.
fn has_word_boundary(chars: &[char], position: usize) -> bool {
    position == 0 || !is_word_char(chars[position - 1])
}

/// Returns the end of the maximal value-token run that starts at `start`.
fn value_run_end(chars: &[char], start: usize) -> usize {
    let mut end = start;
    while end < chars.len() && is_value_char(chars[end]) {
        end += 1;
    }
    end
}

/// Case-insensitive keyword match at `position`, bounded on the left by a word edge.
fn match_keyword(chars: &[char], position: usize, keyword: &str) -> bool {
    let expected = keyword.as_bytes();
    if position + expected.len() > chars.len() {
        return false;
    }
    if !has_word_boundary(chars, position) {
        return false;
    }
    chars[position..position + expected.len()]
        .iter()
        .zip(expected)
        .all(|(candidate, byte)| candidate.eq_ignore_ascii_case(&char::from(*byte)))
}

/// Returns the end of a JWT at `position`, if one starts there.
fn jwt_end(chars: &[char], position: usize) -> Option<usize> {
    if !chars[position..].starts_with(&JWT_PREFIX) {
        return None;
    }
    if !has_word_boundary(chars, position) {
        return None;
    }
    let end = value_run_end(chars, position);
    let dots = chars[position..end].iter().filter(|c| **c == '.').count();
    (dots >= 2).then_some(end)
}

/// Returns the end of an `sk-` / `sk_` API key at `position`, if one starts there.
fn api_key_end(chars: &[char], position: usize) -> Option<usize> {
    if chars.get(position) != Some(&'s') || chars.get(position + 1) != Some(&'k') {
        return None;
    }
    if !matches!(chars.get(position + 2), Some('-' | '_')) {
        return None;
    }
    if !has_word_boundary(chars, position) {
        return None;
    }
    let end = value_run_end(chars, position);
    (end >= position + API_KEY_MIN_LEN).then_some(end)
}

/// Replaces credential-shaped substrings with [`REDACTED`].
fn redact_value(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut out = String::with_capacity(value.len());
    let mut index = 0;
    while index < chars.len() {
        if match_keyword(&chars, index, BEARER_SCHEME) {
            let scheme_end = index + BEARER_SCHEME.len();
            let mut token_start = scheme_end;
            while token_start < chars.len() && chars[token_start].is_whitespace() {
                token_start += 1;
            }
            if token_start > scheme_end
                && token_start < chars.len()
                && is_value_char(chars[token_start])
            {
                for character in &chars[index..scheme_end] {
                    out.push(*character);
                }
                out.push(' ');
                out.push_str(REDACTED);
                index = value_run_end(&chars, token_start);
                continue;
            }
        }
        if let Some(end) = jwt_end(&chars, index) {
            out.push_str(REDACTED);
            index = end;
            continue;
        }
        if let Some(end) = api_key_end(&chars, index) {
            out.push_str(REDACTED);
            index = end;
            continue;
        }
        out.push(chars[index]);
        index += 1;
    }
    out
}

/// Error returned when telemetry cannot be initialized.
#[derive(Debug)]
pub enum InitError {
    /// `RUST_LOG` was set but contained an invalid filter directive.
    InvalidFilter(String),
}

impl fmt::Display for InitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFilter(detail) => write!(formatter, "invalid RUST_LOG filter: {detail}"),
        }
    }
}

impl std::error::Error for InitError {}

/// Installs the global tracing subscriber.
pub fn init() -> Result<(), InitError> {
    let filter = match std::env::var("RUST_LOG") {
        Ok(raw) => {
            EnvFilter::try_new(&raw).map_err(|err| InitError::InvalidFilter(err.to_string()))?
        }
        Err(_) => EnvFilter::new("info"),
    };

    let subscriber = tracing_subscriber::registry().with(filter).with(
        tracing_subscriber::fmt::layer()
            .event_format(RedactingFormat)
            .with_writer(std::io::stderr),
    );

    let _already_initialized = tracing::subscriber::set_global_default(subscriber).is_err();
    Ok(())
}

/// Event formatter that renders metadata and field values with redaction.
struct RedactingFormat;

impl<S, N> FormatEvent<S, N> for RedactingFormat
where
    S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        _context: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();
        write!(writer, "{} {}: ", metadata.level(), metadata.target())?;
        let mut visitor = RedactingVisitor {
            writer: &mut writer,
            error: Ok(()),
        };
        event.record(&mut visitor);
        visitor.error?;
        writeln!(writer)
    }
}

/// Field visitor that replaces sensitive values with [`REDACTED`].
struct RedactingVisitor<'writer> {
    writer: &'writer mut dyn fmt::Write,
    error: fmt::Result,
}

impl RedactingVisitor<'_> {
    fn write_field(&mut self, field: &Field, value: &dyn fmt::Debug) -> fmt::Result {
        let name = field.name();
        if is_sensitive(name) {
            return write!(self.writer, " {name}={REDACTED}");
        }
        let rendered = format!("{value:?}");
        let sanitized = redact_value(&rendered);
        write!(self.writer, " {name}={sanitized}")
    }
}

impl Visit for RedactingVisitor<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if self.error.is_ok() {
            self.error = self.write_field(field, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{is_sensitive, RedactingFormat};
    use std::io;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;
    use tracing_subscriber::prelude::*;

    /// Shared in-memory sink so a test can inspect exactly what was written.
    #[derive(Clone, Default)]
    struct Buffer(Arc<Mutex<Vec<u8>>>);

    impl Buffer {
        fn contents(&self) -> String {
            let guard = self.0.lock().expect("buffer lock");
            String::from_utf8(guard.clone()).expect("log output must be UTF-8")
        }
    }

    struct BufferWriter<'buffer>(std::sync::MutexGuard<'buffer, Vec<u8>>);

    impl io::Write for BufferWriter<'_> {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'buffer> MakeWriter<'buffer> for Buffer {
        type Writer = BufferWriter<'buffer>;

        fn make_writer(&'buffer self) -> Self::Writer {
            BufferWriter(self.0.lock().expect("buffer lock"))
        }
    }

    /// Runs `emit` against a fresh in-memory subscriber and returns the log.
    fn capture(emit: impl FnOnce()) -> String {
        let buffer = Buffer::default();
        let subscriber = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .event_format(RedactingFormat)
                .with_writer(buffer.clone())
                .with_ansi(false),
        );
        tracing::subscriber::with_default(subscriber, emit);
        buffer.contents()
    }

    #[test]
    fn sensitive_field_values_are_redacted() {
        let output = capture(|| {
            tracing::info!(
                token = "synthetic-token-value",
                password = "synthetic-password-value",
                user = "alice",
                "capture completed"
            );
        });

        println!("redacted log line: {output}");
        assert!(output.contains("token"), "field name missing: {output}");
        assert!(output.contains("password"), "field name missing: {output}");
        assert!(
            output.contains("[redacted]"),
            "redaction marker missing: {output}"
        );
        assert!(
            !output.contains("synthetic-token-value"),
            "token value leaked: {output}"
        );
        assert!(
            !output.contains("synthetic-password-value"),
            "password value leaked: {output}"
        );
        assert!(
            output.contains("alice"),
            "non-sensitive value missing: {output}"
        );
        assert!(
            output.contains("capture completed"),
            "message missing: {output}"
        );
    }

    #[test]
    fn message_field_is_not_sensitive() {
        assert!(!is_sensitive("message"));
        assert!(!is_sensitive("MESSAGE"));
    }

    #[test]
    fn bearer_token_interpolated_into_message_is_redacted() {
        let output = capture(|| {
            tracing::info!("using bearer {}", "sk-abc123secret");
        });
        println!("bearer log line: {output}");
        assert!(
            !output.contains("sk-abc123secret"),
            "interpolated bearer secret leaked: {output}"
        );
        assert!(
            output.contains("[redacted]"),
            "redaction marker missing: {output}"
        );
    }

    #[test]
    fn api_key_shaped_value_is_redacted_without_bearer_prefix() {
        let output = capture(|| {
            tracing::info!("api key {}", "sk_live_abcdef123456");
        });
        assert!(
            !output.contains("sk_live_abcdef123456"),
            "interpolated api key leaked: {output}"
        );
        assert!(
            output.contains("[redacted]"),
            "redaction marker missing: {output}"
        );
    }

    #[test]
    fn jwt_interpolated_into_message_is_redacted() {
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.s3cr3tSignature";
        let output = capture(|| {
            tracing::info!("forwarding {jwt}");
        });
        assert!(
            !output.contains("eyJhbGciOiJIUzI1NiJ9"),
            "jwt header leaked: {output}"
        );
        assert!(
            !output.contains("s3cr3tSignature"),
            "jwt signature leaked: {output}"
        );
        assert!(
            output.contains("[redacted]"),
            "redaction marker missing: {output}"
        );
    }

    #[test]
    fn ordinary_message_is_unchanged() {
        let message = "capture completed /tmp/project 550e8400-e29b-41d4-a716-446655440000";
        let output = capture(|| {
            tracing::info!("{message}");
        });
        assert!(
            output.contains(message),
            "ordinary message altered: {output}"
        );
    }

    #[test]
    fn key_value_jwt_is_redacted() {
        let output = capture(|| {
            tracing::info!("token=eyJaaaaaaaa.bbbbbbbb.cccccccc");
        });
        println!("jwt key=value log line: {output}");
        assert!(!output.contains("eyJaaaaaaaa"), "jwt leaked: {output}");
        assert!(
            output.contains("[redacted]"),
            "redaction marker missing: {output}"
        );
    }

    #[test]
    fn key_value_api_key_is_redacted() {
        let output = capture(|| {
            tracing::info!("api_key=sk-abc123secret");
        });
        assert!(
            !output.contains("sk-abc123secret"),
            "api key leaked: {output}"
        );
        assert!(
            output.contains("[redacted]"),
            "redaction marker missing: {output}"
        );
    }

    #[test]
    fn key_value_authorization_bearer_is_redacted() {
        let output = capture(|| {
            tracing::info!("authorization=Bearer sk-abc123secret");
        });
        println!("authorization log line: {output}");
        assert!(
            !output.contains("sk-abc123secret"),
            "bearer secret leaked: {output}"
        );
        assert!(
            output.contains("[redacted]"),
            "redaction marker missing: {output}"
        );
    }

    #[test]
    fn credential_lookalikes_in_ordinary_text_are_unchanged() {
        let message = "task-sk-thing url=https://example.test/path?q=1";
        let output = capture(|| {
            tracing::info!("{message}");
        });
        assert!(
            output.contains(message),
            "ordinary message altered: {output}"
        );
    }

    #[test]
    fn url_path_segment_sk_docs_is_unchanged() {
        let message = "see https://example.test/sk-docs";
        let output = capture(|| {
            tracing::info!("{message}");
        });
        assert!(
            output.contains(message),
            "url path segment was mangled: {output}"
        );
    }

    #[test]
    fn query_jwt_is_redacted_and_suffix_preserved() {
        let output = capture(|| {
            tracing::info!("?token=eyJaaaaaaaa.bbbbbbbb.cccccccc&x=1");
        });
        println!("query jwt log line: {output}");
        assert!(!output.contains("eyJaaaaaaaa"), "jwt leaked: {output}");
        assert!(output.contains("&x=1"), "query suffix was eaten: {output}");
    }

    #[test]
    fn bearer_standard_base64_is_fully_redacted() {
        let output = capture(|| {
            tracing::info!("Bearer dGVzdC8xMjM0NTY3");
        });
        assert!(
            !output.contains("dGVzdC8xMjM0NTY3"),
            "base64 token leaked: {output}"
        );

        let with_slash = capture(|| {
            tracing::info!("Bearer ab/cd+ef1234");
        });
        assert!(
            !with_slash.contains("cd+ef1234"),
            "slash segment leaked: {with_slash}"
        );
        assert!(
            with_slash.contains("Bearer [redacted]"),
            "slash token not fully redacted: {with_slash}"
        );
    }
}
