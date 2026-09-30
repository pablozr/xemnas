# Arquitetura proposta para o aplicativo desktop

> **Histórico / não vigente para o MVP:** esta proposta recomenda .NET + Avalonia, mas foi substituída pela decisão aprovada em `../../produto/MVP-SPEC.md`: Rust + GPUI. Ela permanece apenas como contexto de alternativas consideradas; não deve orientar implementação.

## Recomendação

Construir o produto como um aplicativo desktop local em **.NET + Avalonia**, organizado como um monólito modular. A interface e o motor rodam no mesmo processo; uma API local pequena existe somente para integrações externas.

Essa escolha deve ser confirmada por um protótipo curto que meça inicialização, memória, navegação de listas grandes, renderização do grafo e publicação do instalador. A meta é escolher com dados do produto, não com benchmarks genéricos.

Se o produto decidir permanecer exclusivamente no Windows, **WinUI 3** passa a ser a alternativa principal. Se a prioridade absoluta for eficiência nativa e a maior maturidade multiplataforma, aceitando C++ e maior complexidade, **Qt 6** é a alternativa mais forte.

## Um backend, mas não dois aplicativos

Internamente existe uma separação lógica entre frontend e backend:

```text
Desktop UI
    ↓ chamadas em memória
Application Services
    ↓
Domain Core
    ↓
Infrastructure
    ├── SQLite
    ├── arquivos e PDFs
    ├── índice de busca
    ├── modelos/LLMs
    └── Git somente leitura por padrão
```

Isso não exige que a interface chame uma API HTTP para cada ação. UI, regras e persistência vivem no mesmo executável e se comunicam por interfaces e chamadas em memória. O resultado é menor latência, menos falhas operacionais e desenvolvimento mais simples.

## API apenas na fronteira externa

```text
OpenCode plugin ──┐
Outros adapters ──┼──> API/IPC local ──> Application Services
Agentes via MCP ──┘
```

A API local serve para:

- receber pacotes capturados por adapters;
- consultar o estado do aplicativo;
- pesquisar decisões e contexto;
- solicitar um Context Pack;
- permitir que um agente de implementação consulte conhecimento autorizado.

Ela deve escutar apenas em loopback, usar token local e ser iniciada somente enquanto o aplicativo estiver aberto. Se estiver fechada, o adapter grava a captura numa outbox durável e o app a importa posteriormente.

Para agentes, a interface preferida no futuro é um servidor MCP local iniciado sob demanda. Ele pode conversar diretamente com os serviços da aplicação ou por IPC, sem tornar a API pública na rede.

## Processos do sistema

### MVP

```text
Processo 1: aplicativo desktop
  UI + motor + workers + SQLite + API local opcional

Processo 2: OpenCode
  plugin fino + outbox de fallback
```

Não há daemon permanente. Tarefas como ingestão, indexação, OCR e avaliações são executadas por workers internos enquanto o aplicativo estiver aberto, com estado persistido para continuar depois de uma interrupção.

### Futuro, somente se necessário

Um serviço separado pode surgir se houver necessidade comprovada de indexação contínua com a UI fechada, modelos locais pesados, isolamento de falhas ou vários clientes simultâneos. A arquitetura modular permite extrair esse processo depois sem pagar esse custo no MVP.

## Estrutura recomendada do repositório

```text
/
├── src/
│   ├── Desktop/                 # janela, navegação, views e view-models
│   ├── Application/             # casos de uso e orquestração
│   ├── Domain/                  # decisões, claims, evidências e regras puras
│   ├── Infrastructure/          # SQLite, filesystem, busca, PDF, Git e IA
│   ├── Integration.Contracts/   # envelopes e contratos versionados
│   ├── LocalApi/                # entrada autenticada para adapters
│   └── McpServer/               # consulta por agentes; pode vir depois
├── adapters/
│   └── opencode/                # plugin fino, sem regras do domínio
├── tests/
│   ├── Domain.Tests/
│   ├── Application.Tests/
│   ├── Infrastructure.Tests/
│   ├── Contract.Tests/
│   └── Desktop.Tests/
├── docs/
│   ├── architecture/
│   └── adr/
└── tools/                       # scripts de build, migração e diagnóstico
```

## Módulos funcionais do monólito

```text
Capture
  recebe e normaliza turnos, diffs e outros Source Artifacts

Decision Intelligence
  gera candidatos, assessments e relações

Decision Governance
  Inbox, confirmação, correção, rejeição e histórico

Project Context
  claims, snapshots, escopo, validade e Context Packs

Knowledge Library
  ingestão, metadados, extração, indexação e citações

Retrieval
  busca lexical, filtros e posteriormente busca semântica

Agent Access
  API local, MCP, autorização e auditoria de consultas
```

Cada módulo possui seus próprios casos de uso e tabelas lógicas, mas todos usam inicialmente um único SQLite. Não há microserviços.

## Persistência local

```text
Application data/
├── app.db                 # entidades, relações, estados e jobs
├── library/               # originais ou cópias autorizadas dos documentos
├── extracted/             # texto derivado e metadados
├── indexes/               # índices reconstruíveis
├── outbox/                # capturas ainda não processadas
└── logs/                  # logs locais com retenção controlada
```

- SQLite armazena decisões, claims, relações, candidatos e jobs.
- Busca textual pode começar com FTS5.
- Arquivos grandes permanecem no filesystem, referenciados pelo banco.
- Índices derivados podem ser apagados e reconstruídos.
- Vetores e GraphRAG não são necessários para o primeiro MVP.

## Fluxo de uma captura

```text
1. OpenCode conclui um checkpoint de sessão.
2. O adapter coleta turno, respostas relacionadas e diff.
3. Se o app estiver aberto, envia um Capture Envelope à API local.
4. Se estiver fechado, grava o envelope na outbox.
5. O módulo Capture normaliza e deduplica o pacote.
6. Decision Intelligence cria ou atualiza candidatos.
7. A UI apresenta os candidatos na Decision Inbox.
8. A confirmação humana cria uma Engineering Decision local.
```

Nenhuma etapa faz commit ou altera automaticamente o repositório do projeto.

## Fluxo de consulta de um agente

```text
1. O agente descreve a tarefa e o escopo.
2. O acesso MCP pede um Context Pack.
3. Retrieval seleciona somente decisões, claims e fontes aplicáveis.
4. O pacote retorna conteúdo curto, citações e validade temporal.
5. O agente usa o pacote na implementação.
6. A consulta é registrada para explicar qual contexto foi fornecido.
```

## Regra de dependência

```text
Desktop ────────┐
LocalApi ───────┼──> Application ──> Domain
McpServer ──────┘          ↑
                    Infrastructure implementa portas
```

O Domain não conhece Avalonia, SQLite, OpenCode, HTTP, MCP ou modelos de IA. Essa fronteira é o que permite trocar a interface, extrair um serviço ou adicionar adapters sem reescrever o produto.

## Decisão que ainda precisa de prova

Antes de congelar Avalonia, construir um vertical slice com:

- lista virtualizada de milhares de decisões;
- uma tela de detalhe;
- um grafo pequeno;
- SQLite e FTS5;
- importação de PDF em background;
- endpoint local autenticado;
- build self-contained e, se viável, Native AOT;
- medição de cold start, memória ociosa e fluidez.

O protótipo deve ser descartável e servir apenas para validar a stack.
