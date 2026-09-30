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
  - `get_decision(reference)`: aceita `D:bbbbcccc`, `bbbbcccc` ou o id completo; devolve status, versão, pergunta, escolha, motivo, premissas, "reconsiderar quando", escopo, consequências, relações (com a pergunta da outra decisão), claims derivadas vigentes e a quantidade de evidências.
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

## Evidências

- `apps/mcp-server`: testes do protocolo com backend falso e do binário real por stdio contra uma API local falsa (sessão completa, token enviado, app fechado).
- `local-api/tests/ingest.rs`: rotas `/v1/agent/*` com store real (abrir referência, busca, 404/400/403/422/401, consulta fora do log).
- `storage-sqlite/tests/agent_access.rs`: referência curta, relações, claims, busca sem deduplicação e sem registro.

## Pendências registradas

- Confirmar o formato de configuração MCP na versão real do OpenCode.
- Incluir `xemnas-mcp.exe` no pacote ZIP (`tools/package.ps1`), validando no Windows.
- Registro de acesso dos agentes (`agent_access_log` do documento de arquitetura) para auditoria, se o uso real pedir.
- Filtros por componente e arquivo nas ferramentas quando o grafo de entidades existir (`docs/roadmap/fase-4/00-plano-grafo-de-entidades.md`).
