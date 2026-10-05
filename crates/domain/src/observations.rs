//! Descriptive manifest facts, never normative claims or graph nodes.

/// Fact established by a manifest parser, not by AI or confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationKind {
    /// A package declared by its manifest.
    PackageDeclared,
    /// A dependency declaration (not an installed or resolved version).
    DependencyDeclared,
}

/// Lifecycle of a descriptive fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationStatus {
    /// Verified against the latest checked source.
    Current,
    /// No longer supported by the checked source.
    Invalidated,
}

/// Outcome of checking one source; failures are not evidence of absence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCheckStatus {
    /// Retired explicit workspace membership; not a filesystem absence.
    OutOfScope,
    /// Read and parsed successfully.
    Verified,
    /// Previously known source no longer exists.
    Missing,
    /// Source could not be read.
    Unreadable,
    /// Source syntax or declaration is unsupported.
    Unsupported,
    /// Reading would exceed the bounded refresh quota.
    QuotaExceeded,
}
