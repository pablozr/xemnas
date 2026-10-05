# Stack e arquitetura — Rust + GPUI

## Resumo executivo

O produto será um **monólito modular local** distribuído como aplicativo desktop. A UI em GPUI, os casos de uso, o motor decisional, a biblioteca de conhecimento e os workers rodam no mesmo processo. A comunicação interna usa chamadas Rust tipadas, não HTTP. Uma API HTTP local autenticada existe somente para adapters externos; MCP será a interface de consulta para agentes.

```text
                         ┌───────────────────────────────┐
OpenCode plugin ────────>│ API local / Capture Gateway   │
   └─ outbox se fechado  └──────────────┬────────────────┘
                                        │
┌───────────────────────────────────────▼──────────────────────┐
│                    Aplicativo desktop                        │
│                                                              │
│  GPUI ──> Application ──> Domain Modules ──> Infrastructure  │
│                    │              │                │          │
│                    │              │                ├─ SQLite  │
│                    │              │                ├─ files   │
│                    │              │                ├─ search  │
│                    │              │                └─ AI      │
│                    └──────────────┴──> background jobs        │
└───────────────────────────────────────┬──────────────────────┘
                                        │
                              MCP / Context Packs
                                        │
                                implementation agents
```

## Stack recomendada

| Área | Escolha inicial | Papel |
|---|---|---|
| Linguagem | Rust stable | Todo o núcleo e aplicativo desktop |
| Workspace | Cargo workspace | Monorepo com crates por módulo |
| UI | GPUI com versão/commit fixado | Janelas, navegação e design system |
| Async da UI | Executor do GPUI | Tarefas ligadas ao ciclo da interface |
| Async de I/O | Tokio | Workers, HTTP local e operações externas |
| Comunicação UI/motor | comandos Rust + channels | Sem HTTP interno |
| API local | Axum + Tower | Entrada de adapters externos |
| Serialização | Serde | Capture Envelopes e contratos persistidos |
| Contratos | Schemars + JSON Schema | Compatibilidade Rust ↔ adapters TypeScript |
| Banco | SQLite + rusqlite | Fonte local de dados estruturados |
| Migrações | rusqlite_migration | Evolução explícita do schema |
| Busca inicial | SQLite FTS5 | Pesquisa lexical e filtros |
| Arquivos | filesystem local content-addressed | PDFs, documentos e derivados grandes |
| Observabilidade | tracing + tracing-subscriber | Logs estruturados e diagnóstico |
| Erros | thiserror nos módulos; anyhow nos executáveis | Erros estáveis dentro, contexto nas bordas |
| IDs | UUID v7 | Identificadores ordenáveis e portáveis |
| MCP | binário Rust por stdio | Consulta de contexto por agentes |
| Testes | cargo test/nextest + snapshots pontuais | Domínio, contratos, storage e UI |
| Supply chain | cargo-deny + cargo-audit | Licenças, vulnerabilidades e dependências |

Nenhuma biblioteca de embeddings, banco vetorial ou GraphRAG entra no MVP. Essas escolhas somente serão feitas depois de medir a qualidade da busca lexical e dos Context Packs.

## Organização do repositório

**Divisão-alvo conceitual, não árvore atual.** Para arquivos e símbolos existentes,
consulte o [mapa do código](mapa-do-codigo.md).

```text
/
├── Cargo.toml
├── Cargo.lock
├── CONTEXT.md
├── apps/
│   ├── desktop-gpui/
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── app.rs
│   │   │   ├── navigation/
│   │   │   ├── screens/
│   │   │   │   ├── projects/
│   │   │   │   ├── decision_inbox/
│   │   │   │   ├── decisions/
│   │   │   │   ├── context/
│   │   │   │   ├── knowledge/
│   │   │   │   └── settings/
│   │   │   └── design_system/
│   │   └── assets/
│   └── mcp-server/
│       └── src/main.rs
├── crates/
│   ├── domain/
│   ├── application/
│   ├── projects/
│   ├── capture/
│   ├── decisions/
│   ├── project-context/
│   ├── assessments/
│   ├── knowledge/
│   ├── retrieval/
│   ├── agent-access/
│   ├── integration-contracts/
│   ├── storage-sqlite/
│   ├── content-store/
│   ├── local-api/
│   └── telemetry/
├── adapters/
│   └── opencode/
│       ├── src/
│       ├── schemas/
│       └── package.json
├── migrations/
├── tests/
│   ├── fixtures/
│   ├── contract/
│   └── end-to-end/
├── docs/
│   ├── adr/
│   └── architecture/
└── tools/
```

Crates não serão criadas apenas para obter uma árvore bonita. Esta é a divisão-alvo; no MVP, módulos pequenos podem começar juntos e só ganhar uma crate própria quando houver uma interface real ou necessidade clara de compilação/teste independente.

## Regra de dependência

```text
desktop-gpui ─┐
local-api ────┼──> application ──> domain
mcp-server ───┘         │             ▲
                        └─ interfaces ─┤
                                      │
storage-sqlite / content-store / AI adapters
          implementam essas interfaces
```

Regras:

1. `domain` não depende de GPUI, SQLite, HTTP, OpenCode, MCP ou provedores de IA.
2. `application` orquestra casos de uso e define as interfaces que precisa.
3. infraestrutura implementa essas interfaces.
4. GPUI converte interação visual em comandos da aplicação e renderiza resultados.
5. adapters externos só conhecem contratos versionados.
6. nenhum tipo do GPUI pode aparecer fora de `apps/desktop-gpui`.

## Módulos de domínio

### Projects

Responsável pela identidade e configuração dos projetos acompanhados.

Interface conceitual:

```rust
trait Projects {
    fn register(&self, location: ProjectLocation) -> Result<Project>;
    fn get(&self, id: ProjectId) -> Result<Project>;
    fn list(&self) -> Result<Vec<ProjectSummary>>;
}
```

Não analisa código nem decisões; apenas estabelece qual Project existe e quais fontes podem se associar a ele.

### Capture

Recebe `CaptureEnvelope`, valida versão, autentica origem, deduplica, cria Source Artifacts e atualiza checkpoints.

```rust
trait CaptureGateway {
    fn ingest(&self, envelope: CaptureEnvelope) -> Result<CaptureReceipt>;
}
```

Toda complexidade de idempotência, schema, limites, redaction e associação de mensagens fica escondida atrás dessa interface.

### Decisions

Controla o ciclo de vida decisional:

```text
observed → candidate → confirmed
                     ├─ rejected
                     ├─ superseded
                     └─ pending evidence
```

Casos de uso públicos:

- detectar ou atualizar candidatos a partir de Source Artifacts;
- listar a Decision Inbox;
- confirmar, corrigir, rejeitar ou adiar candidato;
- consultar uma Engineering Decision e seu histórico;
- registrar que uma decisão foi substituída, sem apagar a anterior.

### Project Context

Mantém Context Claims, validade temporal, relações, snapshots e geração de Context Packs.

Sua interface mais importante deve permanecer pequena:

```rust
trait ContextProvider {
    fn build_pack(&self, request: ContextRequest) -> Result<ContextPack>;
}
```

Quem pede contexto informa Project, tarefa, escopo, orçamento de tamanho e data de referência. O módulo decide como selecionar, ranquear, citar e limitar o conteúdo.

### Assessments

Produz análises reproduzíveis sobre decisões e contexto, mantendo modelo, prompt/política, entradas, horário, resultado e confiança. Assessment nunca altera uma Engineering Decision automaticamente.

### Knowledge Library

Gerencia Knowledge Sources globais, versões, coleções, extração e Project Knowledge Links.

Fluxo:

```text
documento/site
   ↓
registrar origem e direitos
   ↓
armazenar conteúdo
   ↓
extrair texto e estrutura
   ↓
fragmentos citáveis
   ↓
indexar
```

Uma fonte global vinculada a um Project fica disponível para consulta, mas não vira Context Claim nem decisão.

### Retrieval

Esconde como conteúdo é encontrado. Inicialmente combina FTS5, filtros de escopo, recência e tipos. Busca semântica pode ser adicionada como outro adapter interno sem mudar `ContextProvider`.

### Agent Access

Aplica autorização, orçamento de contexto, redaction e auditoria antes de entregar Context Packs a agentes. Esse módulo impede que MCP vire acesso irrestrito ao banco.

## Camada Application

A camada `application` expõe comandos e consultas orientados a intenção, não CRUD genérico:

```rust
enum Command {
    RegisterProject(RegisterProject),
    ImportCapture(ImportCapture),
    ReviewCandidate(ReviewCandidate),
    ImportKnowledgeSource(ImportKnowledgeSource),
    LinkKnowledgeToProject(LinkKnowledgeToProject),
}

enum Query {
    ProjectDashboard(ProjectDashboardQuery),
    DecisionInbox(DecisionInboxQuery),
    DecisionDetails(DecisionDetailsQuery),
    SearchKnowledge(SearchKnowledgeQuery),
    BuildContextPack(ContextRequest),
}
```

Não é necessário criar um único `CommandBus` abstrato desde o começo. As telas podem receber uma fachada `AppUseCases` com poucos métodos; comandos tipados mostram o formato desejado e facilitam logging, testes e tarefas assíncronas.

## Estado e concorrência

### Thread da UI

- GPUI mantém somente estado de apresentação e navegação.
- Nenhum acesso pesado ao banco, PDF ou modelo ocorre na thread da UI.
- Cada tela dispara um caso de uso e recebe progresso/resultado por mensagem.

### Runtime de trabalho

- Um runtime Tokio interno executa HTTP, importação, extração e chamadas de modelo.
- CPU pesada usa `spawn_blocking` ou um pool próprio limitado.
- Jobs são persistidos antes da execução e possuem estados `queued`, `running`, `completed`, `failed`, `cancelled`.
- Ao reiniciar, jobs interrompidos voltam para `queued` quando a operação for idempotente.
- Jobs rodam em filas por assunto (sessões, documentação, sugestões), cada uma com seus workers; detalhes em `desempenho-e-escala.md`.

### SQLite

- Escritas passam por uma fila única para evitar contenção e `database is locked`.
- Leituras usam conexões separadas e curtas.
- WAL permite leitura enquanto uma escrita é confirmada.
- Transações seguem o caso de uso, não a tela.
- A UI recebe DTOs imutáveis e nunca uma conexão ou row do SQLite.

## API local

### Ciclo de vida

- Inicializada junto com o aplicativo.
- Bind exclusivo em `127.0.0.1`/`::1`.
- Porta escolhida dinamicamente.
- Um arquivo de discovery local informa porta, versão do protocolo e instance ID.
- Token aleatório por instalação ou sessão, armazenado com permissões restritas.
- Encerrada junto com o aplicativo.

### Endpoints iniciais

```text
GET  /v1/health
GET  /v1/capabilities
POST /v1/captures
GET  /v1/captures/{id}
```

Somente depois:

```text
POST /v1/context-packs
GET  /v1/projects/{id}/decisions
POST /v1/search
```

`POST /v1/captures` recebe um Capture Envelope, exige `Idempotency-Key`, aplica limite de tamanho e retorna rapidamente após persistir. Detecção de decisões acontece em job assíncrono.

### Segurança mínima

- nunca escutar em `0.0.0.0` por padrão;
- Bearer token local;
- nenhuma política CORS permissiva;
- limite de corpo e timeout;
- validação estrita do JSON Schema;
- canonicalização e allowlist de caminhos de projetos;
- redaction antes da persistência;
- logs sem conteúdo sensível por padrão;
- trilha de consultas feitas por agentes.

## Adapter do OpenCode

O adapter continua pequeno e separado, provavelmente em TypeScript por ser o ambiente natural do plugin:

```text
session.status → idle
       ↓
identifica novas mensagens/checkpoint
       ↓
obtém user turn + child responses + message diff
       ↓
monta Capture Envelope versionado
       ↓
app disponível? ── sim ──> POST /v1/captures
       │
       não
       ↓
grava arquivo temporário + rename atômico na outbox
```

Ele não chama modelo, não decide relevância e não cria Decision Candidate. Sua responsabilidade termina após entregar uma captura válida.

### Capture Envelope conceitual

```json
{
  "schema_version": 1,
  "capture_id": "uuid-v7",
  "idempotency_key": "opencode:session:message:diff-hash",
  "source": {
    "adapter": "opencode",
    "adapter_version": "...",
    "session_id": "...",
    "message_id": "..."
  },
  "project": {
    "canonical_path": "..."
  },
  "observed_at": "...",
  "artifacts": []
}
```

O contrato real será produzido por tipos Rust e JSON Schema. Campos de reasoning, segredos e conteúdos completos não entram por padrão.

## Outbox

A outbox é transporte de fallback, não banco alternativo:

```text
outbox/
├── pending/
├── sending/
├── accepted/
└── rejected/
```

O adapter grava em arquivo temporário e renomeia atomicamente para `pending`. Quando o app abre, importa, deduplica e move o item para `accepted`; inválidos vão para `rejected` com diagnóstico. A retenção apaga aceitos após período configurável.

## MCP para agentes

`apps/mcp-server` será um binário pequeno iniciado pelo cliente de agente via stdio. Ele não acessa tabelas diretamente; chama a interface de Agent Access.

Ferramentas futuras:

```text
list_projects
get_project_summary
search_decisions
get_decision
build_context_pack
search_knowledge
explain_context_source
```

Princípios:

- ferramentas orientadas a intenção, não SQL/CRUD;
- resultados com IDs, citações e validade;
- limite explícito de tamanho;
- leitura por padrão;
- nenhum agente confirma decisões;
- nenhuma ferramenta escreve no repositório.

Enquanto o app estiver aberto, MCP conversa com ele por IPC/API local. Suporte headless com o app fechado só será adicionado se houver necessidade real.

## Persistência

### SQLite — agrupamento lógico de tabelas

```text
projects

source_artifacts
capture_receipts
adapter_checkpoints

decision_candidates
engineering_decisions
decision_revisions
decision_relations

context_claims
claim_relations
evidence_links
assessments

knowledge_sources
knowledge_versions
knowledge_fragments
knowledge_collections
project_knowledge_links

jobs
agent_access_log
schema_migrations
```

Relações de evidência usam IDs e tipos explícitos; não armazenaremos “um grafo genérico” sem semântica. A visualização em grafo é uma projeção dessas relações.

### Filesystem

```text
app-data/
├── state/app.db
├── content/sha256/...
├── extracted/...
├── indexes/...
├── outbox/...
├── logs/...
└── runtime/discovery.json
```

- `content/` guarda originais importados quando autorizado.
- `extracted/` e `indexes/` são derivados e reconstruíveis.
- caminhos de projetos são referências; o app não copia o repositório.
- decisões confirmadas ficam locais; exportação é uma ação explícita.

## Uso de IA

O domínio não conhece LLMs. `application` usa interfaces pequenas:

```rust
trait CandidateExtractor {
    fn extract(&self, input: DecisionEvidence) -> Result<Vec<CandidateProposal>>;
}

trait AssessmentGenerator {
    fn assess(&self, input: AssessmentInput) -> Result<AssessmentDraft>;
}
```

Adapters poderão implementar modelo local ou provedor externo. Toda saída passa por validação estruturada e mantém proveniência. Um resultado de IA nunca é persistido como decisão confirmada sem ação humana.

### Provedores e perfis

O usuário seleciona um `AI Execution Profile`, não um provedor codificado no caso de uso. Um perfil combina:

- adapter de provedor ou gateway;
- modelo;
- capacidades necessárias, como saída estruturada e contexto máximo;
- limites de custo e tokens;
- política de envio e redaction;
- finalidade autorizada, como extração ou assessment.

Adapters iniciais possíveis:

```text
OpenAIProvider
AnthropicProvider
OpenAICompatibleProvider   # Ollama, LM Studio e endpoints locais
OpenCodeGatewayProvider    # delega ao servidor programável do OpenCode
FakeProvider               # testes determinísticos
```

OpenCode é tratado como gateway/orquestrador, não como modelo. O adapter usa sua interface pública e nunca lê diretamente seu armazenamento de credenciais. `CandidateExtractor` depende de capacidades e de um perfil; não conhece nomes de fornecedores.

Chamadas externas permanecem desativadas até configuração explícita. Antes do primeiro envio, a interface mostra quais dados podem sair da máquina; cada Assessment registra perfil, modelo, política e fontes utilizadas.

## Telas do aplicativo

```text
Projects
  └── Project Dashboard
      ├── Decision Inbox
      ├── Decisions
      ├── Context
      ├── Timeline
      ├── Graph
      └── Sources

Knowledge Library
  ├── Sources
  ├── Collections
  └── Imports

Engineering Assistant        # futuro
Settings
  ├── Adapters
  ├── Models and privacy
  ├── Storage
  └── Diagnostics
```

Primeiro design system GPUI necessário: typography, spacing, colors, button, icon button, text field, select, checkbox, tabs, list row, virtual list, modal, popover, tooltip, toast, progress, empty/error states e focus ring.

A regra visual canônica está em [VISUAL-IDENTITY.md](../design/VISUAL-IDENTITY.md).
O design system original é referência histórica complementar. A implementação
deve centralizar as receitas de glass e não espalhar cores ou efeitos pelas views.

## Fases de implementação

### Fase 0 — vertical slice de GPUI

- shell e navegação;
- design tokens e cinco controles básicos;
- lista virtualizada com 10 mil itens;
- detalhe de decisão;
- SQLite;
- job em background;
- API local autenticada;
- testes de teclado e Narrator;
- medição de startup, memória e scroll;
- empacotamento Windows.

Saída: aceitar GPUI ou trocar somente `desktop-gpui`.

### Fase 1 — captura confiável

- Projects;
- contratos e JSON Schema;
- Local API;
- adapter OpenCode;
- outbox;
- Source Artifacts e checkpoints;
- diagnóstico de captura.

### Fase 2 — Decision Inbox

- extração de candidatos;
- diff como evidência;
- confirmar, corrigir, rejeitar e adiar;
- decisões locais e histórico;
- busca inicial.

### Fase 3 — contexto recuperável

- Context Claims;
- relações e validade;
- Context Packs manuais;
- timeline;
- primeira visualização em grafo.

### Fase 4 — biblioteca global

- importação de PDFs e sites;
- metadados e citações;
- coleções e vínculos com projetos;
- busca unificada.

### Fase 5 — acesso por agentes

- MCP read-only;
- políticas e auditoria;
- Context Packs por tarefa;
- avaliação de utilidade antes de injeção automática.

### Fase 6 — assistente de engenharia

- análise de alternativas;
- explicação de trade-offs;
- detecção de conflitos;
- condições de reconsideração;
- avaliações citadas, nunca decisões autônomas.

## O que não entra no MVP

- microserviços;
- daemon permanente;
- banco de grafo;
- vector database obrigatório;
- GraphRAG;
- sincronização cloud;
- multiusuário;
- escrita automática no repositório;
- commits automáticos;
- confirmação autônoma de decisões;
- dependência do OpenCode dentro do núcleo.

## Primeira decisão executável

O próximo passo correto não é implementar todo o produto. É construir a Fase 0 como protótipo descartável, medir seus gates e então criar o workspace definitivo. Assim confirmamos que GPUI atende o produto sem acoplar a visão inteira a uma hipótese de UI ainda pré-1.0.
