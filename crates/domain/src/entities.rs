//! Project entities and the typed, human-confirmed edges that tie decisions
//! and claims to them (ADR-0005).

/// Longest accepted entity name, alias or description line.
pub const MAX_NAME_CHARS: usize = 80;

/// Longest accepted path pattern.
pub const MAX_PATTERN_CHARS: usize = 200;

/// Longest accepted entity description.
pub const MAX_DESCRIPTION_CHARS: usize = 500;

/// What an entity is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EntityKind {
    /// A part of the project, located by path patterns.
    Component,
    /// A technology the project uses (language, library, service).
    Technology,
}

impl EntityKind {
    /// Every kind, in a stable order.
    pub const ALL: [Self; 2] = [Self::Component, Self::Technology];

    /// Persisted literal.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Component => "component",
            Self::Technology => "technology",
        }
    }

    /// Parses a persisted literal.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

/// What the source of an edge is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeKind {
    /// An Engineering Decision.
    Decision,
    /// A Context Claim.
    Claim,
    /// An entity.
    Entity,
}

impl NodeKind {
    /// Every kind, in a stable order.
    pub const ALL: [Self; 3] = [Self::Decision, Self::Claim, Self::Entity];

    /// Persisted literal.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Claim => "claim",
            Self::Entity => "entity",
        }
    }

    /// Parses a persisted literal.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

/// Kind of edge from a node to an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EdgeKind {
    /// Decision → component it changes.
    Affects,
    /// Decision → technology it adopts or relies on.
    Uses,
    /// Claim → component or technology it constrains.
    AppliesTo,
    /// Component → the component it belongs to.
    PartOf,
}

impl EdgeKind {
    /// Every kind, in a stable order.
    pub const ALL: [Self; 4] = [Self::Affects, Self::Uses, Self::AppliesTo, Self::PartOf];

    /// Persisted literal.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Affects => "affects",
            Self::Uses => "uses",
            Self::AppliesTo => "applies_to",
            Self::PartOf => "part_of",
        }
    }

    /// Parses a persisted literal.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }

    /// Checks the source and target kinds this edge allows.
    pub fn check(&self, source: NodeKind, target: EntityKind) -> Result<(), EntityError> {
        let allowed = match self {
            Self::Affects => source == NodeKind::Decision && target == EntityKind::Component,
            Self::Uses => source == NodeKind::Decision && target == EntityKind::Technology,
            Self::AppliesTo => source == NodeKind::Claim,
            Self::PartOf => source == NodeKind::Entity && target == EntityKind::Component,
        };
        if allowed {
            Ok(())
        } else {
            Err(EntityError::EdgeNotAllowed)
        }
    }
}

/// Who created an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EdgeOrigin {
    /// Created by the user; confirmed at creation.
    Human,
    /// Derived from captured work; a suggestion until confirmed.
    Derived,
}

impl EdgeOrigin {
    /// Persisted literal.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Derived => "derived",
        }
    }

    /// Parses a persisted literal.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "human" => Some(Self::Human),
            "derived" => Some(Self::Derived),
            _ => None,
        }
    }
}

/// Who confirmed or invalidated an edge. An edge a person confirmed is never
/// touched by a derivation; the others are the machine's guesses and can be
/// revised when the evidence changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EdgeActor {
    /// A person, in the app or through a use case they triggered.
    Person,
    /// The automatic review's free rules.
    Rules,
    /// The automatic review's AI judge.
    Ai,
    /// The map itself: a rule that follows the tie of its source decision.
    Inherited,
}

impl EdgeActor {
    /// Every actor, in a stable order.
    pub const ALL: [Self; 4] = [Self::Person, Self::Rules, Self::Ai, Self::Inherited];

    /// Persisted literal.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Rules => "rules",
            Self::Ai => "ai",
            Self::Inherited => "inherited",
        }
    }

    /// Parses a persisted literal.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|actor| actor.as_str() == value)
    }
}

/// Why an entity, pattern or edge is not acceptable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityError {
    /// The name is blank or has no letter or digit.
    EmptyName,
    /// The name exceeds [`MAX_NAME_CHARS`].
    NameTooLong,
    /// The description exceeds [`MAX_DESCRIPTION_CHARS`].
    DescriptionTooLong,
    /// The pattern is empty, absolute, escapes the project or is too long.
    InvalidPattern,
    /// Only components have path patterns.
    PatternOnTechnology,
    /// The edge does not connect these kinds.
    EdgeNotAllowed,
    /// A component cannot be part of itself.
    SelfReference,
    /// The `part_of` edge would close a cycle.
    Cycle,
    /// The component already belongs to another component.
    SecondParent,
}

impl EntityError {
    /// Short, stable code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyName => "empty_name",
            Self::NameTooLong => "name_too_long",
            Self::DescriptionTooLong => "description_too_long",
            Self::InvalidPattern => "invalid_pattern",
            Self::PatternOnTechnology => "pattern_on_technology",
            Self::EdgeNotAllowed => "edge_not_allowed",
            Self::SelfReference => "self_reference",
            Self::Cycle => "cycle",
            Self::SecondParent => "second_parent",
        }
    }
}

impl std::fmt::Display for EntityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::EmptyName => "o nome precisa ter ao menos uma letra ou dígito",
            Self::NameTooLong => "o nome passa de 80 caracteres",
            Self::DescriptionTooLong => "a descrição passa de 500 caracteres",
            Self::InvalidPattern => {
                "use um caminho relativo ao projeto, como crates/app/**, sem .. nem letra de disco"
            }
            Self::PatternOnTechnology => "só componentes têm padrões de caminho",
            Self::EdgeNotAllowed => "esse vínculo não liga esses tipos",
            Self::SelfReference => "um componente não pode fazer parte de si mesmo",
            Self::Cycle => "o vínculo criaria um ciclo de componentes",
            Self::SecondParent => "o componente já faz parte de outro componente",
        })
    }
}

impl std::error::Error for EntityError {}

/// A trimmed, validated entity name.
pub fn entity_name(name: &str) -> Result<String, EntityError> {
    let name = name.trim();
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(EntityError::NameTooLong);
    }
    if entity_key(name).is_empty() {
        return Err(EntityError::EmptyName);
    }
    Ok(name.to_string())
}

/// A trimmed, validated description (may be empty).
pub fn entity_description(text: &str) -> Result<String, EntityError> {
    let text = text.trim();
    if text.chars().count() > MAX_DESCRIPTION_CHARS {
        return Err(EntityError::DescriptionTooLong);
    }
    Ok(text.to_string())
}

/// Normalized key used to resolve names and aliases: lowercase letters,
/// digits and the signs that change a name's meaning (`+`, `#`), so "SQLite 3"
/// → "sqlite3" and "storage-sqlite" → "storagesqlite", while C, C++ and C#
/// stay three keys.
pub fn entity_key(name: &str) -> String {
    name.chars()
        .filter(|character| character.is_alphanumeric() || matches!(character, '+' | '#'))
        .flat_map(char::to_lowercase)
        .collect()
}

/// A validated path pattern, stored with forward slashes and no leading `./`.
pub fn path_pattern(pattern: &str) -> Result<String, EntityError> {
    let normalized = pattern.trim().replace('\\', "/");
    let normalized = normalized.trim_start_matches("./").to_string();
    if normalized.is_empty()
        || normalized.chars().count() > MAX_PATTERN_CHARS
        || normalized.starts_with('/')
        || normalized.contains(':')
        || normalized.split('/').any(|segment| segment == "..")
    {
        return Err(EntityError::InvalidPattern);
    }
    Ok(normalized)
}

/// Whether a relative `path` matches `pattern` (`*` within a segment, `**`
/// across segments, `?` one character). Case-insensitive, like Windows paths.
/// A pattern without wildcards matches the path itself and everything below it.
pub fn pattern_matches(pattern: &str, path: &str) -> bool {
    let path = normalize_path(path);
    let pattern = pattern.to_lowercase();
    let path = path.to_lowercase();
    if !pattern.contains(['*', '?']) {
        let prefix = pattern.trim_end_matches('/');
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    let pattern: Vec<&str> = pattern.split('/').collect();
    let path: Vec<&str> = path.split('/').collect();
    match_segments(&pattern, &path)
}

/// A path relative to the project, with forward slashes.
pub fn normalize_path(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

fn match_segments(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => (0..=path.len()).any(|skip| match_segments(rest, &path[skip..])),
        Some((segment, rest)) => match path.split_first() {
            Some((head, tail)) => match_segment(segment, head) && match_segments(rest, tail),
            None => false,
        },
    }
}

fn match_segment(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let (mut p, mut t) = (0, 0);
    let (mut star, mut resume) = (None, 0);
    while t < text.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            p += 1;
            resume = t;
        } else if let Some(position) = star {
            p = position + 1;
            resume += 1;
            t = resume;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

/// Container folders whose children are components (`crates/<name>`).
const CONTAINERS: &[&str] = &[
    "crates", "apps", "packages", "adapters", "libs", "services", "modules",
];

/// The component prefix a changed file suggests: `crates/<name>` and similar
/// containers, else the first folder. `None` for files at the project root.
pub fn component_prefix(path: &str) -> Option<String> {
    let path = normalize_path(path);
    let segments: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    match segments.as_slice() {
        [container, name, _, ..] if CONTAINERS.contains(&container.to_lowercase().as_str()) => {
            Some(format!("{container}/{name}"))
        }
        [folder, _, ..] => Some((*folder).to_string()),
        _ => None,
    }
}

/// Checks that adding `child part_of parent` keeps the components a tree:
/// no cycle and one parent per component, given the existing `(child,
/// parent)` pairs. Context is inherited up a single chain, so a second parent
/// would be silently ignored on read.
pub fn check_part_of(
    child: &str,
    parent: &str,
    existing: &[(String, String)],
) -> Result<(), EntityError> {
    if child == parent {
        return Err(EntityError::SelfReference);
    }
    if existing
        .iter()
        .any(|(from, to)| from == child && to != parent)
    {
        return Err(EntityError::SecondParent);
    }
    let mut current = vec![parent.to_string()];
    let mut seen = std::collections::BTreeSet::new();
    while let Some(node) = current.pop() {
        if node == child {
            return Err(EntityError::Cycle);
        }
        if !seen.insert(node.clone()) {
            continue;
        }
        current.extend(
            existing
                .iter()
                .filter(|(from, _)| *from == node)
                .map(|(_, to)| to.clone()),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_ignore_case_spacing_and_punctuation() {
        assert_eq!(entity_key("SQLite 3"), "sqlite3");
        assert_eq!(entity_key("storage-sqlite"), "storagesqlite");
        assert_eq!(entity_key("Serviço de Captura"), "serviçodecaptura");
        // Signs that make different languages stay distinct.
        assert_eq!(entity_key("C"), "c");
        assert_eq!(entity_key("C++"), "c++");
        assert_eq!(entity_key("C#"), "c#");
        assert_eq!(entity_key("F#"), "f#");
        assert_eq!(entity_name("  GPUI  ").as_deref(), Ok("GPUI"));
        assert_eq!(entity_name(" -- "), Err(EntityError::EmptyName));
        assert_eq!(entity_name(&"x".repeat(81)), Err(EntityError::NameTooLong));
    }

    #[test]
    fn patterns_are_relative_and_inside_the_project() {
        assert_eq!(
            path_pattern("./crates/app/**").as_deref(),
            Ok("crates/app/**")
        );
        assert_eq!(
            path_pattern("crates\\app\\*").as_deref(),
            Ok("crates/app/*")
        );
        for bad in ["", "/etc/**", "C:/repo/**", "crates/../../x", "   "] {
            assert_eq!(path_pattern(bad), Err(EntityError::InvalidPattern), "{bad}");
        }
    }

    #[test]
    fn globs_match_like_codeowners() {
        assert!(pattern_matches(
            "crates/storage-sqlite/**",
            "crates/storage-sqlite/src/store.rs"
        ));
        assert!(pattern_matches(
            "crates/storage-sqlite",
            "crates/storage-sqlite/src/store.rs"
        ));
        assert!(pattern_matches(
            "crates/storage-sqlite",
            "crates/storage-sqlite"
        ));
        assert!(!pattern_matches(
            "crates/storage-sqlite",
            "crates/storage-sqlite-extra/x.rs"
        ));
        assert!(pattern_matches(
            "**/*.sql",
            "crates/storage-sqlite/src/migrations/0001.sql"
        ));
        assert!(pattern_matches(
            "apps/*/src/*.rs",
            "apps/desktop-gpui/src/main.rs"
        ));
        assert!(!pattern_matches(
            "apps/*/src/*.rs",
            "apps/desktop-gpui/src/ui/theme.rs"
        ));
        assert!(pattern_matches("docs/adr-00??.md", "docs/adr-0005.md"));
        assert!(pattern_matches("Crates/App/**", "crates\\app\\src\\lib.rs"));
    }

    #[test]
    fn prefixes_follow_workspace_containers() {
        assert_eq!(
            component_prefix("crates/storage-sqlite/src/store.rs").as_deref(),
            Some("crates/storage-sqlite")
        );
        assert_eq!(
            component_prefix("adapters/opencode/src/index.ts").as_deref(),
            Some("adapters/opencode")
        );
        assert_eq!(
            component_prefix("docs/design/x.md").as_deref(),
            Some("docs")
        );
        assert_eq!(component_prefix("README.md"), None);
        assert_eq!(
            component_prefix("crates/Cargo.toml").as_deref(),
            Some("crates")
        );
    }

    #[test]
    fn edges_connect_only_their_kinds() {
        use EntityKind::{Component, Technology};
        assert!(EdgeKind::Affects
            .check(NodeKind::Decision, Component)
            .is_ok());
        assert!(EdgeKind::Affects
            .check(NodeKind::Decision, Technology)
            .is_err());
        assert!(EdgeKind::Uses.check(NodeKind::Decision, Technology).is_ok());
        assert!(EdgeKind::AppliesTo
            .check(NodeKind::Claim, Technology)
            .is_ok());
        assert!(EdgeKind::AppliesTo
            .check(NodeKind::Decision, Component)
            .is_err());
        assert!(EdgeKind::PartOf.check(NodeKind::Entity, Component).is_ok());
        assert!(EdgeKind::PartOf
            .check(NodeKind::Entity, Technology)
            .is_err());
        for kind in EdgeKind::ALL {
            assert_eq!(EdgeKind::parse(kind.as_str()), Some(kind));
        }
    }

    #[test]
    fn part_of_rejects_self_and_cycles() {
        let existing = vec![
            ("b".to_string(), "a".to_string()),
            ("c".to_string(), "b".to_string()),
        ];
        assert_eq!(
            check_part_of("a", "a", &existing),
            Err(EntityError::SelfReference)
        );
        assert_eq!(check_part_of("a", "c", &existing), Err(EntityError::Cycle));
        assert!(check_part_of("d", "c", &existing).is_ok());
    }

    #[test]
    fn a_component_has_one_parent() {
        let existing = vec![("api".to_string(), "backend".to_string())];
        assert_eq!(
            check_part_of("api", "services", &existing),
            Err(EntityError::SecondParent)
        );
        assert_eq!(check_part_of("api", "backend", &existing), Ok(()));
        assert_eq!(check_part_of("web", "backend", &existing), Ok(()));
    }
}
