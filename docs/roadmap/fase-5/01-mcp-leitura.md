# Fase 5 — MCP somente leitura para agentes

**O que foi construído:** o binário `xemnas-mcp`, um servidor MCP sobre stdio que deixa o agente abrir uma decisão pela referência curta e buscar o contexto do projeto, sempre pelo app aberto.

**Status: done (backend); configuração a confirmar no OpenCode real.**

**Autorização:** pedido pelo usuário em 2026-09-30 como continuação da injeção mínima (a ADR-0003 deixava o MCP para depois). Implementação própria do protocolo, sem SDK, escolhida por ter só três métodos e nenhuma dependência nova.

## Como funciona

```text
agente (OpenCode, Claude Code…)
  └─ inicia xemnas-mcp (stdio, JSON-RPC por linha)
       └─ POST /v1/agent/decision | /v1/agent/search  (loopback + token de sessão)
            └─ AgentAccess no app aberto (somente leitura)
```

- **Ferramentas (definições com menos de 1.500 bytes, para o custo fixo por turno ficar baixo):**
  - `get_decision(reference)` (também aceita o alias `ref`; `reference` vence): aceita `D:bbbbcccc`, `bbbbcccc` ou o id completo; devolve status, versão, pergunta, escolha, motivo, premissas, "reconsiderar quando", escopo, consequências, relações (com a pergunta da outra decisão), claims derivadas vigentes e a quantidade de evidências.
  - `search_context(query, path?)`: o mesmo bloco compacto da injeção, sem deduplicação (é uma leitura pedida pelo agente); com `path`, o que o mapa do projeto liga ao arquivo vem primeiro (ADR-0005).
  - `file_context(path)`: decisões vigentes e regras ligadas aos componentes do arquivo pelo mapa do projeto (ADR-0005), via `POST /v1/agent/file`.
- **Legenda do formato:** vai no campo `instructions` do `initialize` e nas descrições das ferramentas; o bloco injetado continua sem custo extra.
- **Projeto:** `--project <dir>` ou o diretório em que o agente iniciou o servidor. Diretório não cadastrado vira erro de ferramenta explicativo.
- **App fechado:** as ferramentas respondem "o app está fechado"; não há modo headless (spec).
- **Privacidade:** a consulta do agente nunca vai para log; leituras do MCP não entram no registro de injeções.

## Compilar

```powershell
cargo build --release --locked -p mcp-server   # gera target\release\xemnas-mcp.exe
```

## Ligar no agente

OpenCode (`opencode.json` do projeto ou global; formato da documentação pública do OpenCode, a confirmar na versão em uso):

```json
{
  "mcp": {
    "xemnas": {
      "type": "local",
      "command": ["C:/caminho/para/xemnas-mcp.exe"],
      "enabled": true
    }
  }
}
```

Claude Code:

```powershell
claude mcp add xemnas -- C:/caminho/para/xemnas-mcp.exe
```

## Hooks do Claude Code

O mesmo binário atende os hooks do Claude Code (injeção e captura), sem processo residente. Todos terminam com código 0 e não imprimem nada em caso de erro (no máximo uma linha sanitizada no stderr, sem texto de prompt ou conteúdo):

- `xemnas-mcp hook prompt` (`UserPromptSubmit`): lê o JSON do evento, chama `POST /v1/context` (timeout de 300 ms; app fechado sai em silêncio) com o prompt e até 8 arquivos recentes (`Edit`, `Write`, `MultiEdit`, `NotebookEdit`, `Read`, lidos só do fim do transcript) e imprime `hookSpecificOutput.additionalContext` quando há contexto.
- `xemnas-mcp hook stop` (`Stop`): lê o transcript de forma incremental e envia um Capture Envelope por **troca substantiva** a `POST /v1/captures` (`Idempotency-Key`, timeout de 2 s). Com o app fechado ou erro 5xx grava em `outbox/pending`. Até 20 capturas por execução.
  - **Dobra de turnos triviais.** Turno trivial: sem `tool_use`, prompt de até 80 caracteres (após trim) e texto do assistente de até 600. Turnos triviais consecutivos são dobrados na captura do turno anterior: prompts e respostas são anexados, em ordem e separados por linha em branco, aos artefatos `user_text` e `assistant_text` (mesmos limites de tamanho; até 10 turnos dobrados por captura). `message_id` e chave de idempotência continuam os do primeiro turno. Assim "vamos de X?" / "pode" fica junto da proposta. Um trivial sem turno anterior (o primeiro da sessão) é capturado sozinho.
  - **Retenção do mais novo.** O Stop não envia o grupo mais recente, porque o próximo turno pode dobrar nele; ele sai quando existe um turno não trivial depois. Uma captura já enviada nunca é reenviada com mais conteúdo: triviais que chegam depois dela abrem o próprio grupo (e só dobram nele enquanto ele estiver pendente).
  - **Ponto de retomada** em `<dados>/adapter/claude-code/<sessão>.json` (um arquivo por sessão; o id vira `[A-Za-z0-9-]`, o resto `_`, até 128 caracteres): o deslocamento do início do grupo retido e o uuid do último turno enviado. A execução seguinte relê o grupo retido inteiro e os novos; sem grupo retido (após `session-end`) o deslocamento é o do último turno enviado, pulado pelo uuid. Se o app responder 403 (diretório que não é projeto registrado), as capturas são passadas por cima, sem outbox e sem novas chamadas naquela execução.
- **Worktrees do git.** Um worktree de um projeto registrado conta como esse projeto: o app resolve `canonical_path` (captura, contexto e `/v1/agent/*`) pelo `git rev-parse --git-common-dir`, que é o mesmo do repositório registrado (`application::repo_identity`; git sem shell, timeout de 2 s, cache de 5 min por diretório, inclusive para quem não é repositório). Outro repositório continua recusado (403). No `stop`, uma edição fora do `cwd` que caia em outro worktree do mesmo repositório é mantida, com o caminho relativo à raiz desse worktree (`git rev-parse --show-toplevel`, uma consulta por diretório distinto); arquivos fora de qualquer worktree do repositório continuam descartados.
- `xemnas-mcp hook session-end` (`SessionEnd`, entrada `{session_id, transcript_path, cwd, hook_event_name, reason}`): igual ao `stop`, mas envia tudo, inclusive o grupo retido (repete até esvaziar o atraso). Sai sempre com código 0 e sem stdout.

`~/.claude/settings.json` (os hooks rodam junto com os que já existirem; acrescente aos arrays):

```json
{
  "hooks": {
    "UserPromptSubmit": [
      { "hooks": [{ "type": "command", "command": "C:/caminho/para/xemnas-mcp.exe hook prompt", "timeout": 5 }] }
    ],
    "Stop": [
      { "hooks": [{ "type": "command", "command": "C:/caminho/para/xemnas-mcp.exe hook stop", "timeout": 15 }] }
    ],
    "SessionEnd": [
      { "hooks": [{ "type": "command", "command": "C:/caminho/para/xemnas-mcp.exe hook session-end", "timeout": 15 }] }
    ]
  }
}
```

Código em `apps/mcp-server/src/hook/`; testes em `src/hook/*` e `tests/hook.rs` (inclui latência do `hook prompt`, dobra, retenção, `session-end` e leitura só do fim do transcript no segundo Stop).

## Evidências

- `apps/mcp-server`: testes do protocolo com backend falso e do binário real por stdio contra uma API local falsa (sessão completa, token enviado, app fechado).
- `local-api/tests/ingest.rs`: rotas `/v1/agent/*` com store real (abrir referência, busca, 404/400/403/422/401, consulta fora do log).
- `storage-sqlite/tests/agent_access.rs`: referência curta, relações, claims, busca sem deduplicação e sem registro.

## Pendências registradas

- Confirmar o formato de configuração MCP na versão real do OpenCode.
- Incluir `xemnas-mcp.exe` no pacote ZIP (`tools/package.ps1`), validando no Windows.
- Registro de acesso dos agentes (`agent_access_log` do documento de arquitetura) para auditoria, se o uso real pedir.
- Filtros por componente e arquivo nas ferramentas quando o grafo de entidades existir (`docs/roadmap/fase-4/00-plano-grafo-de-entidades.md`).
