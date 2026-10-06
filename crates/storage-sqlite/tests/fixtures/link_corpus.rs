//! Synthetic corpus for the AI link proposer (05/10/2026): 6 components of a
//! small TypeScript monorepo that reviews an agent's turn, and 20 decisions
//! taken from ADRs, specs and conversations (no file touched), each labeled
//! with the components it governs. Eight are negatives that share words with a
//! component (its name, the tool it runs on, its topic) without governing its
//! behavior or code. Not human judgments from a real project; the labels are
//! the author's. The live generator in `ai-provider/tests` includes this file
//! too, so it only needs `application`.

/// A component of the map: (name, path patterns, aliases, description).
pub type Component = (
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
    &'static str,
);

/// A decision: (key, question, choice, rationale, components it governs).
pub type Decision = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static [&'static str],
);

pub const COMPONENTS: &[Component] = &[
    (
        "core",
        &["packages/core/**"],
        &["@acme/core"],
        "Engine that weighs the evidence of a turn and returns its outcome.",
    ),
    (
        "opencode-adapter",
        &["packages/opencode-adapter/**"],
        &["@acme/opencode-adapter"],
        "Translates OpenCode session events into the core's input.",
    ),
    (
        "testkit",
        &["packages/testkit/**"],
        &["@acme/testkit"],
        "Fake runtimes and fixtures shared by the tests of every package.",
    ),
    (
        "plugin",
        &["packages/plugin/**"],
        &["@acme/guard"],
        "OpenCode plugin that reviews a completed agent turn and reports the outcome.",
    ),
    (
        "cli",
        &["packages/cli/**"],
        &["acme"],
        "Command line to run a review by hand and print the report.",
    ),
    (
        "storage",
        &["packages/storage/**"],
        &[],
        "Local persistence of evidence and outcomes in SQLite.",
    ),
];

pub const DECISIONS: &[Decision] = &[
    // Positives.
    (
        "outcome-policy",
        "Como o resultado de um turno é expresso?",
        "O core devolve sempre um de três desfechos: aprovado, reprovado ou inconclusivo.",
        "Silêncio nunca pode ser lido como aprovação.",
        &["core"],
    ),
    (
        "evidence-weight",
        "Como o core pesa evidências de fontes diferentes?",
        "Evidência executada pesa mais que evidência declarada pelo agente.",
        "Testes que rodaram são fatos; o texto do agente é afirmação.",
        &["core"],
    ),
    (
        "opencode-version",
        "Qual versão do OpenCode o adaptador suporta?",
        "O adaptador suporta apenas a linha 1.x e recusa eventos de versões desconhecidas.",
        "O formato dos eventos mudou entre linhas maiores.",
        &["opencode-adapter"],
    ),
    (
        "event-mapping",
        "Como os eventos de sessão viram entrada do core?",
        "Cada mensagem final do agente vira exatamente uma entrada de revisão.",
        "Evita revisar o mesmo turno duas vezes.",
        &["opencode-adapter", "core"],
    ),
    (
        "fake-runtime",
        "Como testar sem um OpenCode de verdade?",
        "Os testes usam o runtime falso do testkit, nunca um processo real.",
        "Testes ficam rápidos e determinísticos.",
        &["testkit"],
    ),
    (
        "plugin-report",
        "Onde o plugin mostra o desfecho ao usuário?",
        "O plugin escreve o desfecho como mensagem na própria sessão do OpenCode.",
        "O usuário já está olhando para a sessão.",
        &["plugin"],
    ),
    (
        "plugin-failure",
        "O que o plugin faz quando a revisão falha?",
        "O plugin nunca bloqueia o agente: em erro, registra e deixa o turno seguir.",
        "Uma falha do revisor não pode travar o trabalho.",
        &["plugin"],
    ),
    (
        "cli-exit-code",
        "Qual código de saída o comando de revisão usa?",
        "O comando sai com 0 para aprovado, 1 para reprovado e 2 para inconclusivo.",
        "Scripts de CI dependem do código de saída.",
        &["cli"],
    ),
    (
        "storage-retention",
        "Por quanto tempo as evidências ficam guardadas?",
        "As evidências ficam no SQLite local por 30 dias e depois são apagadas.",
        "Evidências podem conter trechos de código.",
        &["storage"],
    ),
    (
        "redaction",
        "Como tratar segredos nas evidências?",
        "Segredos são redigidos antes de qualquer evidência chegar ao storage.",
        "Nunca gravar um token em disco.",
        &["storage", "core"],
    ),
    (
        "report-format",
        "Qual o formato do relatório de revisão?",
        "O relatório é JSON versionado, com o campo schema_version em todo documento.",
        "Plugin e CLI leem o mesmo documento.",
        &["core", "plugin", "cli"],
    ),
    (
        "fixture-layout",
        "Como organizar as fixtures de teste?",
        "Cada fixture do testkit é um diretório com input.json e expected.json.",
        "Facilita adicionar casos sem tocar em código.",
        &["testkit"],
    ),
    // Negatives: shared vocabulary, not governed.
    (
        "npm-scope",
        "Sob qual escopo publicar os pacotes no npm?",
        "Publicar o plugin, o core e a CLI sob o escopo @acme.",
        "O nome da organização já está reservado.",
        &[],
    ),
    (
        "docs-hosting",
        "Onde hospedar a documentação do plugin?",
        "Em um site estático no GitHub Pages.",
        "Sem custo e sem servidor.",
        &[],
    ),
    (
        "review-meeting",
        "Com que frequência revisar o core em reunião?",
        "Uma vez por mês, em reunião curta de arquitetura.",
        "Mantém a equipe alinhada.",
        &[],
    ),
    (
        "testkit-owners",
        "Quem aprova mudanças no testkit?",
        "Dois mantenedores precisam aprovar.",
        "Uma mudança no testkit quebra todos os pacotes.",
        &[],
    ),
    (
        "opencode-announce",
        "Como avisar a equipe sobre uma versão nova do OpenCode?",
        "Postar no canal da equipe com o link das notas de versão.",
        "Todos acompanham o canal.",
        &[],
    ),
    (
        "sqlite-license",
        "Qual licença usar para o repositório?",
        "MIT, como a do SQLite que usamos no storage.",
        "Licença permissiva e conhecida.",
        &[],
    ),
    (
        "cli-name",
        "Qual o nome do binário da linha de comando?",
        "acme, curto e fácil de digitar.",
        "Evita conflito com outros binários.",
        &[],
    ),
    (
        "turn-glossary",
        "Como chamar a unidade de trabalho do agente na documentação?",
        "Chamar de turno, nunca de rodada nem de iteração.",
        "Um só termo evita confusão em issues e relatórios.",
        &[],
    ),
];

/// The components as map entities, ids `e-<name>`.
pub fn entities() -> Vec<application::graph::EntityRecord> {
    COMPONENTS
        .iter()
        .map(
            |(name, patterns, aliases, description)| application::graph::EntityRecord {
                entity_id: format!("e-{name}"),
                project_id: "corpus".into(),
                kind: application::graph::EntityKind::Component,
                name: (*name).into(),
                key: name.to_lowercase(),
                description: (*description).into(),
                patterns: patterns.iter().map(|pattern| (*pattern).into()).collect(),
                aliases: aliases.iter().map(|alias| (*alias).into()).collect(),
                created_at: "2026-01-01T00:00:00Z".into(),
                retired_at: None,
            },
        )
        .collect()
}

/// The decision as the proposer sees it.
pub fn subject(decision: &Decision) -> application::link_suggestions::LinkSubject {
    application::link_suggestions::LinkSubject {
        question: decision.1.into(),
        choice: decision.2.into(),
        rationale: decision.3.into(),
        scope: Vec::new(),
    }
}
