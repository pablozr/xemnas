//! Explicit knowledge qualifications; absence is never inferred from rationale.

use serde::{Deserialize, Serialize};

/// Maximum qualifications attached to one knowledge item.
pub const MAX_QUALIFIERS: usize = 16;
/// Maximum Unicode characters in one qualification.
pub const MAX_QUALIFIER_CHARS: usize = 2_000;

/// What a qualification restricts or attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualifierKind {
    /// Who stated the knowledge or which source it describes.
    Attribution,
    /// Where or when the knowledge applies.
    Scope,
    /// What has or has not been verified.
    Validation,
}

/// An explicit qualification, independent of an optionally shortened rationale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeQualifier {
    /// Qualification category.
    pub kind: QualifierKind,
    /// Literal source excerpt, or a human reviewer's declaration.
    pub text: String,
    /// Supporting artifact; absent means a reviewer declaration, not verified support.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
}

/// Checks bounds without shortening qualifications or manufacturing support.
pub fn validate_qualifiers(items: &[KnowledgeQualifier]) -> Result<(), String> {
    if items.len() > MAX_QUALIFIERS {
        return Err("qualificadores demais".into());
    }
    for item in items {
        if item.text.trim().is_empty() || item.text.chars().count() > MAX_QUALIFIER_CHARS {
            return Err("texto de qualificador vazio ou longo demais".into());
        }
        if item
            .artifact_id
            .as_ref()
            .is_some_and(|id| id.trim().is_empty() || id.len() > 256)
        {
            return Err("referência de qualificador inválida".into());
        }
    }
    Ok(())
}

/// Checks extracted qualifications against literal artifact content.
/// Human declarations without an artifact are intentionally rejected here.
pub fn validate_extracted(
    items: &[KnowledgeQualifier],
    evidence: &crate::extract::DecisionEvidence,
) -> Result<(), String> {
    validate_qualifiers(items)?;
    for item in items {
        let supported = item.artifact_id.as_ref().is_some_and(|id| {
            evidence.artifacts.iter().any(|artifact| {
                &artifact.artifact_id == id && artifact.content.contains(&item.text)
            })
        });
        if !supported {
            return Err("qualificador sem citação literal verificável".into());
        }
    }
    Ok(())
}

/// Keeps the extracted qualifications that `validate_extracted` would accept
/// and drops the rest, so one misquoted excerpt costs that qualification and
/// not the whole decision. An artifact reference that merely contains a real
/// artifact id (`artifact <id>`) is canonicalized to the id; the text must
/// still be a literal excerpt of that artifact.
pub fn retain_supported(
    items: Vec<KnowledgeQualifier>,
    evidence: &crate::extract::DecisionEvidence,
) -> Vec<KnowledgeQualifier> {
    items
        .into_iter()
        .take(MAX_QUALIFIERS)
        .filter_map(|mut item| {
            let id = item.artifact_id.as_deref()?.trim();
            let artifact = evidence
                .artifacts
                .iter()
                .find(|artifact| artifact.artifact_id == id)
                .or_else(|| {
                    evidence
                        .artifacts
                        .iter()
                        .find(|artifact| id.contains(&artifact.artifact_id))
                })?;
            item.artifact_id = Some(artifact.artifact_id.clone());
            let kept = validate_qualifiers(std::slice::from_ref(&item)).is_ok()
                && artifact.content.contains(&item.text);
            kept.then_some(item)
        })
        .collect()
}

/// Decodes persisted qualifications; malformed data is an error, not an empty list.
pub fn decode(text: &str) -> Result<Vec<KnowledgeQualifier>, String> {
    let items: Vec<KnowledgeQualifier> =
        serde_json::from_str(text).map_err(|_| "qualificadores inválidos".to_string())?;
    validate_qualifiers(&items)?;
    Ok(items)
}

/// An indivisible context admission, including every qualification and its version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedAdmission {
    /// Full item text when admitted; absent when the complete item does not fit.
    pub text: Option<String>,
    /// Whether the budget omitted the complete item.
    pub omitted: bool,
}

/// Admits choice, qualifications, scope and reference/version atomically.
pub fn admit(
    reference: &str,
    version: i64,
    choice: &str,
    scope: &[String],
    qualifiers: &[KnowledgeQualifier],
    remaining_chars: usize,
) -> Result<QualifiedAdmission, String> {
    validate_qualifiers(qualifiers)?;
    let text = serde_json::json!({
        "reference": reference, "version": version, "choice": choice,
        "scope": scope, "qualifiers": qualifiers,
    })
    .to_string();
    if text.chars().count() > remaining_chars {
        Ok(QualifiedAdmission {
            text: None,
            omitted: true,
        })
    } else {
        Ok(QualifiedAdmission {
            text: Some(text),
            omitted: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retain_supported_keeps_only_literal_excerpts_within_bounds() {
        let evidence = crate::extract::DecisionEvidence {
            capture_id: "c".into(),
            project_id: "p".into(),
            adapter: None,
            session_id: None,
            observed_at: None,
            artifacts: vec![crate::extract::EvidenceArtifact {
                artifact_id: "a1".into(),
                kind: "document".into(),
                content: "uses SQLite only".into(),
                metadata: "{}".into(),
            }],
        };
        let item = |text: &str, id: &str| KnowledgeQualifier {
            kind: QualifierKind::Scope,
            text: text.into(),
            artifact_id: Some(id.into()),
        };
        let kept = retain_supported(
            vec![
                item("SQLite only", "a1"),
                item("sqlite only", "a1"),
                item("", "a1"),
                item("SQLite only", "a2"),
            ],
            &evidence,
        );
        assert_eq!(kept, vec![item("SQLite only", "a1")]);
        // Whatever survives passes the strict check.
        assert!(validate_extracted(&kept, &evidence).is_ok());
    }

    #[test]
    fn unicode_bounds_and_total_omission() {
        let qualifier = KnowledgeQualifier {
            kind: QualifierKind::Validation,
            text: "não validado".into(),
            artifact_id: None,
        };
        assert!(validate_qualifiers(std::slice::from_ref(&qualifier)).is_ok());
        let admission = admit("D:1234", 3, "usar", &[], &[qualifier], 0).unwrap();
        assert!(admission.omitted);
        assert!(admission.text.is_none());
        assert!(decode("[]").unwrap().is_empty());
        assert!(decode("invalid").is_err());
    }
}
