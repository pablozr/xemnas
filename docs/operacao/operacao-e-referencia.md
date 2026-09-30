# Operação e referência técnica

Detalhes que saíram do README principal: execução, dados locais, integração com o
OpenCode, Context Pack, MCP, privacidade, recuperação, limitações conhecidas e
validação. O README traz só a visão geral.

## Instalação e execução

**Requisitos:** Windows; Rust estável (só para buildar a partir do código).

```powershell
cargo build --release --locked -p desktop-gpui --bin xemnas
.\target\release\xemnas.exe
```

Para explorar o layout com candidatos fictícios:

```powershell
cargo run --locked -p desktop-gpui --bin xemnas -- --demo
# Prévia na menor janela suportada:
cargo run --locked -p desktop-gpui --bin xemnas -- --demo --compact
# Evidência extensa para conferir rolagem e virtualização:
cargo run --locked -p desktop-gpui --bin xemnas -- --demo --long-evidence
```

A demonstração usa um banco em memória e não inicia workers, API ou provedores.
Os dados desaparecem ao fechar a janela. Na tela atual, Revisão permite ler
candidatos e evidências do projeto selecionado, ajustar, confirmar, rejeitar e
adiar/retomar candidatos. A contagem da aba considera toda a fila do projeto.
Detalhes mantém as propriedades e a remoção do acompanhamento.

Pacote distribuível (ZIP versionado em `dist\`):

```powershell
.\tools\package.ps1          # build release + staging verificado por SHA-256
.\tests\e2e\install-clean.ps1  # prova instalação/desinstalação limpas
```

**Onde ficam os dados** (tudo local):

| Item | Caminho |
| --- | --- |
| Diretório de dados | `XEMNAS_DATA_DIR` (se definido) ou `%LOCALAPPDATA%\xemnas` |
| Banco SQLite | `<dados>\state\app.db` (migrations forward-only, versão atual 14) |
| API local | somente loopback; token por sessão em `<dados>\api-token`; porta em `<dados>\discovery.json` |
| Outbox de capturas | `XEMNAS_OUTBOX_DIR` ou `<dados>\outbox` (`pending/`, `accepted/`, `rejected/`, `stalled/`) |

## Integração com o OpenCode

O adapter em [`adapters/opencode/`](adapters/opencode/) é um plugin fino: disparo em idle → reconcile pela API pública do OpenCode → validação do **Capture Envelope** → `POST /v1/captures` (ou escrita na outbox quando o desktop está fechado). Ele não contém regras de domínio e nunca chama modelo.

**Captura automática (padrão).** O OpenCode entrega ao plugin um `client` já vinculado à instância, ao diretório e à autorização da sessão, com requisições em processo. O adapter usa esse client para ler mensagens e diffs — não é preciso configurar porta nem `OPENCODE_URL`. A paginação segue o cursor opaco do header `Link` (nunca sintetizado) no mesmo client, e uma falha do client é erro fatal: o adapter **não** cai para uma URL HTTP alternativa, para não capturar de outra instância. `OPENCODE_URL` só é usada como caminho legado/diagnóstico quando o plugin roda sem client utilizável (fixtures e testes).

```powershell
cd adapters/opencode
npm ci
npm test                      # testes de contrato e captura automática
npm run send-fixture          # envia fixture pela outbox (modo CLI)
```

Variáveis relevantes: `XEMNAS_DATA_DIR`, `XEMNAS_OUTBOX_DIR` e `OPENCODE_URL` (override **legado** da fonte HTTP de diagnóstico; padrão `http://127.0.0.1:4096`). A injeção de contexto é ligada por projeto no app (desligada, medir ou ativa) e acontece em dois momentos: no prompt e logo depois de cada edição de arquivo, com o que o mapa do projeto liga àquele arquivo (ADR-0005); o plugin só aceita `XEMNAS_CONTEXT_TIMEOUT_MS` como ajuste opcional (ver [`docs/roadmap/fase-3/02-injecao-de-contexto.md`](docs/roadmap/fase-3/02-injecao-de-contexto.md)).

**Ativação (uma vez, sem publicar):** o OpenCode carrega plugins de arquivos locais — crie `~/.config/opencode/plugins/xemnas.ts` reexportando o build (`export { XemnasOpenCodeAdapter as Xemnas } from "<repo>/adapters/opencode/dist/src/index.js"`; caminho relativo a partir de `plugins/` é `../../../orca/projects/xemnas/...`). O wrapper deve ter **um único export** (o factory), para o OpenCode não registrar os exports utilitários do módulo. Reinicie a sessão do OpenCode após criar o arquivo.

## Context Pack (Fase 3)

O backend monta um **Context Pack** para uma tarefa: decisões vigentes e premissas/regras válidas numa data, escolhidas por busca lexical, com citações (decisão e versão, evidências, relações) e limite de tamanho. Exportar para Markdown ou JSON exige ação explícita e destino escolhido. Decisões podem ser substituídas sem apagar a anterior. No OpenCode, o plugin pode anexar ao pedido um bloco compacto (cerca de 300 tokens, sem repetir na sessão), desligado por padrão, ligado por projeto nas configurações do app, com modo sombra para medir antes de ativar. Detalhes e contrato para a UI: [`docs/roadmap/fase-3/01-context-pack-manual.md`](docs/roadmap/fase-3/01-context-pack-manual.md) e [ADR-0003](docs/arquitetura/adr/0003-fase-3-contexto-recuperavel.md).

## MCP para agentes (somente leitura)

`xemnas-mcp` é um servidor MCP sobre stdio com três ferramentas: `get_decision` (abre a decisão pela referência `D:xxxx` que aparece no bloco injetado), `search_context` (busca decisões vigentes e regras do projeto; aceita `path` de um arquivo envolvido) e `file_context` (o que o mapa do projeto liga a um arquivo, ADR-0005). Ele consulta o app aberto pela API local e nunca altera nada. Compilação e configuração no OpenCode e no Claude Code: [`docs/roadmap/fase-5/01-mcp-leitura.md`](docs/roadmap/fase-5/01-mcp-leitura.md).

## Privacidade

- **Local por padrão.** Nenhum dado sai da máquina sem consentimento explícito e perfil configurado (§13 da spec).
- A API de IA só é usada com consentimento vigente (`preview_hash` verificado) e segredos no cofre do sistema (keyring) — nunca em texto no SQLite.
- Providers externos exigem HTTPS. HTTP é permitido apenas em IPs de loopback, como `http://127.0.0.1:11434/v1` ou `http://[::1]:11434/v1`; URLs com credenciais, query ou fragmento são rejeitadas e redirecionamentos não são seguidos. O consentimento inclui o endpoint completo: consentimentos anteriores à inclusão desse vínculo exigem nova aprovação.
- **Redação em duas camadas, antes de qualquer persistência:** o adapter mascara segredos (`adapters/opencode/src/redact.ts`) e o motor Rust reaplica as mesmas regras na ingestão (`crates/application/src/redact.rs`) para toda captura recebida pela API ou importada da outbox: blocos PEM de chave privada, linhas `TOKEN`/`API_KEY`/`SECRET`/`PASSWORD`/`ACCESS_KEY`/`AUTHORIZATION` e chaves `sk-`, `ghp_`, `github_pat_` e `xox?-`. O fingerprint gravado é o do conteúdo redigido.
- Logs nunca contêm token bearer, prompts, conversas ou diffs; falhas de job são sanitizadas; o **diagnóstico exportado é sanitizado por construção** (só estrutura — sem conteúdo de artefatos, decisões, caminhos ou credenciais).
- Exportação de decisão exige ação explícita, preview e destino escolhido pelo usuário; confirmar não escreve nada no repositório e o produto nunca faz commit.

## Recovery

- Jobs interrompidos voltam para `queued` na reinicialização quando idempotentes; capturas na outbox são importadas depois (com desktop fechado inclusive) sem duplicar (chaves de idempotência + dedup por constraint). Um item recusado 5 vezes por projeto não cadastrado vai intacto para `stalled/` e só volta para `pending/` por ação explícita (`outbox::retry_stalled`); diagnósticos em `rejected/` são removidos após 30 dias.
- `cargo test` inclui testes de crash/restart; E2Es: `tests\e2e\jobs-recovery.ps1`, `tests\e2e\capture-outbox.ps1`, `tests\e2e\install-clean.ps1`.

## Limitações conhecidas

- A interface (telas Inbox/Decisions/Settings/Diagnostics, Context Pack e navegação por teclado) é entregue em paralelo — o backend dos fluxos já está completo.
- A redação é por padrões conhecidos (mesmas regras do adapter): um segredo em formato não reconhecido ainda é persistido. Os arquivos em `outbox/accepted/` guardam o envelope como o adapter o escreveu, até a retenção removê-los.
- Busca é lexical (FTS5) — sem embeddings/vector graph (spec: provar filtros antes de embeddings).
- `superseded` está modelado no schema, sem ação/UI ainda.
- Builds bit-a-bit reproduzíveis não são prometidos (timestamps Windows); o caminho `--locked` + CI é o mesmo.
- Cross build (`pwsh -File tools\build-windows-cross.ps1`): só faz sentido em **release** — em debug o GPUI resolve os shaders HLSL em runtime pelo `CARGO_MANIFEST_DIR` do container, caminho inexistente no Windows (panic `os error 3`). O script gera o `shaders_bytes.rs` no host com o `fxc.exe` da SDK e o container o copia para o `OUT_DIR` antes de compilar.
- Smart App Control pode bloquear binários novos não assinados (erros `os error 4551`, DLLs de proc-macro em `target\debug\deps`, `E0463` em compilações): apague o artefato bloqueado (e o fingerprint em `target\debug\.fingerprint`) e recompile — o novo arquivo costuma ganhar veredito novo. Desligar a SAC é decisão do usuário (é irreversível sem reinstalar o Windows).

- Testes de provedor pago são opt-in; a suíte padrão usa fake/fixtures.
- Estado de conclusão do MVP e dogfood: ver [`docs/roadmap/mvp/issues/20-dogfood-e-conclusao.md`](docs/roadmap/mvp/issues/20-dogfood-e-conclusao.md) e [`docs/operacao/dogfood-log.md`](docs/operacao/dogfood-log.md).

## Desenvolvimento (validação)

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo deny check
cargo audit
```

CI (`.github/workflows/ci.yml`): `quality` (fmt, clippy, testes, audit, deny), `contract` (testes TS do adapter) e `package` (ZIP como artifact).
