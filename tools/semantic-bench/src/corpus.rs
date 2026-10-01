use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Document {
    pub id: usize,
    pub text: String,
}

#[derive(Serialize, Deserialize)]
pub struct Query {
    pub text: String,
    pub relevant: Vec<usize>,
}

#[derive(Deserialize)]
pub struct Dataset {
    pub documents: Vec<Document>,
    pub queries: Vec<Query>,
}

// Fixed before running models. Two relevant PT/EN decisions per question,
// two hard negatives sharing vocabulary but proposing a different behavior.
pub fn fixture() -> (Vec<Document>, Vec<Query>) {
    let topics = [
        [
            "Adotamos fila durável SQLite para indexar decisões aprovadas em segundo plano. Jobs persistem após reiniciar, com tentativas e chave idempotente. A aprovação não aguarda embeddings.",
            "Approved decisions enqueue durable SQLite background jobs. Embedding generation retries after restart and uses an idempotency key without blocking approval.",
            "Decidimos gerar embeddings sincronamente durante aprovação. Falhas abortam a aprovação e não há fila persistente.",
            "A fila de notificações fica somente em memória e desaparece ao encerrar. Ela não é usada na indexação das decisões.",
            "Como garantir que o trabalho de indexação sobreviva ao fechamento do aplicativo?",
            "Find the decision that keeps approval responsive while recovering indexing after a crash.",
        ],
        [
            "A busca combina BM25 lexical e similaridade vetorial por Reciprocal Rank Fusion. Isso recupera identificadores exatos e paráfrases sem depender da escala dos scores.",
            "Hybrid retrieval merges lexical BM25 and dense vector rankings using reciprocal rank fusion to handle exact symbols and paraphrases.",
            "Adotamos apenas comparação textual literal para busca. Não usamos vetores nem fusão de rankings.",
            "Somamos diretamente scores BM25 e cosine sem normalização; removemos Reciprocal Rank Fusion para simplificar a busca.",
            "Qual estratégia encontra nomes de funções e perguntas que usam palavras diferentes?",
            "How do we merge keyword results with semantic matches without comparing incompatible score scales?",
        ],
        [
            "Vínculos observados entre decisão e componente vêm dos caminhos editados por apply_patch, write e edit, atribuídos à evidência específica. Ler um arquivo não prova alteração.",
            "A decision targets components identified by attributable edit and patch events in its own evidence. Read-only file access is not evidence of modification.",
            "Ligamos decisões a todos os arquivos lidos na sessão, inclusive consultas sem edição e mudanças pertencentes a outra decisão.",
            "Exigimos que o usuário escreva manualmente cada caminho de componente no prompt, sem aproveitar eventos de ferramentas.",
            "De onde extrair os módulos realmente modificados sem pedir caminhos ao usuário?",
            "Which recorded operations justify linking a decision to the code it changed?",
        ],
        [
            "Relações semânticas sugeridas no grafo guardam origem, evidência, confiança e versão do extrator. Similaridade recupera candidatos, mas não prova dependência causal.",
            "Semantic graph links retain provenance, supporting evidence, confidence and extractor version. Embedding similarity is candidate retrieval, not proof of causality.",
            "Toda similaridade acima de 0.8 vira automaticamente uma dependência causal definitiva no grafo, sem evidência nem origem.",
            "O grafo não armazena confiança ou histórico; links inferidos e observados têm a mesma autoridade e são indistinguíveis.",
            "Como rastrear por que a IA criou uma aresta e evitar confundir proximidade com causa?",
            "Find our policy for provenance and uncertainty on inferred decision relationships.",
        ],
        [
            "Separação por projeto é obrigatória antes de selecionar vizinhos. O filtro de project_id precede top-k para não vazar contexto e não perder resultados válidos.",
            "Project isolation applies project_id before nearest-neighbor top-k. Prefiltering prevents cross-project context leakage and missing valid local matches.",
            "Buscamos top-k global e só depois filtramos por projeto, mesmo quando isso deixa menos de k resultados.",
            "A memória universal mistura projetos por padrão e envia resultados de qualquer repositório sem escopo ou autorização.",
            "Como impedir que a busca traga decisões de outro repositório?",
            "Why must tenant filtering happen before choosing nearest neighbors?",
        ],
        [
            "Ao trocar o modelo de embeddings, criamos nova geração do índice com model_id, revisão e dimensão. Reconstruímos e trocamos atomicamente; não misturamos espaços vetoriais.",
            "Changing embedding models creates a fresh index generation recording model revision and dimensions. Rebuild first, then switch atomically without mixing vector spaces.",
            "Na troca de modelo reutilizamos vetores antigos quando têm a mesma dimensão, assumindo compatibilidade sem reconstrução.",
            "Removemos model_id e revisão do banco. O índice passa a aceitar quaisquer vetores e troca o modelo sem migração.",
            "O que fazer com a base vetorial quando substituímos o encoder?",
            "How should we migrate retrieval when the embedding model changes?",
        ],
        [
            "Exclusão e substituição de decisões invalidam seus vetores e arestas derivadas. Mantemos tombstones e regeneração idempotente para impedir contexto obsoleto.",
            "Deleting or superseding a decision invalidates its embeddings and derived links. Tombstones and idempotent rebuilds stop stale context from returning.",
            "Decisões excluídas continuam no índice vetorial permanentemente e aparecem nas consultas para preservar desempenho.",
            "A substituição cria nova decisão mas conserva arestas e embeddings antigos como se ainda estivessem vigentes.",
            "Como evitar que uma escolha abandonada volte nas recomendações?",
            "What removes stale retrieval context after a decision is superseded?",
        ],
        [
            "A configuração local usa CPU, no máximo quatro threads de inferência e lotes pequenos. O modelo é carregado sob demanda e descarregado após inatividade em PCs de 8 GB.",
            "On 8 GB CPU-only machines, inference uses at most four threads and small batches. Load the embedding model on demand and unload it after idle time.",
            "Exigimos GPU dedicada com 16 GB de VRAM para gerar embeddings, usando lotes de 256 itens e todos os núcleos.",
            "Mantemos dois modelos grandes residentes o tempo inteiro e não limitamos memória nem threads no aplicativo local.",
            "Qual política reduz a pressão de memória em computadores modestos?",
            "How do we run embeddings on a low-memory computer without a discrete graphics card?",
        ],
        [
            "FTS5 com tokenizer unicode61 e remoção de diacríticos serve à busca lexical em português. Desativamos stemming e stopwords inglesas na comparação com LanceDB.",
            "Portuguese keyword search uses Unicode tokenization and accent folding. The benchmark disables English stemming and stop words in LanceDB for fair lexical comparison.",
            "Aplicamos stemming e stopwords exclusivamente ingleses aos textos portugueses sem medir o efeito na recuperação.",
            "FTS5 é usado para gerar vetores neurais; o tokenizer também substitui o modelo multilíngue na busca semântica.",
            "Como configurar a busca de palavras para textos em português com acentos?",
            "Which lexical tokenizer settings avoid treating Portuguese text as English?",
        ],
        [
            "A aprovação grava decisão e evento de indexação na mesma transação SQLite, seguindo o transactional outbox. O consumidor processa depois e pode repetir sem duplicar vínculos.",
            "Transactional outbox commits the approved decision and indexing event together in SQLite. An idempotent consumer processes the event later.",
            "Publicamos o evento antes de persistir a decisão, em operações separadas sem outbox, aceitando perder indexações em caso de falha.",
            "O produtor grava só a decisão e espera que um serviço remoto detecte a mudança, sem evento transacional local.",
            "Como eliminar a janela de falha entre salvar a decisão e agendar sua indexação?",
            "Which pattern atomically persists approval and its background indexing event?",
        ],
        [
            "O aplicativo funciona offline após baixar uma vez os pesos de embeddings. Inferência e índices ficam no computador, sem servidor obrigatório nem cobrança por consulta.",
            "After the initial model download, embeddings and embedded indexes run offline on the user's computer without a mandatory hosted service or per-query fees.",
            "Cada embedding exige chamada à API remota paga e conexão ativa; não oferecemos inferência no computador.",
            "O banco vetorial embedded depende obrigatoriamente de cluster hospedado e a busca local para quando a internet cai.",
            "O usuário consegue pesquisar quando estiver sem internet?",
            "Which deployment choice avoids recurring hosting costs for retrieval?",
        ],
        [
            "Medimos carregamento e primeira inferência separados da latência aquecida. Reportamos p50 e p95 da geração de consulta e da busca, sem incluir download nos tempos de inferência.",
            "Benchmarks separate model loading and first inference from warm query encoding and index search. Report p50 and p95, and keep network download time separate.",
            "Usamos uma única consulta e reportamos só sua média, incluindo download dos pesos como se fosse tempo habitual de busca.",
            "Só medimos o índice com vetores prontos e chamamos esse número de latência total do assistente, ignorando o encoder.",
            "Que medições mostram o tempo percebido de consulta sem esconder o custo do modelo?",
            "How should cold start and steady-state retrieval latency be reported?",
        ],
    ];
    let mut docs = Vec::new();
    let mut queries = Vec::new();
    for topic in topics {
        let offset = docs.len();
        for text in &topic[..4] {
            docs.push(Document {
                id: docs.len(),
                text: text.to_string(),
            });
        }
        for text in &topic[4..] {
            queries.push(Query {
                text: text.to_string(),
                relevant: vec![offset, offset + 1],
            });
        }
    }
    (docs, queries)
}
