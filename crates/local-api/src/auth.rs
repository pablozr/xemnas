//! Bearer token generation and constant-time comparison.
//!
//! The token is random per session and never logged (PRIV-001).

/// Generates a fresh session token from 32 random bytes, hex encoded.
///
/// The RNG is `getrandom`, already present in the workspace lock; a failure to
/// read entropy is returned rather than panicking (RUST-001).
pub fn generate_token() -> std::io::Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|error| std::io::Error::other(format!("token entropy unavailable: {error}")))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Compares two byte slices without early exit.
///
/// A length mismatch returns `false` immediately; the token length is fixed and
/// public, so this does not leak a secret.
pub fn constant_time_eq(candidate: &[u8], expected: &[u8]) -> bool {
    if candidate.len() != expected.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left, right) in candidate.iter().zip(expected.iter()) {
        difference |= left ^ right;
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::{constant_time_eq, generate_token};

    #[test]
    fn generated_tokens_are_64_hex_chars_and_unique() {
        let first = generate_token().expect("token");
        let second = generate_token().expect("token");
        assert_eq!(first.len(), 64);
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(first, second);
    }

    #[test]
    fn constant_time_eq_matches_only_equal_values() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"secrez"));
        assert!(!constant_time_eq(b"secret", b"secret-longer"));
        assert!(!constant_time_eq(b"", b"x"));
    }
}
