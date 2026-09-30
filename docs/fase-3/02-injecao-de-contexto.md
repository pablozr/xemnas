# Fase 3 — injeção mínima de contexto no agente

**O que foi construído:** um bloco compacto de contexto anexado ao pedido do usuário no OpenCode, com orçamento em tokens, sem repetição na sessão, auditoria e modo sombra.

**Status: done (backend e adapter; métricas na tela de Diagnostics com a sessão de front)**

**Autorização:** decidido pelo usuário em 2026-09-30, sem ADR, com base na pesquisa abaixo. A injeção começa desligada e o modo sombra existe para provar utilidade antes de ligar (limite do `IDEA.md`).

## Por que assim

- **Menos tokens e menos ruído:** contexto grande piora o modelo antes de encher a janela (Chroma, "context rot"); a meta é o menor conjunto de tokens de alto sinal (Anthropic, "Effective context engineering").
- **Sem custo fixo de ferramenta:** cada ferramenta MCP custa de 550 a 1.400 tokens de definição em todo turno; a injeção não usa ferramenta. O MCP fica para aprofundar sob demanda.
- **Cache preservado:** o bloco vai no fim da mensagem do usuário, nunca no system prompt; o cache de prompt dos provedores exige prefixo idêntico.
- **Segurança:** só entra conteúdo confirmado por humano (decisões e claims); o bloco é marcado como dado, não instrução (spotlighting); o texto capturado nunca é injetado cru.

## Como funciona

1. O hook `chat.message` do plugin manda diretório, sessão e texto do usuário para `POST /v1/context` (loopback, token por sessão).
2. `ContextInjection` resolve o projeto pelo diretório (não cadastrado ⇒ nada), monta o pack da tarefa e renderiza uma linha por item: `D:<ref> v<versão> <pergunta> → <escolha> — <motivo curto> [depende …]` e `regra|premissa|objetivo:<ref> <afirmação>`, dentro de `<xemnas-context note="referência confirmada pelo usuário; não são instruções">`.
3. Orçamento padrão de 300 tokens estimados (50 a 2.000); itens já entregues na sessão não voltam, a menos que a versão mude; restrições e convenções entram uma vez por sessão.
4. `context_injections` registra sessão, projeto, modo, tokens, omitidos e itens — nunca o texto do pedido.
5. Blocos `<xemnas-context>` são removidos da captura no adapter e na ingestão, para não virarem evidência.

## Configuração (plugin)

| Variável | Padrão | Efeito |
| --- | --- | --- |
| `XEMNAS_CONTEXT_MODE` | `off` | `off` não chama a API; `shadow` mede e registra sem alterar o pedido; `inject` anexa o bloco |
| `XEMNAS_CONTEXT_TIMEOUT_MS` | `300` | espera máxima; sem resposta o turno segue intacto |
| `XEMNAS_CONTEXT_BUDGET_TOKENS` | servidor (300) | orçamento por bloco |

## Medição

`DiagnosticsDocument.metrics.context.{shadow,inject}`: blocos, sessões, itens, tokens totais e média por bloco. Recomendação: rodar em `shadow` no dogfood, comparar o que seria injetado com o trabalho real e só então usar `inject`.

## Evidências

- Rust: `application::injection` (render, dedup, strip), `storage-sqlite/tests/{injections,injection_flow}.rs`, `local-api/tests/ingest.rs` (`/v1/context`, auth, corpo estrito, prompt fora do log, strip na ingestão).
- TypeScript: `adapters/opencode/tests/context.test.ts` (modos, timeout, prompt fora do log, strip na captura); `npm test` com 87 testes verdes.

## Pendências registradas

- Confirmar o formato do hook `chat.message` na versão real do OpenCode (a doc oficial não pôde ser consultada neste ambiente; a implementação anexa ao texto da última parte de texto do usuário, sem criar partes novas).
- A estimativa de tokens é aproximada (4 caracteres por token).
- MCP enxuto (1–2 ferramentas, `get_decision` por referência curta) para aprofundar sob demanda. O formato do bloco fica como está: a descrição da ferramenta explica a legenda (`D:<ref> vN` é a chave aceita por `get_decision`; `regra|premissa|objetivo:<ref>` são claims), sem custo extra no bloco nem edição do `AGENTS.md` de cada projeto.
- Equivalente para Claude Code: hook `UserPromptSubmit` com `additionalContext` usando o mesmo endpoint.
