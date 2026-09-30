//! Context Claims: typed, temporal statements about a Project.

use crate::time::Timestamp;

/// Maximum characters in a claim statement.
pub const MAX_STATEMENT_CHARS: usize = 1_000;

/// What a claim asserts about the Project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ClaimKind {
    /// Believed true but not proven.
    Assumption,
    /// A limit the work must respect.
    Constraint,
    /// An outcome the Project pursues.
    Goal,
    /// An agreed way of working.
    Convention,
}

impl ClaimKind {
    /// Every kind, in a stable order.
    pub const ALL: [Self; 4] = [
        Self::Assumption,
        Self::Constraint,
        Self::Goal,
        Self::Convention,
    ];

    /// Persisted literal.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Assumption => "assumption",
            Self::Constraint => "constraint",
            Self::Goal => "goal",
            Self::Convention => "convention",
        }
    }

    /// Parses a persisted literal.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

/// Why a claim is not acceptable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimError {
    /// The statement is blank.
    EmptyStatement,
    /// The statement exceeds [`MAX_STATEMENT_CHARS`].
    StatementTooLong,
    /// A date is not `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM:SSZ`.
    InvalidDate,
    /// The validity ends before it starts.
    InvertedValidity,
    /// The claim had already ended at the requested time.
    AlreadyEnded,
}

impl ClaimError {
    /// Short, stable code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyStatement => "empty_statement",
            Self::StatementTooLong => "statement_too_long",
            Self::InvalidDate => "invalid_date",
            Self::InvertedValidity => "inverted_validity",
            Self::AlreadyEnded => "already_ended",
        }
    }
}

impl std::fmt::Display for ClaimError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::EmptyStatement => "a afirmação não pode ficar vazia",
            Self::StatementTooLong => "a afirmação passa de 1.000 caracteres",
            Self::InvalidDate => "use datas no formato AAAA-MM-DD",
            Self::InvertedValidity => "o fim da validade é anterior ao início",
            Self::AlreadyEnded => "a afirmação já tinha deixado de valer",
        })
    }
}

impl std::error::Error for ClaimError {}

/// A claim valid from `valid_from` (inclusive) until `valid_until` (exclusive).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextClaim {
    kind: ClaimKind,
    statement: String,
    valid_from: Timestamp,
    valid_until: Option<Timestamp>,
}

impl ContextClaim {
    /// Validates and builds a claim; the statement is trimmed.
    pub fn new(
        kind: ClaimKind,
        statement: &str,
        valid_from: &str,
        valid_until: Option<&str>,
    ) -> Result<Self, ClaimError> {
        let statement = statement.trim();
        if statement.is_empty() {
            return Err(ClaimError::EmptyStatement);
        }
        if statement.chars().count() > MAX_STATEMENT_CHARS {
            return Err(ClaimError::StatementTooLong);
        }
        let valid_from = Timestamp::parse(valid_from).ok_or(ClaimError::InvalidDate)?;
        let valid_until = valid_until
            .map(|value| Timestamp::parse(value).ok_or(ClaimError::InvalidDate))
            .transpose()?;
        if valid_until
            .as_ref()
            .is_some_and(|until| *until < valid_from)
        {
            return Err(ClaimError::InvertedValidity);
        }
        Ok(Self {
            kind,
            statement: statement.to_string(),
            valid_from,
            valid_until,
        })
    }

    /// Claim kind.
    pub fn kind(&self) -> ClaimKind {
        self.kind
    }

    /// Trimmed statement.
    pub fn statement(&self) -> &str {
        &self.statement
    }

    /// Start of validity.
    pub fn valid_from(&self) -> &Timestamp {
        &self.valid_from
    }

    /// End of validity, when set.
    pub fn valid_until(&self) -> Option<&Timestamp> {
        self.valid_until.as_ref()
    }

    /// Whether the claim holds at `at`.
    pub fn is_valid_at(&self, at: &Timestamp) -> bool {
        self.valid_from <= *at && self.valid_until.as_ref().is_none_or(|until| at < until)
    }

    /// Ends the claim at `at`, keeping its history.
    pub fn retire(&mut self, at: &str) -> Result<(), ClaimError> {
        let at = Timestamp::parse(at).ok_or(ClaimError::InvalidDate)?;
        if at < self.valid_from {
            return Err(ClaimError::InvertedValidity);
        }
        if self.valid_until.as_ref().is_some_and(|until| *until <= at) {
            return Err(ClaimError::AlreadyEnded);
        }
        self.valid_until = Some(at);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ClaimError, ClaimKind, ContextClaim, MAX_STATEMENT_CHARS};
    use crate::time::Timestamp;

    fn at(value: &str) -> Timestamp {
        Timestamp::parse(value).expect("valid timestamp")
    }

    #[test]
    fn literals_round_trip() {
        for kind in ClaimKind::ALL {
            assert_eq!(ClaimKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(ClaimKind::parse("fact"), None);
    }

    #[test]
    fn validates_statement_and_dates() {
        let claim = |statement: &str, from: &str, until: Option<&str>| {
            ContextClaim::new(ClaimKind::Constraint, statement, from, until)
        };
        assert_eq!(
            claim("  ", "2026-01-01", None),
            Err(ClaimError::EmptyStatement)
        );
        assert_eq!(
            claim(&"x".repeat(MAX_STATEMENT_CHARS + 1), "2026-01-01", None),
            Err(ClaimError::StatementTooLong)
        );
        assert_eq!(claim("ok", "ontem", None), Err(ClaimError::InvalidDate));
        assert_eq!(
            claim("ok", "2026-02-01", Some("2026-01-01")),
            Err(ClaimError::InvertedValidity)
        );
        let valid = claim("  Só Windows  ", "2026-01-01", Some("2026-12-31")).expect("valid");
        assert_eq!(valid.statement(), "Só Windows");
    }

    #[test]
    fn validity_includes_the_start_and_excludes_the_end() {
        let claim = ContextClaim::new(ClaimKind::Goal, "meta", "2026-01-01", Some("2026-02-01"))
            .expect("valid");
        assert!(!claim.is_valid_at(&at("2025-12-31T23:59:59Z")));
        assert!(claim.is_valid_at(&at("2026-01-01")));
        assert!(claim.is_valid_at(&at("2026-01-31T23:59:59Z")));
        assert!(!claim.is_valid_at(&at("2026-02-01")));
        let open = ContextClaim::new(ClaimKind::Goal, "meta", "2026-01-01", None).expect("valid");
        assert!(open.is_valid_at(&at("2099-01-01")));
    }

    #[test]
    fn retire_closes_the_validity_once() {
        let mut claim = ContextClaim::new(ClaimKind::Assumption, "premissa", "2026-01-01", None)
            .expect("valid");
        assert_eq!(
            claim.retire("2025-12-01"),
            Err(ClaimError::InvertedValidity)
        );
        claim.retire("2026-03-01").expect("retire");
        assert_eq!(
            claim.valid_until().map(Timestamp::as_str),
            Some("2026-03-01T00:00:00Z")
        );
        assert!(!claim.is_valid_at(&at("2026-03-01")));
        assert_eq!(claim.retire("2026-04-01"), Err(ClaimError::AlreadyEnded));
    }
}
