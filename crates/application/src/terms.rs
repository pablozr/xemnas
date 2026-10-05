//! Words of a task and of a record brought to a common form, so a task in
//! English finds a decision written in Portuguese and a plural finds its
//! singular. No model: accent folding, a conservative removal of inflection
//! endings in both languages and a bilingual glossary of general software
//! vocabulary.
//!
//! The glossary is general on purpose (written from common software terms,
//! not from the evaluation corpus): it maps an English concept to its usual
//! Portuguese words and back. A wrong or missing pair only costs recall or
//! precision on that word; the context quality gate measures the effect.

use std::collections::BTreeSet;
use std::sync::OnceLock;

/// Lowercase without accents, as the FTS5 `unicode61` tokenizer does.
pub(crate) fn fold(word: &str) -> String {
    word.chars()
        .flat_map(char::to_lowercase)
        .map(|character| match character {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

/// A folded word without its usual inflection ending, so both sides of a
/// comparison meet: plurals in both languages, English `-ing`/`-ed`, then a
/// final `e` (`cache`/`caching`/`caches` all become `cach`, `cores` and
/// `cor` both `cor`). Short words and stems stay as they are.
pub(crate) fn base(word: &str) -> String {
    let word = fold(word);
    if word.chars().count() <= 3 {
        return word;
    }
    let strip = |suffix: &str, replacement: &str, min_stem: usize| -> Option<String> {
        word.strip_suffix(suffix)
            .filter(|stem| stem.chars().count() >= min_stem)
            .map(|stem| format!("{stem}{replacement}"))
    };
    let stem = strip("coes", "cao", 2)
        .or_else(|| strip("oes", "ao", 2))
        .or_else(|| strip("aes", "ao", 2))
        .or_else(|| strip("ais", "al", 2))
        .or_else(|| strip("eis", "el", 2))
        .or_else(|| strip("ies", "y", 3))
        .or_else(|| strip("ing", "", 4))
        .or_else(|| strip("ed", "", 4))
        .or_else(|| (!word.ends_with("ss")).then(|| strip("s", "", 3)).flatten())
        .unwrap_or_else(|| word.clone());
    // Portuguese verb endings (`confirmem`, `confirmar`, `confirmado`), which
    // also bring English verbs closer (`confirm`).
    let stem = VERB_ENDINGS
        .iter()
        .find_map(|ending| {
            stem.strip_suffix(ending)
                .filter(|rest| rest.chars().count() >= 4)
                .map(str::to_owned)
        })
        .unwrap_or(stem);
    match stem.strip_suffix('e') {
        Some(rest) if rest.chars().count() >= 3 => rest.to_owned(),
        _ => stem,
    }
}

const VERB_ENDINGS: &[&str] = &[
    "ando", "endo", "indo", "ado", "ido", "ar", "er", "ir", "em", "am",
];

/// English concept, Portuguese words. Each side is in base form.
const GLOSSARY: &[(&str, &[&str])] = &[
    ("api", &["api"]),
    ("app", &["app", "aplicativo", "aplicacao"]),
    ("application", &["aplicacao", "aplicativo"]),
    ("attempt", &["tentativa"]),
    ("authentication", &["autenticacao"]),
    ("authorization", &["autorizacao"]),
    ("backup", &["backup", "copia"]),
    ("branch", &["branch", "ramo"]),
    ("budget", &["orcamento"]),
    ("bug", &["bug", "defeito", "erro"]),
    ("build", &["build", "compilacao", "compilar"]),
    ("cache", &["cache"]),
    ("caching", &["cache"]),
    ("call", &["chamada", "chamar"]),
    ("candidate", &["candidato"]),
    ("capture", &["captura", "capturar"]),
    (
        "change",
        &["mudanca", "mudar", "alteracao", "alterar", "trocar"],
    ),
    ("check", &["verificacao", "verificar", "checagem"]),
    ("client", &["cliente"]),
    ("code", &["codigo"]),
    ("color", &["cor"]),
    ("column", &["coluna"]),
    ("commit", &["commit"]),
    ("component", &["componente"]),
    ("config", &["configuracao"]),
    ("configuration", &["configuracao"]),
    ("confirm", &["confirmar", "confirmacao"]),
    ("connection", &["conexao"]),
    ("context", &["contexto"]),
    ("continuous", &["continua", "continuo"]),
    ("credential", &["credencial"]),
    ("data", &["dado"]),
    ("database", &["banco"]),
    ("date", &["data"]),
    ("decision", &["decisao"]),
    ("delete", &["apagar", "excluir", "remover"]),
    ("deliver", &["entregar", "entrega"]),
    ("delivery", &["entrega"]),
    ("dependency", &["dependencia"]),
    ("deploy", &["deploy", "implantacao", "publicar"]),
    ("design", &["design", "desenho"]),
    ("directory", &["diretorio", "pasta"]),
    ("distribute", &["distribuir"]),
    ("document", &["documento"]),
    ("documentation", &["documentacao"]),
    ("draw", &["desenhar"]),
    ("duplicate", &["duplicado", "duplicar", "repetido"]),
    ("edit", &["editar", "edicao"]),
    ("encryption", &["criptografia"]),
    ("error", &["erro"]),
    ("event", &["evento"]),
    ("export", &["exportar", "exportacao"]),
    ("fail", &["falhar", "falha"]),
    ("failure", &["falha"]),
    ("feature", &["funcionalidade", "recurso"]),
    ("field", &["campo"]),
    ("file", &["arquivo"]),
    ("filter", &["filtro", "filtrar"]),
    ("folder", &["pasta"]),
    ("font", &["fonte"]),
    ("format", &["formato", "formatar"]),
    ("graph", &["grafo"]),
    ("heading", &["titulo"]),
    ("history", &["historico"]),
    ("hook", &["hook", "gancho"]),
    ("import", &["importar", "importacao"]),
    ("index", &["indice"]),
    ("install", &["instalar", "instalacao"]),
    ("installer", &["instalador"]),
    ("integration", &["integracao"]),
    ("interface", &["interface"]),
    ("item", &["item"]),
    ("job", &["job", "tarefa"]),
    ("keep", &["manter", "guardar"]),
    ("key", &["chave"]),
    ("keychain", &["cofre"]),
    ("language", &["idioma", "linguagem"]),
    ("latency", &["latencia"]),
    ("layer", &["camada"]),
    ("leak", &["vazar", "vazamento"]),
    ("limit", &["limite", "limitar"]),
    ("link", &["vinculo", "ligacao", "link"]),
    ("list", &["lista", "listar"]),
    ("load", &["carregar", "carga"]),
    ("local", &["local"]),
    ("lock", &["trava", "travar", "bloqueio"]),
    ("log", &["log", "registro", "registrar"]),
    ("memory", &["memoria"]),
    ("merge", &["merge", "mesclar", "juntar"]),
    ("message", &["mensagem"]),
    ("migration", &["migracao"]),
    ("model", &["modelo"]),
    ("module", &["modulo"]),
    ("network", &["rede"]),
    ("notification", &["notificacao", "aviso"]),
    ("observation", &["observacao"]),
    ("package", &["pacote", "empacotar"]),
    ("page", &["pagina"]),
    ("paginate", &["paginar", "paginacao"]),
    ("pagination", &["paginacao"]),
    ("password", &["senha"]),
    ("path", &["caminho"]),
    ("performance", &["desempenho", "performance"]),
    ("permission", &["permissao"]),
    ("persistence", &["persistencia"]),
    ("pipeline", &["pipeline"]),
    ("plugin", &["plugin"]),
    ("policy", &["politica"]),
    ("priority", &["prioridade", "priorizar"]),
    ("project", &["projeto"]),
    ("provider", &["provedor"]),
    ("query", &["consulta"]),
    ("queue", &["fila"]),
    ("rate", &["taxa"]),
    ("read", &["ler", "leitura"]),
    ("rebuild", &["recalcular", "reconstruir", "refazer"]),
    ("release", &["release", "versao", "lancamento"]),
    ("render", &["renderizar", "desenhar"]),
    ("request", &["requisicao", "pedido"]),
    ("response", &["resposta"]),
    ("retry", &["reenviar", "repetir", "tentar"]),
    ("review", &["revisao", "revisar"]),
    ("rule", &["regra"]),
    ("run", &["rodar", "executar"]),
    ("schema", &["esquema"]),
    ("screen", &["tela"]),
    ("search", &["busca", "buscar", "pesquisa"]),
    ("secret", &["segredo"]),
    ("security", &["seguranca"]),
    ("send", &["enviar", "envio"]),
    ("server", &["servidor"]),
    ("session", &["sessao"]),
    ("setting", &["ajuste", "configuracao"]),
    ("store", &["guardar", "salvar", "armazenar"]),
    ("storage", &["armazenamento", "persistencia"]),
    ("sync", &["sincronizar", "sincronizacao"]),
    ("table", &["tabela"]),
    ("test", &["teste", "testar"]),
    ("theme", &["tema"]),
    ("thread", &["thread"]),
    ("time", &["tempo", "hora"]),
    ("timeout", &["timeout", "tempo"]),
    ("title", &["titulo"]),
    ("token", &["token"]),
    ("transaction", &["transacao"]),
    ("update", &["atualizar", "atualizacao"]),
    ("upgrade", &["atualizar", "atualizacao"]),
    ("user", &["usuario"]),
    ("validation", &["validacao"]),
    ("value", &["valor"]),
    ("version", &["versao"]),
    ("window", &["janela"]),
    ("write", &["escrever", "gravar"]),
];

fn glossary() -> &'static [(String, Vec<String>)] {
    static BASES: OnceLock<Vec<(String, Vec<String>)>> = OnceLock::new();
    BASES.get_or_init(|| {
        GLOSSARY
            .iter()
            .map(|(english, portuguese)| {
                (base(english), portuguese.iter().map(|w| base(w)).collect())
            })
            .collect()
    })
}

/// The base form of a word and every word a glossary entry pairs it with,
/// in either language (`salvar` meets `store` and also `guardar`).
pub(crate) fn variants(word: &str) -> BTreeSet<String> {
    let root = base(word);
    let mut out = BTreeSet::from([root.clone()]);
    for (english, portuguese) in glossary() {
        if *english == root || portuguese.contains(&root) {
            out.insert(english.clone());
            out.extend(portuguese.iter().cloned());
        }
    }
    out
}

/// What a task asks about: one concept per distinct meaningful word, each
/// with the forms that count as speaking of it.
pub(crate) struct TaskTerms {
    concepts: Vec<BTreeSet<String>>,
}

impl TaskTerms {
    pub(crate) fn new(words: &BTreeSet<String>) -> Self {
        let mut seen = BTreeSet::new();
        let concepts = words
            .iter()
            .filter(|word| seen.insert(base(word)))
            .map(|word| variants(word))
            .collect();
        Self { concepts }
    }

    pub(crate) fn len(&self) -> usize {
        self.concepts.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.concepts.is_empty()
    }

    /// How many of the task's concepts a text's (folded) words speak of.
    pub(crate) fn covered_by(&self, words: &BTreeSet<String>) -> usize {
        let bases: BTreeSet<String> = words.iter().map(|word| base(word)).collect();
        self.concepts
            .iter()
            .filter(|forms| !forms.is_disjoint(&bases))
            .count()
    }

    /// FTS5 `MATCH` for any form of any concept: a prefix query for stems
    /// long enough to be specific (the index keeps words as written, so
    /// `decis*` finds `decisão` and `decisões`), exact words with their
    /// plurals for short ones.
    pub(crate) fn match_query(&self) -> Option<String> {
        let mut terms = BTreeSet::new();
        for form in self.concepts.iter().flatten() {
            let stem = form.strip_suffix("ao").unwrap_or(form);
            if stem.chars().count() >= 4 {
                terms.insert(format!("\"{stem}\"*"));
            } else {
                for word in [form.clone(), format!("{form}s"), format!("{form}es")] {
                    terms.insert(format!("\"{word}\""));
                }
            }
        }
        (!terms.is_empty()).then(|| terms.into_iter().collect::<Vec<_>>().join(" OR "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inflections_meet_their_base() {
        assert_eq!(base("Datas"), base("data"));
        assert_eq!(base("listas"), "lista");
        assert_eq!(base("configurações"), "configuracao");
        assert_eq!(base("decisões"), "decisao");
        assert_eq!(base("caching"), base("cache"));
        assert_eq!(base("caches"), base("cache"));
        assert_eq!(base("retries"), "retry");
        assert_eq!(base("cores"), base("cor"));
        assert_eq!(base("features"), base("feature"));
        assert_eq!(base("níveis"), "nivel");
        assert_eq!(base("api"), "api");
        assert_eq!(base("class"), "class");
        assert_eq!(base("string"), "string");
        assert_eq!(base("tokens"), base("token"));
        assert_eq!(base("confirmem"), base("confirmar"));
        assert_eq!(base("confirming"), base("confirmado"));
    }

    #[test]
    fn english_and_portuguese_meet_through_the_glossary() {
        assert!(variants("passwords").contains("senha"));
        assert!(variants("senhas").contains("password"));
        assert!(variants("database").contains("banco"));
        assert!(variants("Lista").contains("list"));
        assert!(variants("salvar").contains(&base("guardar")));
        assert!(!variants("terminal").contains("cor"));
    }

    #[test]
    fn a_task_counts_concepts_not_words() {
        let words = ["caching", "cache", "passwords"].map(String::from).into();
        let terms = TaskTerms::new(&words);
        assert_eq!(terms.len(), 2);
        let text = ["senhas", "caches"].map(String::from).into();
        assert_eq!(terms.covered_by(&text), 2);
    }

    #[test]
    fn the_index_is_asked_for_stems_and_short_plurals() {
        let words = ["decision", "cor"].map(String::from).into();
        let query = TaskTerms::new(&words).match_query().unwrap();
        assert!(query.contains("\"decis\"*"), "{query}");
        assert!(query.contains("\"cores\""), "{query}");
    }
}
