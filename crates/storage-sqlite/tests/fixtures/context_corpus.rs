//! Frozen synthetic corpus v2: not human judgments, no calibration or providers.
//! Each family has PT, EN and distractor-worded queries. Holdout is family-wide.

pub struct Family {
    pub name: &'static str,
    pub positive: bool,
    pub holdout: bool,
    pub queries: [&'static str; 3],
    pub required: &'static [&'static str],
    pub partial: &'static [&'static str],
    pub files: &'static [&'static str],
}

pub const STANDING: &[&str] = &["standing"];
pub const FORBIDDEN: &[&str] = &["foreign", "expired", "superseded"];

pub const FAMILIES: &[Family] = &[
    Family {
        name: "terminal-color",
        positive: false,
        holdout: false,
        queries: [
            "Qual cor usar no terminal?",
            "Choose the terminal color",
            "Escolher cor do terminal para visualizar contadores de Override",
        ],
        required: &[],
        partial: &[],
        files: &[],
    },
    Family {
        name: "override",
        positive: true,
        holdout: false,
        queries: [
            "Preservar contadores locais ao aplicar Override na avaliação",
            "Preserve local counters when applying an evaluation override",
            "Preservar contadores de Override na avaliação exibida no terminal",
        ],
        required: &["override"],
        partial: &[],
        files: &[],
    },
    Family {
        name: "cache",
        positive: true,
        holdout: false,
        queries: [
            "Implementar cache da API",
            "Implement API response caching",
            "Implementar cache da API que o cliente de terminal consulta",
        ],
        required: &["cache", "rate"],
        partial: &[],
        files: &[],
    },
    Family {
        name: "file-storage",
        positive: true,
        holdout: true,
        queries: [
            "Ajustar persistência neste arquivo",
            "Edit persistence in this file",
            "Ajustar persistência neste arquivo usado pelo terminal",
        ],
        required: &["storage"],
        partial: &[],
        files: &["crates/storage/src/db.rs"],
    },
    Family {
        name: "polarity",
        positive: true,
        holdout: true,
        queries: [
            "Não registrar senhas nos logs",
            "Never log passwords",
            "Impedir senhas nos logs exibidos no terminal",
        ],
        required: &["password"],
        partial: &["logging"],
        files: &[],
    },
    Family {
        name: "database",
        positive: true,
        holdout: false,
        queries: [
            "Escolher banco para persistência local",
            "Choose a database for local persistence",
            "Escolher banco de persistência local usado pelo cliente de terminal",
        ],
        required: &["storage"],
        partial: &[],
        files: &[],
    },
    Family {
        name: "cooking",
        positive: false,
        holdout: false,
        queries: [
            "Preparar receita de pão",
            "Bake sourdough bread",
            "Preparar receita de pão lendo instruções no terminal",
        ],
        required: &[],
        partial: &[],
        files: &[],
    },
    Family {
        name: "foreign-project",
        positive: false,
        holdout: false,
        queries: [
            "Configurar sonar submarino",
            "Configure submarine sonar",
            "Configurar sonar submarino pelo terminal",
        ],
        required: &[],
        partial: &[],
        files: &[],
    },
    Family {
        name: "expired-policy",
        positive: false,
        holdout: true,
        queries: [
            "Suportar fax legado",
            "Support legacy fax",
            "Suportar fax legado acionado pelo terminal",
        ],
        required: &[],
        partial: &[],
        files: &[],
    },
    Family {
        name: "unlinked-file",
        positive: false,
        holdout: true,
        queries: [
            "Editar paisagem lunar",
            "Edit lunar landscape",
            "Editar paisagem lunar com ferramenta de terminal",
        ],
        required: &[],
        partial: &[],
        files: &["art/moon.svg"],
    },
];
