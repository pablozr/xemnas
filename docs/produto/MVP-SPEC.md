# Contextual Engineering — especificação executável do MVP

**Status:** aprovado para implementação  
**Produto:** aplicativo desktop local de memória decisional para engenharia de software  
**Plataforma inicial:** Windows  
**UI:** Rust + GPUI  
**Primeira integração:** OpenCode  
**Design system:** [Quiet Glass](../design/design-system-quiet-glass.md)  

Este documento é a fonte de verdade para agentes que forem planejar e implementar o MVP. Em caso de conflito, as decisões e limites aqui descritos prevalecem sobre documentos exploratórios anteriores.

---

## 1. O que é o projeto

O produto é um aplicativo desktop local que transforma o rastro de trabalho de agentes de programação em uma memória decisional revisável e navegável por projeto.

Enquanto o desenvolvedor trabalha no OpenCode, uma integração observa silenciosamente os turnos e os diffs da sessão. O sistema identifica escolhas que talvez restrinjam trabalho futuro, cria **Decision Candidates** e os coloca numa inbox. Quando quiser, o usuário revisa cada candidato, corrige o que estiver errado e decide se aquilo merece virar uma **Engineering Decision**.

O produto não é um gravador de conversas, um gerador automático de ADRs nem um agente que decide pelo usuário. Sua função é preservar intenção, justificativa, evidência e condições de reconsideração sem interromper o fluxo de programação.

### Definição em uma frase

> Uma memória de engenharia local que observa o trabalho com agentes, propõe decisões relevantes e permite ao desenvolvedor transformar essas propostas em contexto confiável para o futuro.

### Problema central

Em desenvolvimento assistido por agentes, o humano costuma decidir o que construir, enquanto várias escolhas de execução surgem durante a implementação. Parte dessas escolhas altera contratos, dependências, persistência, segurança ou padrões do projeto sem ser registrada como decisão. Depois, nem o humano nem outro agente conseguem reconstruir com confiança por que o sistema ficou daquele jeito.

ADRs manuais ajudam, mas normalmente são escritos tarde, dependem de disciplina e capturam apenas o resultado final. O produto busca observar o comportamento que antecede a decisão e reduzir o custo de preservá-lo.

---

## 2. Visão de longo prazo

O destino do produto é um ambiente local de inteligência de engenharia com três camadas:

1. **Memória por projeto:** decisões, premissas, restrições, evidências, contexto atual, histórico, timeline e relações.
2. **Biblioteca global:** papers, livros, PDFs, documentação, normas, sites e anotações citáveis e vinculáveis a projetos.
3. **Assistente de engenharia:** comparação de alternativas, explicação de trade-offs, identificação de conflitos e montagem de contexto específico para pessoas e agentes.

No futuro, cada projeto poderá ser navegado visualmente, incluindo relações semelhantes à experiência de um knowledge graph. Entretanto, o grafo será uma projeção de relações tipadas e auditáveis — não o modelo de armazenamento principal.

Agentes de implementação poderão pedir **Context Packs** pequenos e específicos para uma tarefa por MCP ou API. Eles não receberão um despejo irrestrito do banco e não poderão confirmar decisões em nome do usuário.

Essa visão orienta as fronteiras arquiteturais, mas não amplia o MVP.

---

## 3. Objetivo e escopo exato do MVP

O MVP deve provar este ciclo completo:

```text
trabalho normal no OpenCode
        ↓
captura silenciosa de um turno e seu diff
        ↓
persistência confiável e deduplicada
        ↓
extração de possíveis decisões
        ↓
Decision Inbox sem interrupção
        ↓
revisão humana
        ↓
Engineering Decision local e consultável
```

### O MVP inclui

- aplicativo desktop Windows em Rust + GPUI;
- cadastro de projetos locais;
- integração inicial por adapter/plugin do OpenCode;
- captura de turno, metadados e session diff por APIs públicas do OpenCode;
- API HTTP exclusivamente local e autenticada para receber capturas;
- outbox quando o aplicativo estiver fechado;
- persistência SQLite local;
- jobs de análise em background;
- interface substituível `CandidateExtractor`;
- extractor fake/determinístico para testes;
- pelo menos um adapter real configurável de IA;
- escolha explícita de perfil/provedor de IA nas configurações;
- Decision Inbox;
- confirmação, edição, rejeição e adiamento de candidatos;
- lista e detalhe de decisões confirmadas;
- evidências rastreáveis até o turno e diff capturados;
- busca lexical básica por decisões;
- diagnóstico da integração e política de privacidade visível;
- exportação manual de uma decisão em Markdown ou JSON, sem alterar o repositório;
- design visual Quiet Glass e acessibilidade básica por teclado/Narrator.

### O MVP não inclui

- daemon permanente;
- confirmação durante a sessão do OpenCode;
- modais ou prompts automáticos que interrompam o trabalho;
- commits automáticos ou criação de arquivos no repositório;
- Git/GitHub como fonte primária de detecção;
- reconstrução completa de decisões históricas;
- biblioteca global de PDFs e sites;
- embeddings, vector database ou GraphRAG;
- visualização completa em grafo;
- Context Packs automáticos;
- servidor MCP para agentes;
- assistente de engenharia;
- sincronização cloud, colaboração ou multiusuário;
- suporte de produção a adapters além do OpenCode;
- microserviços ou banco de grafo.

### Critério de sucesso do MVP

O MVP é útil se, por pelo menos uma semana de uso real:

- o fluxo de programação não for interrompido;
- capturas válidas não forem perdidas quando o app estiver fechado;
- candidatos duplicados forem raros e controláveis;
- a maioria dos candidatos exibidos puder ser revisada em menos de 45 segundos;
- decisões aceitas preservarem justificativa e evidência suficientes para serem compreendidas dias depois;
- o usuário preferir manter o sistema ligado em vez de desligá-lo por ruído ou atrito.

Quantidade de decisões não é métrica de sucesso. Precisão percebida, baixo atrito e utilidade futura são.

---

## 4. Princípios de produto

1. **Captura silenciosa, confirmação assíncrona.** Trabalhar nunca depende de revisar candidatos.
2. **Proposta não é fato.** IA produz candidatos; somente a ação humana produz uma decisão confirmada.
3. **Turno fornece intenção; diff fornece evidência.** Nenhum deles isoladamente é a verdade completa.
4. **Local e privado por padrão.** Dados não saem da máquina sem configuração e consentimento explícitos.
5. **Proveniência sempre visível.** Toda afirmação relevante deve apontar para sua origem.
6. **Recuperação acima de documentação ornamental.** O valor é entender uma escolha futura, não acumular ADRs.
7. **Fronteiras substituíveis.** OpenCode e qualquer provedor de IA são adapters, não partes do domínio.
8. **Não inventar certeza.** Lacunas, baixa confiança e evidência incompleta devem aparecer como tais.
9. **Sem mutação surpresa.** O sistema não escreve nem faz commit no projeto automaticamente.
10. **Arquitetura proporcional.** Monólito modular primeiro; distribuição apenas quando houver necessidade medida.

---

## 5. Modelo mental e linguagem do domínio

Use estes termos consistentemente em código, UI e documentação.

### Project

Um repositório ou diretório de trabalho acompanhado. Estabelece identidade, localização canônica e configurações próprias. Não é cópia do repositório.

### Source Artifact

Evidência imutável observada por um adapter: trecho de turno, metadado de ferramenta, hunk de diff ou outro artefato de origem. Possui proveniência e fingerprint.

### Capture Envelope

Contrato versionado entregue por um adapter. Agrupa identidade da fonte, projeto, momento observado e Source Artifacts. Sua aceitação não significa que exista uma decisão.

### Decision Episode

Intervalo de deliberação potencial que pode atravessar mais de um turno. No MVP ele pode ser apenas uma estrutura interna ou de apresentação. Não se deve tentar descobrir uma “task” perfeita.

### Decision Candidate

Proposta revisável de que uma escolha durável ocorreu. É falível, pode estar incompleta e nunca deve ser apresentada como decisão confirmada.

Estados:

```text
pending → accepted
        → edited_and_accepted
        → dismissed
        → snoozed
```

### Engineering Decision

Escolha confirmada pelo usuário com pergunta, escolha, justificativa, escopo, premissas, evidências e condições de reconsideração. Continua local até exportação explícita.

### Evidence Link

Relação tipada de um candidato ou decisão com Source Artifacts. A evidência sustenta ou contextualiza uma decisão; não prova automaticamente que ela é correta.

### Assessment

Análise gerada por IA ou regra sobre evidências e decisões. Deve guardar entradas, política, modelo, horário, resultado e confiança. Nunca altera uma decisão automaticamente.

### AI Execution Profile

Configuração escolhida pelo usuário que combina adapter, modelo, finalidade, limites e política de envio/redaction. OpenAI, outro provedor, endpoint compatível ou gateway OpenCode são implementações possíveis.

### Outbox

Transporte local temporário usado pelo adapter quando o aplicativo não responde. Não é um segundo banco.

---

## 6. Quando uma escolha merece ser proposta

O extractor deve procurar compromisso com uma direção, não qualquer edição de código.

Uma condição forte já pode justificar um candidato:

- difícil ou caro de reverter;
- altera contrato público, schema ou persistência;
- atravessa componentes, repositórios ou boundaries;
- afeta segurança, privacidade, confiabilidade ou compliance;
- introduz dependência externa durável;
- rejeita alternativa plausível que provavelmente será rediscutida;
- condiciona trabalho futuro de pessoas ou agentes;
- possui blast radius material.

Condições moderadas ganham força quando aparecem juntas:

- alternativas reais comparadas;
- trade-off entre atributos de qualidade;
- desacordo ou incerteza;
- custo relevante;
- validade esperada por meses;
- impacto em manutenção ou onboarding;
- escolha delegada ao agente sem instrução explícita;
- premissa importante ainda não comprovada.

Não criar candidato para:

- detalhe local e facilmente reversível;
- formatação ou preferência estética sem consequência;
- aplicação de padrão já confirmado;
- fato sem escolha;
- modificação que um teste, lint ou comentário explica melhor;
- implementação sem alternativa relevante.

O sistema pode exibir os sinais detectados, mas não deve apresentar um score opaco como verdade.

---

## 7. Fluxo funcional detalhado

### 7.1 Captura no OpenCode

O adapter usa eventos oficiais apenas como gatilhos. A fonte canônica é a API/SDK pública do OpenCode.

Fluxo preferido:

```text
session.status → idle (ou session.idle compatível)
        ↓ debounce curto
identificar novas mensagens desde o checkpoint
        ↓
buscar user turn + respostas filhas + session diff por message ID
        ↓
redigir/limitar conteúdo segundo política
        ↓
montar Capture Envelope versionado
        ↓
POST local ou outbox
```

Regras:

- `idle` é checkpoint técnico, não fim semântico da tarefa;
- a Session é contêiner e o user turn é a unidade técnica inicial;
- o adapter retorna rapidamente e nunca executa modelo;
- reasoning interno, segredos e blobs completos não são capturados por padrão;
- diffs grandes são reduzidos a hunks e limites configuráveis;
- side effects fora do diretório ativo são declarados como não observados;
- checkpoints tornam o fluxo idempotente;
- eventos perdidos devem poder ser reconciliados pela API.

### 7.2 Quando o app está fechado

O adapter escreve primeiro um arquivo temporário e faz rename atômico para `outbox/pending`. Na próxima abertura, o app valida, deduplica e importa.

```text
outbox/
├── pending/
├── sending/
├── accepted/
└── rejected/
```

Itens aceitos possuem retenção limitada. Itens rejeitados preservam diagnóstico seguro. Nunca apagar um pending silenciosamente.

### 7.3 Ingestão

`POST /v1/captures`:

1. autentica o caller;
2. valida schema e tamanho;
3. canonicaliza o caminho do projeto;
4. verifica allowlist/registro do projeto;
5. deduplica por `Idempotency-Key` e fingerprint;
6. persiste receipt e artefatos numa transação;
7. agenda análise;
8. retorna receipt sem esperar a IA.

### 7.4 Extração

Executar em duas passagens:

1. filtro barato identifica sinais de escolha durável;
2. somente casos relevantes são enviados ao `CandidateExtractor` profundo.

O extractor recebe evidência limitada e produz saída estruturada validada. Se falhar, o job registra diagnóstico e pode ser reprocessado. Falha de IA jamais invalida a captura.

### 7.5 Inbox

A inbox aparece quando o usuário abrir o app ou a tela. Não usar modal automático. No máximo, exibir badge discreto.

Cada item deve mostrar:

- pergunta/decisão proposta;
- escolha identificada;
- justificativa inferida, claramente marcada;
- projeto, sessão e data;
- sinais de relevância;
- resumo do diff e arquivos afetados;
- confiança explicável;
- ações `Confirmar`, `Ajustar`, `Rejeitar` e `Adiar`.

Confirmar abre ou mantém edição inline antes de salvar. O usuário deve poder corrigir todos os campos substantivos.

### 7.6 Decisão confirmada

Formato mínimo:

```yaml
id: uuid-v7
project_id: uuid-v7
status: accepted
question: string
choice: string
rationale: string
assumptions: []
reconsider_when: []
scope: []
consequences: []
evidence: []
created_at: timestamp
confirmed_at: timestamp
```

Uma decisão confirmada pode ser editada por revisão versionada. Nunca apagar silenciosamente a versão anterior. No futuro poderá ser superseded; o MVP pode modelar esse estado mesmo sem UI completa.

Não criar arquivo no working tree ao confirmar. Exportar é outro comando explícito, com preview e destino escolhido pelo usuário. O produto nunca realiza commit.

---

## 8. Experiência desktop do MVP

### Navegação principal

```text
Projects
  └── Project Dashboard
      ├── Decision Inbox
      └── Decisions

Settings
  ├── OpenCode integration
  ├── AI profiles and privacy
  └── Diagnostics
```

### Telas obrigatórias

#### Projects

- registrar diretório local;
- listar projetos;
- mostrar estado da captura, candidatos pendentes e última atividade;
- remover acompanhamento sem apagar o repositório.

#### Decision Inbox

- lista virtualizada e filtrável;
- painel de detalhe;
- diff e fontes sob disclosure;
- edição antes da confirmação;
- ações em lote apenas para rejeitar/adiar, nunca confirmar sem revisão.

#### Decisions

- lista de decisões aceitas;
- busca textual;
- detalhe com proveniência;
- histórico de revisões;
- exportação manual.

#### Settings — OpenCode

- status da integração;
- versão detectada e compatibilidade;
- caminho da outbox;
- último checkpoint;
- teste de conexão;
- mensagens de erro acionáveis.

#### Settings — IA e privacidade

- habilitar explicitamente chamadas externas;
- escolher perfil e modelo;
- preview representativo do que pode ser enviado;
- opções de redaction e limites;
- teste com resposta estruturada;
- possibilidade de usar fake/offline sem configurar provedor.

#### Diagnostics

- jobs e falhas recentes;
- receipts de captura sem expor conteúdo sensível por padrão;
- reprocessar job falho;
- abrir diretório de logs/outbox;
- exportar diagnóstico sanitizado.

### Design visual

Implementar o [Quiet Glass Design System](../design/design-system-quiet-glass.md) e usar [esta referência canônica](../design/design-system-reference.png).

Direção resumida:

- fundo azul-carvão;
- lavanda mineral suave ocupando menos de 8% da tela;
- glass seletivo e centralizado em primitives;
- tipografia Inter Variable e JetBrains Mono para código;
- listas contínuas, não mosaicos de cards;
- sofisticação por proporção e material, não decoração;
- uma ação primária por tela;
- estados completos de teclado e focus-visible.

Não inventar cores, glass ou componentes locais fora dos tokens. A ausência de blur real deve usar o fallback documentado.

---

## 9. Arquitetura aprovada

### Forma geral

Um **monólito modular local**. GPUI, casos de uso, domínio, persistência e workers vivem no mesmo aplicativo. Comunicação interna usa tipos e chamadas Rust. HTTP existe somente na fronteira com adapters externos.

```text
OpenCode adapter
      │ HTTP local ou outbox
      ▼
Capture Gateway / Local API
      ▼
Application Use Cases
      ▼
Domain ──────────────> ports/interfaces
                          ▲
             SQLite / AI / filesystem

GPUI ───────────────> Application Use Cases
```

### Stack

| Área | Escolha |
|---|---|
| Linguagem | Rust stable |
| Workspace | Cargo workspace |
| UI | GPUI com revisão fixada |
| I/O assíncrono | Tokio |
| API local | Axum + Tower |
| Persistência | SQLite + rusqlite |
| Migrações | rusqlite_migration |
| Contratos | Serde + Schemars + JSON Schema |
| Busca | SQLite FTS5 |
| Logs | tracing + tracing-subscriber |
| Erros | thiserror em módulos; anyhow nos binários |
| IDs | UUID v7 |
| Adapter OpenCode | TypeScript |
| Testes | cargo test/nextest + contract/E2E |

Python/FastAPI não será usado no runtime principal. Embora seja familiar e válido para protótipos, criaria um segundo runtime, distribuição mais complexa e contratos internos desnecessários. Rust de ponta a ponta é a escolha equilibrada para um aplicativo desktop local, com TypeScript somente onde o ecossistema do OpenCode pede.

### Regras de dependência

- `domain` não conhece GPUI, SQLite, HTTP, OpenCode ou fornecedores de IA;
- `application` orquestra casos de uso e define ports;
- infraestrutura implementa os ports;
- views GPUI recebem DTOs e enviam comandos;
- nenhum tipo GPUI escapa do app desktop;
- adapters externos conhecem apenas contratos versionados;
- módulos não acessam tabelas de outros módulos diretamente por conveniência;
- evitar abstrações genéricas sem dois casos concretos.

### Estrutura inicial recomendada

```text
/
├── Cargo.toml
├── Cargo.lock
├── CONTEXT.md
├── apps/
│   └── desktop-gpui/
├── crates/
│   ├── domain/
│   ├── application/
│   ├── integration-contracts/
│   ├── storage-sqlite/
│   ├── local-api/
│   └── telemetry/
├── adapters/
│   └── opencode/
├── migrations/
├── tests/
│   ├── fixtures/
│   ├── contract/
│   └── end-to-end/
└── docs/
    └── adr/
```

Não criar uma crate por substantivo antecipadamente. `projects`, `capture` e `decisions` podem iniciar como módulos coesos dentro de `domain`/`application` e ser extraídos quando houver seam real.

### Concorrência

- thread GPUI mantém apenas estado de apresentação;
- Tokio executa HTTP, filesystem e provedores;
- trabalho pesado usa `spawn_blocking` ou pool limitado;
- jobs são persistidos antes de executar;
- escritas SQLite passam por fila única;
- leituras usam conexões curtas separadas;
- WAL habilitado;
- operações canceláveis não podem deixar estado parcial.

### Ciclo de vida da API local

- inicia e encerra com o desktop;
- bind apenas em `127.0.0.1`/`::1`;
- porta dinâmica;
- arquivo local de discovery com porta, versão e instance ID;
- token aleatório guardado com permissão restrita;
- limites de corpo, timeout e schemas estritos;
- sem CORS permissivo.

Endpoints do MVP:

```text
GET  /v1/health
GET  /v1/capabilities
POST /v1/captures
GET  /v1/captures/{id}
```

Não expor CRUD do domínio por HTTP. UI e domínio não conversam por HTTP.

---

## 10. Contrato inicial de captura

Forma conceitual, sujeita a refinamento sem mudar a semântica:

```json
{
  "schema_version": 1,
  "capture_id": "uuid-v7",
  "idempotency_key": "opencode:session:user-message:diff-fingerprint",
  "source": {
    "adapter": "opencode",
    "adapter_version": "string",
    "session_id": "string",
    "message_id": "string"
  },
  "project": {
    "canonical_path": "absolute-path"
  },
  "observed_at": "rfc3339",
  "artifacts": [
    {
      "artifact_id": "uuid-v7",
      "kind": "user_text | assistant_text | diff_hunk | tool_summary",
      "content": "redacted-and-bounded-string",
      "metadata": {},
      "fingerprint": "sha256"
    }
  ]
}
```

O contrato real nasce de tipos Rust e gera JSON Schema consumido pelo adapter TypeScript. Testes de contrato verificam fixtures válidas, inválidas e compatibilidade da versão suportada do OpenCode.

---

## 11. Modelo de persistência mínimo

Tabelas lógicas:

```text
projects
source_artifacts
capture_receipts
adapter_checkpoints
decision_candidates
engineering_decisions
decision_revisions
evidence_links
assessments
jobs
schema_migrations
```

Requisitos:

- foreign keys habilitadas;
- timestamps UTC;
- migrations forward-only testadas em banco vazio e banco da versão anterior;
- conteúdo derivado deve ser reconstruível;
- artefatos grandes não viram colunas ilimitadas sem necessidade;
- FTS5 indexa apenas campos necessários;
- decisões e revisões não são hard-deleted pela UI normal;
- deduplicação possui constraint ou transação, não apenas verificação em memória.

Estado de job:

```text
queued | running | completed | failed | cancelled
```

Na reinicialização, jobs interrompidos voltam para `queued` somente se idempotentes.

---

## 12. Estratégia de IA

Interfaces centrais:

```rust
trait CandidateExtractor {
    fn extract(&self, input: DecisionEvidence) -> Result<Vec<CandidateProposal>>;
}

trait AssessmentGenerator {
    fn assess(&self, input: AssessmentInput) -> Result<AssessmentDraft>;
}
```

Regras obrigatórias:

- o domínio não conhece nomes de fornecedores;
- `FakeCandidateExtractor` é determinístico e usado nos testes;
- a saída do modelo usa schema estruturado e validação estrita;
- texto do modelo nunca vira SQL, path ou comando;
- chamadas externas permanecem desligadas até consentimento;
- o usuário vê que categorias de dados podem sair da máquina;
- cada execução registra profile, adapter, modelo, política, timestamps, hashes das entradas e resultado;
- segredos são armazenados no mecanismo seguro do sistema operacional, não no SQLite em texto puro;
- falha, rate limit ou indisponibilidade do provedor não bloqueiam a captura nem a UI;
- uma recomendação de IA nunca confirma uma decisão.

Adapters previstos pela interface:

```text
FakeProvider
OpenAIProvider
OpenAICompatibleProvider
OpenCodeGatewayProvider
```

OpenCode gateway é orquestrador, não “modelo”. A implementação inicial deve selecionar apenas um provider real de menor esforço, além do fake; não implementar todos antecipadamente.

---

## 13. Privacidade e segurança

O aplicativo processará código e conversas potencialmente sensíveis. Segurança não é fase futura.

### Obrigações do MVP

- somente loopback;
- autenticação bearer na API local;
- token não registrado em logs;
- validação de path após canonicalização;
- projeto precisa estar registrado/permitido;
- proteção contra path traversal e symlinks inesperados;
- body limits e timeouts;
- conteúdo sensível omitido de logs por padrão;
- redaction configurável antes de persistir/enviar;
- API de IA sem envio implícito;
- nenhum reasoning privado capturado por padrão;
- exportação sempre intencional e com preview;
- diagnóstico exportado deve ser sanitizado;
- dependências auditadas com `cargo-deny` e `cargo-audit`.

### Modelo de ameaça mínimo

Documentar e testar:

- processo local não autorizado tenta postar captura;
- site no navegador tenta atingir localhost;
- envelope aponta para caminho fora de projeto;
- payload enorme ou malformado;
- segredo aparece em diff ou mensagem;
- arquivo de outbox é adulterado;
- adapter envia o mesmo evento várias vezes;
- app cai entre persistência e agendamento;
- modelo retorna campos extras ou conteúdo hostil.

---

## 14. Observabilidade e erros

- logs estruturados com correlation ID por captura/job;
- níveis e retenção configuráveis;
- métricas locais: capturas aceitas/rejeitadas, deduplicação, tempo de job, candidatos por captura e ações da inbox;
- conteúdo de mensagens/diffs nunca em log normal;
- mensagens de erro da UI dizem o que ocorreu, impacto e ação possível;
- falha da integração deve ser visível sem bloquear o restante do app;
- panic em job não deve derrubar a janela;
- nenhuma telemetria remota no MVP.

---

## 15. Plano de implementação com gates

Não implementar tudo num único salto. Cada etapa deve fechar com testes e demonstração.

### Gate 0 — Spike descartável de GPUI

Objetivo: reduzir o maior risco técnico antes de acoplar o produto.

Entregas:

- janela Quiet Glass;
- navigation rail, lista virtualizada com 10 mil itens e painel de detalhe;
- teclado, focus-visible e teste básico com Narrator;
- SQLite lido fora da UI thread;
- job em background atualizando progresso;
- servidor Axum local autenticado;
- build/empacotamento Windows;
- medição de startup, memória e scroll.

Gate de saída: registrar uma ADR aceitando GPUI ou substituir apenas a camada de UI. O spike não deve virar arquitetura definitiva por inércia.

### Gate 1 — Fundação do workspace

- workspace e regras de dependência;
- tokens/primitives do design system;
- migrations e repositories mínimos;
- use cases de Projects;
- jobs persistidos;
- tracing e configuração;
- CI com fmt, clippy, testes e auditoria.

### Gate 2 — Captura vertical

- tipos Rust e JSON Schema;
- API local/discovery/token;
- adapter OpenCode mínimo;
- outbox atômica;
- ingestão, receipt, deduplicação e checkpoint;
- tela de diagnóstico;
- teste E2E: OpenCode fixture → captura persistida.

### Gate 3 — Extração

- filtro determinístico de relevância;
- `CandidateExtractor` fake;
- perfil real de IA configurável;
- validação estruturada;
- retries/backoff seguros;
- proveniência do assessment;
- fixtures de decisões e não decisões.

### Gate 4 — Decision Inbox

- lista/detalhe/diff;
- editar, confirmar, rejeitar e adiar;
- decisões e revisões;
- busca FTS5;
- exportação manual;
- estados vazios, loading e erro;
- fluxo completo por teclado.

### Gate 5 — Hardening e dogfood

- instalador/build reproduzível;
- migrations de upgrade;
- crash recovery de jobs/outbox;
- teste com sessões reais durante uma semana;
- medir ruído, latência e tempo de revisão;
- corrigir falhas críticas e registrar aprendizados.

Ao final do Gate 5, o MVP está concluído. Biblioteca, MCP, grafo e assistente começam somente numa especificação posterior.

---

## 16. Estratégia de testes

### Unitários

- transições de estado de Candidate e Decision;
- regra de idempotência;
- fingerprints;
- redaction;
- limites e validação;
- filtros determinísticos de relevância;
- serialização de contratos.

### Banco

- migrations em banco vazio;
- upgrade da versão anterior;
- rollback transacional por falha;
- constraints e deduplicação concorrente;
- FTS5 e revisão versionada.

### Contrato

- JSON Schema Rust ↔ TypeScript;
- fixtures de versões suportadas do OpenCode;
- eventos repetidos, fora de ordem e incompletos;
- payloads inválidos e grandes.

### Integração

- API autenticada e rejeição sem token;
- bind somente em loopback;
- app fechado → outbox → app aberto → receipt;
- capture → job → candidate;
- provider indisponível sem perda de captura.

### E2E

- registrar projeto;
- importar uma sessão fixture;
- receber candidato;
- abrir evidência/diff;
- editar e confirmar;
- encontrar decisão na busca;
- exportar sem tocar no repositório.

### UI e acessibilidade

- navegação completa por teclado;
- focus-visible;
- labels acessíveis;
- screenshots dos estados principais;
- contraste e fallback sem blur;
- lista grande sem travamento perceptível;
- Narrator nos fluxos centrais.

Testes que chamam provedor pago não rodam na suíte padrão. Use fake e fixtures; smoke tests externos são opt-in.

---

## 17. Critérios de aceite finais

O MVP só pode ser considerado pronto quando todos forem verdadeiros:

1. O usuário consegue cadastrar um projeto local.
2. Uma sessão OpenCode suportada produz captura sem pausar ou bloquear o agente.
3. Com o desktop fechado, a captura permanece na outbox e é importada depois.
4. Repetir o mesmo evento não duplica artefato nem candidato.
5. O processamento pesado não congela a interface.
6. O extractor fake permite executar todo o fluxo offline e de forma determinística.
7. Um provider real pode ser configurado sem acoplar o domínio.
8. Nenhum dado é enviado externamente antes de consentimento e configuração.
9. O usuário pode inspecionar evidência e diff antes de confirmar.
10. O usuário pode editar, confirmar, rejeitar e adiar.
11. Confirmar não cria arquivo nem commit no repositório.
12. Decisões confirmadas aparecem na lista, busca e detalhe com proveniência.
13. Exportar exige ação explícita e preview.
14. Reiniciar o app não perde decisões, captures ou jobs recuperáveis.
15. A API escuta somente em loopback, exige token e limita payloads.
16. O fluxo principal funciona por teclado e possui foco visível.
17. A UI segue Quiet Glass sem cores ou glass improvisados.
18. Logs e diagnóstico não expõem conteúdo sensível por padrão.
19. `cargo fmt`, `clippy`, testes e auditorias configuradas passam.
20. A documentação explica instalação, integração, privacidade, recovery e limitações conhecidas.

---

## 18. Decisões arquiteturais registradas

### AD-01 — Desktop local como produto principal

Necessário para arquivos locais, privacidade, múltiplos projetos, índice e experiência navegável. Plugin é integração satélite.

### AD-02 — Rust + GPUI

Escolhido por performance, binário local e controle visual. GPUI é pré-1.0; por isso existe Gate 0 e a UI fica isolada.

### AD-03 — Monólito modular

Evita custo de distribuição interna e mantém fronteiras suficientes para evolução.

### AD-04 — SQLite como fonte local estruturada

Adequado ao single-user local, transacional e com FTS5. Grafo é projeção, não banco separado.

### AD-05 — OpenCode como primeiro adapter

Oferece eventos, mensagens e session diff. Não entra no domínio e poderá ser substituído ou acompanhado por outros adapters.

### AD-06 — Turno como unidade técnica inicial

`sessionID + userMessageID + respostas filhas + diff` evita tentar inferir uma task perfeita. Episodes podem agrupar turnos depois.

### AD-07 — Captura automática e revisão assíncrona

Confirmação durante o trabalho criaria atrito. A inbox nunca bloqueia sessões.

### AD-08 — Diff é evidência, não origem exclusiva

Turnos expressam intenção; diff confirma parte implementada. Mudanças externas e decisões não implementadas podem existir.

### AD-09 — Sem daemon no MVP

Quando o app fecha, a outbox absorve capturas. Um serviço permanente só será considerado por necessidade comprovada.

### AD-10 — Decisões ficam locais

Confirmar grava no banco do app. Arquivo/Markdown é exportação explícita; commit é sempre responsabilidade do usuário.

### AD-11 — IA configurável por perfil

Interfaces por capacidade evitam lock-in. Fake determinístico é obrigatório; providers reais são adapters.

### AD-12 — Sem envio externo implícito

Consentimento, preview da política e proveniência são requisitos do primeiro provider real.

### AD-13 — Quiet Glass é o design system canônico

Tokens e primitives precedem telas. A referência visual controla hierarquia, densidade e material.

### AD-14 — Busca lexical antes de semântica

FTS5 e filtros devem provar seus limites antes de embeddings, vector DB ou GraphRAG.

### AD-15 — Agentes não confirmam decisões

Automação observa, extrai e aconselha. Autoridade decisória permanece humana.

---

## 19. Regras operacionais para agentes implementadores

1. Leia integralmente este documento, `../design/design-system-quiet-glass.md` e `../arquitetura/stack-e-arquitetura-rust-gpui.md` antes de alterar código.
2. Execute os gates na ordem. Não antecipe funcionalidades futuras.
3. Quando uma escolha relevante não estiver resolvida, escreva uma ADR curta em vez de escondê-la na implementação.
4. Não substitua tecnologias aprovadas por preferência pessoal sem evidência e decisão explícita.
5. Preserve a independência do domínio em cada PR.
6. Faça slices verticais pequenos e demonstráveis.
7. Teste a falha, reinicialização e idempotência, não apenas o caminho feliz.
8. Não use conteúdo real sensível em fixtures ou screenshots.
9. Nunca registre tokens, prompts completos, conversas ou diffs nos logs padrão.
10. Não “melhore” o escopo adicionando cloud, grafo, RAG, MCP ou novos adapters.
11. Não criar abstração sem necessidade observada; preferir módulos profundos com interface pequena.
12. Cada entrega deve incluir testes, documentação relevante e instruções de verificação.
13. Alterações visuais devem ser comparadas com a imagem canônica.
14. Se GPUI bloquear um requisito essencial no Gate 0, reporte evidência antes de construir workarounds extensos.
15. Nunca tomar uma decisão de produto em nome do usuário apenas para destravar código.

### Definition of done por tarefa

- comportamento solicitado implementado;
- testes proporcionais ao risco;
- erros e estados vazios tratados;
- nenhuma regressão de privacidade;
- UI responsiva e acessível quando aplicável;
- logs úteis e sanitizados;
- documentação/ADR atualizada quando a decisão mudou;
- comandos de validação executados e resultados registrados;
- nenhuma mudança fora do escopo misturada.

---

## 20. Questões deliberadamente adiadas

Estas perguntas não bloqueiam o MVP e não devem ser respondidas prematuramente:

- qual será o formato final do grafo de contexto;
- qual motor de busca semântica usar;
- como importar e licenciar livros/PDFs/sites;
- protocolo completo de Context Packs e MCP;
- como medir decision quality longitudinalmente;
- como reconstruir decisões históricas;
- como operar multi-repo e conflitos complexos;
- se haverá sync, conta ou colaboração;
- quais outros agentes terão adapters;
- como o assistente fará counterfactual reasoning;
- se algum componente headless permanente será necessário.

Registrar evidências durante o dogfood, sem transformar hipóteses futuras em infraestrutura atual.

---

## 21. Resultado esperado

Ao abrir o MVP, o usuário encontra um ambiente desktop calmo e rápido. Seus projetos mostram discretamente possíveis decisões observadas durante sessões do OpenCode. Ele abre uma proposta, entende qual intenção e qual diff a originaram, corrige a interpretação se necessário e confirma apenas o que merece memória duradoura. Depois, consegue reencontrar a decisão e sua justificativa sem vasculhar chats ou histórico Git.

Esse é o produto mínimo. Se esse ciclo não for útil e agradável, biblioteca, grafo e assistente não compensarão. Se ele funcionar, as mesmas decisões confirmadas se tornam a fundação confiável para toda a visão futura.
