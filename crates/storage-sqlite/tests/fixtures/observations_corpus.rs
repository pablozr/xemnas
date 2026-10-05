//! Reserved holdout oracles, not an executed test or a precision benchmark.
//! The 16 scenarios mix selection and lifecycle cases; count them separately.
//! No graph, artifact authority, installed version, or normative claim is seeded implicitly.

use application::observations::{DependencyCategory, ManifestKind, ObservationAuthority};

pub struct ManifestSeed {
    pub project: &'static str,
    pub path: &'static str,
    pub kind: ManifestKind,
    pub content: &'static str,
}

pub struct ExpectedFact {
    pub source_path: &'static str,
    pub subject: Subject,
    pub name: &'static str,
    pub category: Option<DependencyCategory>,
    pub declared_version: Option<&'static str>,
    pub requirement: Option<&'static str>,
    pub target: Option<&'static str>,
    pub authority: ObservationAuthority,
}

pub enum Subject {
    Package,
    Dependency,
}

pub enum Action {
    Refresh,
    ReplaceManifest {
        path: &'static str,
        content: &'static str,
    },
    RemoveManifest {
        path: &'static str,
    },
    MakeUnreadable {
        path: &'static str,
    },
    Deliver {
        session: &'static str,
        mode: DeliveryMode,
    },
    QueryProject {
        project: &'static str,
    },
    QueryAsOf {
        timestamp: &'static str,
    },
    QueryCurrent,
    SeedNorm {
        statement: &'static str,
    },
    SetBudget {
        chars: usize,
    },
    ChangeParserPolicy {
        version: &'static str,
    },
    /// Independent subcase, not cumulative with other quota subcases.
    QuotaProbe {
        dimension: QuotaDimension,
        at_limit: usize,
        over_limit: usize,
    },
}

pub enum DeliveryMode {
    Inject,
    Shadow,
}

pub enum QuotaDimension {
    Sources,
    SourceBytes,
    RefreshBytes,
    Observations,
}

pub enum Oracle {
    /// Facts below are selected with source provenance, descriptive authority only.
    SelectFacts,
    /// No descriptive selection; does not assert dependency absence in the project.
    NoSelection,
    /// Same stable ID, new version, old version invalidated; requirement is still not installed.
    ReplaceVersion,
    /// Verified removal invalidates old facts, including the removed declaration.
    InvalidateRemoved,
    /// Failed read does not invalidate old facts or establish absence; coverage records failure.
    UnreadableIsNotAbsence,
    /// Each independent boundary admits at_limit, bounds over_limit and reports partial coverage.
    QuotasAreBounded,
    /// Correct original delivered versions once per session/project/mode, not globally.
    ScopedCorrections,
    /// No facts or corrections leak into the fresh project.
    IsolatedProject,
    /// Historical checkpoint omits facts and reports unknown coverage; subsequent current
    /// checkpoint retains the seeded norm with the same normative budget/selection as without
    /// observations. Descriptive additions must not displace it or count as normative text.
    HistoricalThenNormativePriority,
    /// Unchanged hash/policy creates no new version; changed policy rechecks despite same bytes.
    CacheAndPolicy,
}

pub struct Scenario {
    pub id: &'static str,
    pub manifests: &'static [ManifestSeed],
    pub query: &'static str,
    pub files: &'static [&'static str],
    pub actions: &'static [Action],
    /// Required selected facts for SelectFacts/ReplaceVersion/CacheAndPolicy. Empty means
    /// exact no-selection only for NoSelection/IsolatedProject and the historical checkpoint;
    /// other lifecycle oracles specify their own checkpoints, not a selection denominator.
    pub expected_facts: &'static [ExpectedFact],
    /// Semantic statements that must not be asserted; not raw substring bans on quoted input.
    pub forbidden_assertions: &'static [&'static str],
    pub oracle: Oracle,
}

const CARGO: &str = "[package]\nname = \"servico\"\nversion = \"0.1.0\"\n\
    [dependencies]\nserde = \"1\"\n";
const CARGO_CHANGED: &str = "[package]\nname = \"servico\"\nversion = \"0.1.0\"\n\
    [dependencies]\nserde = \"2\"\n";
const CARGO_REMOVED: &str = "[package]\nname = \"servico\"\nversion = \"0.1.0\"\n";
const NPM: &str = r#"{"name":"painel","version":"0.2.0","dependencies":{"zod":"^3.23"}}"#;
const CATEGORIES: &str = "[package]\nname = \"servico\"\nversion = \"0.1.0\"\n\
    [dev-dependencies]\ninsta = \"1\"\n[build-dependencies]\ncc = \"1\"\n\
    [target.'cfg(windows)'.dependencies]\nwindows-sys = { version = \"0.59\", optional = true }\n";
const NPM_OPTIONAL: &str = r#"{"name":"painel","version":"0.2.0","devDependencies":{"vitest":"^2"},"optionalDependencies":{"fsevents":"^2"}}"#;

const CARGO_SEED: &[ManifestSeed] = &[ManifestSeed {
    project: "projeto-a",
    path: "servico/Cargo.toml",
    kind: ManifestKind::Cargo,
    content: CARGO,
}];
const NPM_SEED: &[ManifestSeed] = &[ManifestSeed {
    project: "projeto-a",
    path: "painel/package.json",
    kind: ManifestKind::PackageJson,
    content: NPM,
}];
const CATEGORY_SEEDS: &[ManifestSeed] = &[
    ManifestSeed {
        project: "projeto-a",
        path: "servico/Cargo.toml",
        kind: ManifestKind::Cargo,
        content: CATEGORIES,
    },
    ManifestSeed {
        project: "projeto-a",
        path: "painel/package.json",
        kind: ManifestKind::PackageJson,
        content: NPM_OPTIONAL,
    },
];
const FORBIDDEN: &[&str] = &[
    "O usuário confirmou uma regra para usar esta dependência.",
    "A dependência está instalada nesta versão.",
    "O manifesto prova uma decisão arquitetural.",
];
const SERDE: ExpectedFact = ExpectedFact {
    source_path: "servico/Cargo.toml",
    subject: Subject::Dependency,
    name: "serde",
    category: Some(DependencyCategory::Runtime),
    declared_version: None,
    requirement: Some("1"),
    target: None,
    authority: ObservationAuthority::Descriptive,
};

/// Data only. A future harness must expand quota probes and lifecycle checkpoints explicitly.
pub const SCENARIOS: &[Scenario] = &[
    Scenario {
        id: "cargo_declarations",
        manifests: CARGO_SEED,
        query: "Qual pacote e dependências são declarados em servico/Cargo.toml?",
        files: &[],
        actions: &[Action::Refresh],
        expected_facts: &[
            ExpectedFact {
                source_path: "servico/Cargo.toml",
                subject: Subject::Package,
                name: "servico",
                category: None,
                declared_version: Some("0.1.0"),
                requirement: None,
                target: None,
                authority: ObservationAuthority::Descriptive,
            },
            SERDE,
        ],
        forbidden_assertions: FORBIDDEN,
        oracle: Oracle::SelectFacts,
    },
    Scenario {
        id: "npm_declarations",
        manifests: NPM_SEED,
        query: "Qual pacote e dependências são declarados em painel/package.json?",
        files: &[],
        actions: &[Action::Refresh],
        expected_facts: &[
            ExpectedFact {
                source_path: "painel/package.json",
                subject: Subject::Package,
                name: "painel",
                category: None,
                declared_version: Some("0.2.0"),
                requirement: None,
                target: None,
                authority: ObservationAuthority::Descriptive,
            },
            ExpectedFact {
                source_path: "painel/package.json",
                subject: Subject::Dependency,
                name: "zod",
                category: Some(DependencyCategory::Runtime),
                declared_version: None,
                requirement: Some("^3.23"),
                target: None,
                authority: ObservationAuthority::Descriptive,
            },
        ],
        forbidden_assertions: FORBIDDEN,
        oracle: Oracle::SelectFacts,
    },
    Scenario {
        id: "categories_and_target",
        manifests: CATEGORY_SEEDS,
        query:
            "Quais dependências de desenvolvimento, build e opcionais estes manifestos declaram?",
        files: &["servico/Cargo.toml", "painel/package.json"],
        actions: &[Action::Refresh],
        expected_facts: &[
            ExpectedFact {
                source_path: "servico/Cargo.toml",
                subject: Subject::Dependency,
                name: "insta",
                category: Some(DependencyCategory::Development),
                declared_version: None,
                requirement: Some("1"),
                target: None,
                authority: ObservationAuthority::Descriptive,
            },
            ExpectedFact {
                source_path: "servico/Cargo.toml",
                subject: Subject::Dependency,
                name: "cc",
                category: Some(DependencyCategory::Build),
                declared_version: None,
                requirement: Some("1"),
                target: None,
                authority: ObservationAuthority::Descriptive,
            },
            ExpectedFact {
                source_path: "servico/Cargo.toml",
                subject: Subject::Dependency,
                name: "windows-sys",
                category: Some(DependencyCategory::Optional),
                declared_version: None,
                requirement: Some("0.59"),
                target: Some("cfg(windows)"),
                authority: ObservationAuthority::Descriptive,
            },
            ExpectedFact {
                source_path: "painel/package.json",
                subject: Subject::Dependency,
                name: "vitest",
                category: Some(DependencyCategory::Development),
                declared_version: None,
                requirement: Some("^2"),
                target: None,
                authority: ObservationAuthority::Descriptive,
            },
            ExpectedFact {
                source_path: "painel/package.json",
                subject: Subject::Dependency,
                name: "fsevents",
                category: Some(DependencyCategory::Optional),
                declared_version: None,
                requirement: Some("^2"),
                target: None,
                authority: ObservationAuthority::Descriptive,
            },
        ],
        forbidden_assertions: FORBIDDEN,
        oracle: Oracle::SelectFacts,
    },
    Scenario {
        id: "file_without_graph",
        manifests: CARGO_SEED,
        query: "",
        files: &["servico/src/lib.rs"],
        actions: &[Action::Refresh],
        expected_facts: &[SERDE],
        forbidden_assertions: FORBIDDEN,
        oracle: Oracle::SelectFacts,
    },
    Scenario {
        id: "explicit_named_dependency",
        manifests: CARGO_SEED,
        query: "Qual requisito de serde foi declarado?",
        files: &[],
        actions: &[Action::Refresh],
        expected_facts: &[SERDE],
        forbidden_assertions: FORBIDDEN,
        oracle: Oracle::SelectFacts,
    },
    Scenario {
        id: "negative_unrelated_task",
        manifests: CARGO_SEED,
        query: "Ajustar o espaçamento do botão de salvar",
        files: &[],
        actions: &[Action::Refresh],
        expected_facts: &[],
        forbidden_assertions: FORBIDDEN,
        oracle: Oracle::NoSelection,
    },
    Scenario {
        id: "negative_absent_dependency",
        manifests: CARGO_SEED,
        query: "Qual requisito de tokio foi declarado?",
        files: &[],
        actions: &[Action::Refresh],
        expected_facts: &[],
        forbidden_assertions: FORBIDDEN,
        oracle: Oracle::NoSelection,
    },
    Scenario {
        id: "negative_unrelated_file",
        manifests: CARGO_SEED,
        query: "",
        files: &["manual/introducao.md"],
        actions: &[Action::Refresh],
        expected_facts: &[],
        forbidden_assertions: FORBIDDEN,
        oracle: Oracle::NoSelection,
    },
    Scenario {
        id: "changed_requirement",
        manifests: CARGO_SEED,
        query: "Qual requisito de serde foi declarado?",
        files: &[],
        actions: &[
            Action::Refresh,
            Action::ReplaceManifest {
                path: "servico/Cargo.toml",
                content: CARGO_CHANGED,
            },
            Action::Refresh,
        ],
        expected_facts: &[ExpectedFact {
            requirement: Some("2"),
            ..SERDE
        }],
        forbidden_assertions: FORBIDDEN,
        oracle: Oracle::ReplaceVersion,
    },
    Scenario {
        id: "removed_declaration_and_source",
        manifests: CARGO_SEED,
        query: "Qual requisito de serde foi declarado?",
        files: &[],
        actions: &[
            Action::Refresh,
            Action::ReplaceManifest {
                path: "servico/Cargo.toml",
                content: CARGO_REMOVED,
            },
            Action::Refresh,
            Action::RemoveManifest {
                path: "servico/Cargo.toml",
            },
            Action::Refresh,
        ],
        expected_facts: &[],
        forbidden_assertions: FORBIDDEN,
        oracle: Oracle::InvalidateRemoved,
    },
    Scenario {
        id: "unreadable_source",
        manifests: CARGO_SEED,
        query: "Qual requisito de serde foi declarado?",
        files: &[],
        actions: &[
            Action::Refresh,
            Action::MakeUnreadable {
                path: "servico/Cargo.toml",
            },
            Action::Refresh,
        ],
        expected_facts: &[],
        forbidden_assertions: &["A falha de leitura prova que serde foi removido."],
        oracle: Oracle::UnreadableIsNotAbsence,
    },
    Scenario {
        id: "quota_boundaries",
        manifests: CARGO_SEED,
        query: "Quais dependências são declaradas?",
        files: &[],
        actions: &[
            Action::QuotaProbe {
                dimension: QuotaDimension::Sources,
                at_limit: 64,
                over_limit: 65,
            },
            Action::QuotaProbe {
                dimension: QuotaDimension::SourceBytes,
                at_limit: 256 * 1024,
                over_limit: 256 * 1024 + 1,
            },
            Action::QuotaProbe {
                dimension: QuotaDimension::RefreshBytes,
                at_limit: 2 * 1024 * 1024,
                over_limit: 2 * 1024 * 1024 + 1,
            },
            Action::QuotaProbe {
                dimension: QuotaDimension::Observations,
                at_limit: 1024,
                over_limit: 1025,
            },
        ],
        expected_facts: &[],
        forbidden_assertions: &["A coleta acima da quota é completa."],
        oracle: Oracle::QuotasAreBounded,
    },
    Scenario {
        id: "session_correction",
        manifests: CARGO_SEED,
        query: "Qual requisito de serde foi declarado?",
        files: &[],
        actions: &[
            Action::Refresh,
            Action::Deliver {
                session: "sessao-a",
                mode: DeliveryMode::Inject,
            },
            Action::Deliver {
                session: "sessao-a",
                mode: DeliveryMode::Shadow,
            },
            Action::Deliver {
                session: "sessao-b",
                mode: DeliveryMode::Inject,
            },
            Action::ReplaceManifest {
                path: "servico/Cargo.toml",
                content: CARGO_REMOVED,
            },
            Action::Refresh,
            Action::Deliver {
                session: "sessao-a",
                mode: DeliveryMode::Inject,
            },
            Action::Deliver {
                session: "sessao-a",
                mode: DeliveryMode::Inject,
            },
            Action::Deliver {
                session: "sessao-a",
                mode: DeliveryMode::Shadow,
            },
            Action::Deliver {
                session: "sessao-b",
                mode: DeliveryMode::Inject,
            },
            Action::Deliver {
                session: "sessao-nova",
                mode: DeliveryMode::Inject,
            },
        ],
        expected_facts: &[],
        forbidden_assertions: &["Uma sessão nova recebeu a versão original invalidada."],
        oracle: Oracle::ScopedCorrections,
    },
    Scenario {
        id: "fresh_project_isolation",
        manifests: CARGO_SEED,
        query: "Qual requisito de serde foi declarado?",
        files: &[],
        actions: &[
            Action::Refresh,
            Action::QueryProject {
                project: "projeto-novo",
            },
        ],
        expected_facts: &[],
        forbidden_assertions: FORBIDDEN,
        oracle: Oracle::IsolatedProject,
    },
    Scenario {
        id: "historical_and_normative_priority",
        manifests: CARGO_SEED,
        query: "Qual requisito de serde foi declarado?",
        files: &[],
        actions: &[
            Action::Refresh,
            Action::QueryAsOf {
                timestamp: "2000-01-01T00:00:00Z",
            },
            Action::SeedNorm {
                statement: "Não enviar dados pessoais a serviços externos.",
            },
            Action::SetBudget { chars: 500 },
            Action::QueryCurrent,
        ],
        expected_facts: &[],
        forbidden_assertions: &["O manifesto atual prova o requisito em 2000."],
        oracle: Oracle::HistoricalThenNormativePriority,
    },
    Scenario {
        id: "unchanged_cache_and_policy",
        manifests: CARGO_SEED,
        query: "Qual requisito de serde foi declarado?",
        files: &[],
        actions: &[
            Action::Refresh,
            Action::Refresh,
            Action::ChangeParserPolicy {
                version: "politica-v2",
            },
            Action::Refresh,
        ],
        expected_facts: &[SERDE],
        forbidden_assertions: FORBIDDEN,
        oracle: Oracle::CacheAndPolicy,
    },
];
