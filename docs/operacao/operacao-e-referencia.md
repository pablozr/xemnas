# Operação e referência técnica

Execução, dados locais, variáveis de ambiente, integrações com OpenCode e Claude
Code, Context Pack, MCP, privacidade, recuperação, dogfood, limitações conhecidas
e validação. O README traz só a visão geral. Resultados das avaliações:
[avaliacoes.md](avaliacoes.md).

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
Os dados desaparecem ao fechar a janela.

Pacote distribuível (ZIP versionado em `dist\`):

```powershell
.\tools\package.ps1          # build release + staging verificado por SHA-256
.\tests\e2e\install-clean.ps1  # prova instalação/desinstalação limpas
```

**Onde ficam os dados** (tudo local):

| Item | Caminho |
| --- | --- |
| Diretório de dados | `XEMNAS_DATA_DIR` (se definido) ou `%LOCALAPPDATA%\xemnas` |
| Banco SQLite | `<dados>\state\app.db` (forward-only; [migrações na fonte](../../crates/storage-sqlite/src/store.rs)) |
| API local | somente loopback; token por sessão em `<dados>\api-token`; porta em `<dados>\discovery.json` |
| Outbox de capturas | `XEMNAS_OUTBOX_DIR` ou `<dados>\outbox` (`pending/`, `accepted/`, `rejected/`, `stalled/`) |
| Perfil de IA | `<dados>\settingsi-profile.json` (segredos ficam no keyring, não aqui) |
| Estado dos adapters | `<dados>dapter\` (OpenCode; `claude-code\<sessão>.json` guarda o ponto de retomada dos hooks) |

**Variáveis de ambiente `XEMNAS_*`:**

| Variável | Efeito |
| --- | --- |
| `XEMNAS_DATA_DIR`, `XEMNAS_OUTBOX_DIR` | Raiz de dados e da outbox (`crates/application/src/paths.rs`) |
| `XEMNAS_OUTBOX_ACCEPTED_RETENTION_DAYS` | Dias até a limpeza de `outbox/accepted/` |
| `XEMNAS_CLAUDE_CODE` | Caminho do executável do Claude Code quando não está no `PATH` (provedor de IA) |
| `XEMNAS_DISCOVERY`, `XEMNAS_ADAPTER_STATE_DIR`, `XEMNAS_ADAPTER_*` | Ajustes do adapter OpenCode (descoberta da API, estado, debounce, limites de mensagens/artefatos/diff, timeout); `XEMNAS_CONTEXT_TIMEOUT_MS` ajusta a injeção |
| `XEMNAS_PERF=1` | Registra o tempo de render das telas ([desempenho](../arquitetura/desempenho-e-escala.md)) |
| `XEMNAS_DEMO_*` (`SCALE`, `ARCH`, `APPROVAL`, `CLAUDE`, `CALIBRATION`, `STARTUP_ERROR`) | Cenários do modo `--demo` |
| `XEMNAS_BACKDROP`, `XEMNAS_GRAPH_RENDER` | Material da janela (`mica-alt`/`mica`/`acrylic`) e modo de render do grafo, para diagnóstico visual |
| `XEMNAS_DOGFOOD_DB`, `XEMNAS_DOGFOOD_CASES` | Entradas do teste ignorado `dogfood_context` (banco real e casos fora do repositório) |

## Integração com o OpenCode

O adapter em [`adapters/opencode/`](../../adapters/opencode/) é um plugin fino: disparo em idle → reconcile pela API pública do OpenCode → validação do **Capture Envelope** → `POST /v1/captures` (ou escrita na outbox quando o desktop está fechado). Ele não contém regras de domínio e nunca chama modelo.

**Captura automática (padrão).** O OpenCode entrega ao plugin um `client` já vinculado à instância, ao diretório e à autorização da sessão, com requisições em processo. O adapter usa esse client para ler mensagens e diffs — não é preciso configurar porta nem `OPENCODE_URL`. A paginação segue o cursor opaco do header `Link` (nunca sintetizado) no mesmo client, e uma falha do client é erro fatal: o adapter **não** cai para uma URL HTTP alternativa, para não capturar de outra instância. `OPENCODE_URL` só é usada como caminho legado/diagnóstico quando o plugin roda sem client utilizável (fixtures e testes).

```powershell
cd adapters/opencode
npm ci
npm test                      # testes de contrato e captura automática
npm run send-fixture          # envia fixture pela outbox (modo CLI)
```

Variáveis relevantes: `XEMNAS_DATA_DIR`, `XEMNAS_OUTBOX_DIR` e `OPENCODE_URL` (override **legado** da fonte HTTP de diagnóstico; padrão `http://127.0.0.1:4096`). A injeção de contexto é ligada por projeto no app (desligada, medir ou ativa) e acontece em dois momentos: no prompt e logo depois de cada edição de arquivo, com o que o mapa do projeto liga àquele arquivo (ADR-0005); o plugin só aceita `XEMNAS_CONTEXT_TIMEOUT_MS` como ajuste opcional (ver [contexto entregue aos agentes](../arquitetura/contexto-e-agentes.md#injeção-no-pedido)).

**Ativação (uma vez, sem publicar):** o OpenCode carrega plugins de arquivos locais — crie `~/.config/opencode/plugins/xemnas.ts` reexportando o build (`export { XemnasOpenCodeAdapter as Xemnas } from "<repo>/adapters/opencode/dist/src/index.js"`; caminho relativo a partir de `plugins/` é `../../../orca/projects/xemnas/...`). O wrapper deve ter **um único export** (o factory), para o OpenCode não registrar os exports utilitários do módulo. Reinicie a sessão do OpenCode após criar o arquivo.

Esse wrapper global é a integração instalada, não a fonte: alterações no adapter
pertencem a `adapters/opencode` neste repositório.

### Teste manual de captura

Roteiro para conferir a integração real; o MCP é somente leitura, então estar
conectado não prova que o plugin carregou nem que a extração funciona.

1. O projeto precisa estar acompanhado no mesmo caminho usado pela sessão, e o
   perfil de IA com consentimento vigente. Não use `--demo`: ele não inicia
   API nem workers.
2. Envie no OpenCode: "Decidi usar o identificador `XEMNAS-CAPTURA-TESTE-001`
   nos registros de teste, em vez de nomes livres, para localizar o candidato e
   distingui-lo de decisões reais. O escopo é só este teste. Registre a escolha e
   seu motivo."
3. Quando a resposta terminar, procure o identificador em **Revisão**, confira as
   evidências e rejeite o candidato de teste (ou confirme, para testar a passagem
   a **Decisões**). Um Markdown escrito isoladamente não é importado: o adapter
   captura textos e diffs do turno quando a sessão fica ociosa.
4. Se nada aparecer, o diagnóstico do adapter diz onde parou: `messages-unavailable`
   (não leu a conversa), `capture-rejected` (a API recusou; confira projeto e
   caminho), `outbox-pending` (guardada para depois), `capture-accepted` (chegou,
   sem provar extração). Captura recebida sem candidato: confira perfil de IA,
   consentimento e falhas de job no Diagnóstico. Atualize o plugin com
   `npm run build` em `adapters/opencode` e reinicie o OpenCode.

## Integração com o Claude Code

O mesmo binário `xemnas-mcp` traz os hooks (código em `apps/mcp-server/src/hook/`),
sem processo residente. Todos saem com código 0 e, em erro, imprimem no máximo uma
linha sanitizada no stderr:

- `hook prompt` (`UserPromptSubmit`): chama `POST /v1/context` (timeout de 300 ms;
  app fechado sai em silêncio) com o prompt e até 8 arquivos recentes do
  transcript, e devolve `additionalContext` quando há contexto.
- `hook stop` (`Stop`): lê o transcript de forma incremental e envia um Capture
  Envelope por troca substantiva a `POST /v1/captures`; com o app fechado, grava na
  outbox. O ponto de retomada fica em `<dados>dapter\claude-code\<sessão>.json`.
- `hook session-end` (`SessionEnd`): como `stop`, mas envia também o grupo retido.

Registro no `~/.claude/settings.json`, JSON e requisitos em
[MCP somente leitura](../arquitetura/contexto-e-agentes.md#mcp-somente-leitura). O Claude Code também pode
ser o provedor de IA do app (veja `XEMNAS_CLAUDE_CODE`).

## Context Pack (Fase 3)

O backend monta um **Context Pack** para uma tarefa: decisões vigentes e premissas/regras válidas numa data, escolhidas por busca lexical, com citações (decisão e versão, evidências, relações) e limite de tamanho. Exportar para Markdown ou JSON exige ação explícita e destino escolhido. Decisões podem ser substituídas sem apagar a anterior. No OpenCode, o plugin pode anexar ao pedido um bloco compacto (cerca de 300 tokens, sem repetir na sessão), desligado por padrão, ligado por projeto nas configurações do app, com modo sombra para medir antes de ativar. Detalhes e contrato para a UI: [Context Pack](../arquitetura/contexto-e-agentes.md#context-pack) e [ADR-0003](../arquitetura/adr/0003-fase-3-contexto-recuperavel.md).

## MCP para agentes (somente leitura)

`xemnas-mcp` é um servidor MCP sobre stdio com três ferramentas: `get_decision` (abre a decisão pela referência `D:xxxx` que aparece no bloco injetado), `search_context` (busca decisões vigentes e regras do projeto; aceita `path` de um arquivo envolvido) e `file_context` (o que o mapa do projeto liga a um arquivo, ADR-0005). Ele consulta o app aberto pela API local e nunca altera nada. Contrato e funcionamento: [contexto entregue aos agentes](../arquitetura/contexto-e-agentes.md#mcp-somente-leitura). O mesmo binário traz os hooks do Claude Code `hook prompt` (injeta contexto) e `hook stop` (captura os turnos, com outbox se o app estiver fechado), descritos em [hooks do Claude Code](../arquitetura/contexto-e-agentes.md#hooks-do-claude-code).

## Privacidade

- **Local por padrão.** Nenhum dado sai da máquina sem consentimento explícito e perfil configurado (§13 da spec).
- A API de IA só é usada com consentimento vigente (`preview_hash` verificado) e segredos no cofre do sistema (keyring) — nunca em texto no SQLite.
- Providers externos exigem HTTPS. HTTP é permitido apenas em IPs de loopback, como `http://127.0.0.1:11434/v1` ou `http://[::1]:11434/v1`; URLs com credenciais, query ou fragmento são rejeitadas e redirecionamentos não são seguidos. O consentimento inclui o endpoint completo: consentimentos anteriores à inclusão desse vínculo exigem nova aprovação.
- **Redação em duas camadas, antes de qualquer persistência:** o adapter mascara segredos (`adapters/opencode/src/redact.ts`) e o motor Rust reaplica as mesmas regras na ingestão (`crates/application/src/redact.rs`) para toda captura recebida pela API ou importada da outbox: blocos PEM de chave privada, linhas `TOKEN`/`API_KEY`/`SECRET`/`PASSWORD`/`ACCESS_KEY`/`AUTHORIZATION` e chaves `sk-`, `ghp_`, `github_pat_` e `xox?-`. O fingerprint gravado é o do conteúdo redigido.
- Logs nunca contêm token bearer, prompts, conversas ou diffs; falhas de job são sanitizadas; o **diagnóstico exportado é sanitizado por construção** (só estrutura — sem conteúdo de artefatos, decisões, caminhos ou credenciais).
- Exportação de decisão exige ação explícita, preview e destino escolhido pelo usuário; confirmar não escreve nada no repositório e o produto nunca faz commit.

## Recovery

- Jobs interrompidos voltam para `queued` na reinicialização quando idempotentes; capturas na outbox são importadas depois (com desktop fechado inclusive) sem duplicar (chaves de idempotência + dedup por constraint). Um item de projeto não cadastrado (recusa `Forbidden`) é definitivo: vai direto para `rejected/` (`project_not_registered`) no primeiro drain; falha de storage segue tentando. Itens já parados em `stalled/` por versões antigas só voltam para `pending/` por ação explícita (`outbox::retry_stalled`); diagnósticos em `rejected/` são removidos após 30 dias.
- `cargo test` inclui testes de crash/restart; E2Es: `tests\e2e\jobs-recovery.ps1`, `tests\e2e\capture-outbox.ps1`, `tests\e2e\install-clean.ps1`.

## Limitações conhecidas

- A redação é por padrões conhecidos (mesmas regras do adapter): um segredo em formato não reconhecido ainda é persistido. Os arquivos em `outbox/accepted/` guardam o envelope como o adapter o escreveu, até a retenção removê-los.
- Busca é lexical (FTS5) — sem embeddings/vector graph (spec: provar filtros antes de embeddings).
- Builds bit-a-bit reproduzíveis não são prometidos (timestamps Windows); o caminho `--locked` + CI é o mesmo.
- Cross build (`pwsh -File tools\build-windows-cross.ps1`): só faz sentido em **release** — em debug o GPUI resolve os shaders HLSL em runtime pelo `CARGO_MANIFEST_DIR` do container, caminho inexistente no Windows (panic `os error 3`). O script gera o `shaders_bytes.rs` no host com o `fxc.exe` da SDK e o container o copia para o `OUT_DIR` antes de compilar.

- Testes de provedor pago são opt-in; a suíte padrão usa fake/fixtures.
- Estado de conclusão do MVP: [ticket 20](../roadmap/tickets/20-dogfood-e-conclusao.md).

## Dogfood automático

[`dogfood-log.md`](dogfood-log.md) traz uma tabela diária (capturas, candidatos,
confirmações, ruído, latência, tempo de revisão, injeções, consultas MCP, perdas)
gerada do banco local, só leitura, apenas com agregados: nunca perguntas, escolhas,
caminhos ou nomes de projeto. A tarefa agendada do Windows `xemnas-dogfood-report`
roda todo dia às 23:30 `python tools/dogfood-report.py --write`, que refaz o bloco
entre `<!-- dogfood:auto:start -->` e `<!-- dogfood:auto:end -->` (idempotente).
Sem `--write`, imprime o bloco. `--db` e `--doc` trocam banco e diário. O teste
ignorado `crates/storage-sqlite/tests/dogfood_context.rs` reexecuta consultas reais
contra uma cópia do banco, com casos guardados fora do repositório.

## Smart App Control (SAC)

O Windows pode bloquear binários novos não assinados e DLLs de proc-macro.
Sintomas incluem `os error 4551`, `E0463` durante compilação e falhas `uv_spawn`
ao iniciar uma ferramenta. `E0463` e `uv_spawn` isolados não comprovam SAC:
confira o arquivo envolvido e os eventos de Code Integrity no Windows antes de
atribuir a causa. Se `rg` estiver bloqueado, use `git grep` (exemplos no
[mapa do código](../arquitetura/mapa-do-codigo.md#busca-no-windows)).

O veredito é **por hash e fica em cache**. Copiar o mesmo arquivo para `%TEMP%`
ou reextrair o ZIP não muda o hash; isso não é procedimento de recuperação.
Medição em out/2026: mesmo hash bloqueado em `target\release` e `%TEMP%`;
relinks liberados após 1 ciclo em release e 2 em debug, sem garantia de repetição.

Para tentar um rebuild, identifique primeiro o artefato e seu pacote a partir do
erro. Encerre apenas o processo que mantém esse artefato aberto. Faça uma alteração
legítima ou um clean restrito ao pacote (`cargo clean -p <pacote> --release` para
release; omita `--release` para debug) e refaça o comando de build original com
`--locked`. Confirme o novo hash com `Get-FileHash -Algorithm SHA256 <arquivo>`:
rebuild pode produzir os mesmos bytes e repetir o bloqueio. Se ainda bloquear,
registre erro, hash e evento para investigação; não automatize tentativas infinitas.
Não apague artefatos/fingerprints alheios nem mude a política de segurança como
parte desse procedimento. Desligar SAC é decisão explícita do usuário.

## Desenvolvimento (validação)

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo deny check
cargo audit
python -m unittest discover -s tools -p test_check_doc_links.py
python tools/check-doc-links.py
```

O checker valida destinos locais de links Markdown inline, imagens e definições
de referência em arquivos rastreados de `docs/` e README/AGENTS da raiz. Ignora
código cercado, código inline, URLs externas e âncoras isoladas; remove fragmentos
e query e decodifica URLs. Exige destino existente e versionado, inclusive para
diretórios. Não valida âncoras, HTTP, HTML, labels aninhados ou destinos multilinha.
Para validação focada: `python tools/check-doc-links.py --files docs/README.md`.

CI (`.github/workflows/ci.yml`): `docs` (testes do checker e links locais),
`quality` (fmt, clippy, testes, audit, deny), `contract` (testes TS do adapter)
e `package` (ZIP como artifact).
## Revisão consultiva do conhecimento

Em Contexto › Revisar conhecimento (também na paleta Ctrl K), **Verificar
localmente** consulta apenas o conhecimento registrado. Não indexa documentos,
não escreve dados e não chama IA. **Revisar com IA…** exige confirmação inline
do envio de textos selecionados de decisões, regras, propostas e vínculos ao
provedor configurado; código e pasta não são enviados. O consentimento vigente
em Configurações continua obrigatório e é validado pelo caso de uso.

Resultados são efêmeros e consultivos: tensão não comprova conflito, citações
não comprovam a inferência e ausência de achados não garante consistência.
Confira cobertura, omissões e achados descartados. Falha, indisponibilidade ou
resposta inválida da IA preservam verificações locais. Cancelamento é
cooperativo: uma chamada HTTP em curso pode terminar antes de liberar nova
execução. Trocar projeto cancela e descarta respostas antigas. A demonstração
usa um adaptador sintético em memória, sem rede ou cofre.
