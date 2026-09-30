# 10 — Gate 2: Adapter OpenCode no caminho online

**What to build:** No idle, reconciliar mensagens desde checkpoint, coletar turno/diff limitados e enviar captura.

**Blocked by:** 08 — Gate 2: contrato versionado de captura; 09 — Gate 2: ingestão local segura e idempotente.

**Status:** done

- [x] Adapter retorna rapidamente e não executa IA.
- [x] Reasoning e segredos são omitidos por padrão, sem perda no checkpoint.

## Decisões registradas

**Rust (checkpoint na ingesta):**
- Nova tabela `adapter_checkpoints` (migration `0004_create_adapter_checkpoints.sql`): PK `(adapter, session_id)`, colunas `message_id/capture_id/observed_at/updated_at`, FK `capture_id → capture_receipts(capture_id) ON DELETE CASCADE`.
- Upsert `ON CONFLICT(adapter, session_id) DO UPDATE` executado **dentro da transação `IMMEDIATE`** de `insert_capture`, após o job e antes do commit (`crates/storage-sqlite/src/captures.rs`). Captura rejeitada ⇒ rollback ⇒ zero checkpoints.
- DTO de escrita `CaptureWrite` ganhou `checkpoint: CaptureCheckpointRecord` (montado no caso de uso em `application::captures`); storage segue sem depender de `integration-contracts`.
- **Replay de `idempotency_key` não reescreve o checkpoint** (rollback antes do upsert) — high-water mark nunca régrede; idempotente por construção.
- Sem endpoints novos de leitura de checkpoint (diagnóstico é o ticket 11) e sem handler `analyze_capture` (ticket 12).

**TypeScript (adapter online):**
- Plugin fino em `adapters/opencode/src/`: `config.ts` (discovery/token/envs), `checkpoint.ts` (arquivo + escrita atômica tmp+rename, avança **só após 2xx**), `opencode.ts` (fonte HTTP real/fake com shapes documentados), `redact.ts` (reasoning removido, segredos mascarados, limites, diff→hunks), `envelope.ts` (uuid-v7, validação no schema versionado, sha256 lowercase), `client.ts` (Bearer + `Idempotency-Key` + timeout; classifica 2xx/4xx/unavailable), `index.ts` (export `Plugin`, debounce cancelável, hook enfileira e retorna).
- Gatilhos: `session.status` idle **e** `session.idle` (compatibilidade, §7.1). Debounce default 1000 ms.
- `Idempotency-Key` no formato `opencode:{session}:{message}:{diff_hash}`; header e envelope idênticos; **capture_id gravado vem da receipt do servidor** (replay preserva o original).
- Paginação de mensagens via header `Link: rel="next"` com **cursor opaco do servidor** (`before` cru retornaria 400); para no `last_message_id`, no fim ou em `maxMessagePages` (default 10); filtro `id > checkpoint`.
- Concorrência: **um trabalho em volo por `session_id`** com coalescência (idle durante execução ⇒ exatamente 1 reexecução) + guarda monotônica no `advance` (checkpoint nunca régrede).
- Shapes reais do OpenCode confirmados na doc/SDK (`GET /session/:id/message` → `{info: Message, parts: Part[]}`; `GET /session/:id/diff` → `FileDiff {file, before, after}`); hunks gerados por LCS de linhas (limite 4M células, fallback prefixo/sufixo, contexto 3, limitado por `maxDiffBytes`).
- Suposições configuráveis (registradas): `OPENCODE_URL` default `http://127.0.0.1:4096`; envs `XEMNAS_ADAPTER_*` (timeout 5000 ms, max artifacts 64, max bytes 65536, max diff 32768, message limit 200, max pages 10); `tool_summary` = JSON compacto `{"tool","status"}`.
- **Zero dependências novas** (`node:crypto`, `node:fs`, `fetch` nativo); `@opencode-ai/plugin` não adotado — tipo mínimo local citando a doc oficial.
- App indisponível (conn refused/timeout) ⇒ sem avanço de checkpoint, sem crash, **sem outbox** (ticket 11).

## Evidências

| check | resultado |
|---|---|
| `cargo test --workspace --locked` | exit 0 — 125 pass + 1 ignored (24 suítes); rodada do implementador: 126+1 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | diff apenas em `apps/.../fonts.rs` (sessão de UI paralela, fora do escopo deste ticket) |
| `cargo audit` | exit 0 (3 warnings allow-listados) |
| `cargo deny check` | exit 0 (11 `warning[duplicate]` do GPUI) |
| `npm ci` + `npm test` | exit 0 — **62/62** (35+10 novos de adapter + 17 do contrato 08) |
| `tests\e2e\jobs-recovery.ps1` | **RESULT=PASS** 8/8 com binário real |
| Review round 1 | REPROVADO — 3 bloqueantes (parser vs. shapes reais; `limit` sem paginação; concorrência/receipt no replay) |
| Review round 2 | **APROVADO** — bloqueantes resolvidos, 62/62 revalidados, sem regressões |

Testes novos cobrem: parser com payloads literais documentados; paginação de 3 páginas alcançando o checkpoint; gatilhos idle vs. ativo; retorno rápido do hook; envelope validado no schema + fingerprint sha256; checkpoint só após 2xx; retry com a MESMA `Idempotency-Key`; coalescência de idles concorrentes; guarda monotônica; replay grava `capture_id` da receipt; app indisponível não avança; 4xx registra falha; reasoning/segredos removidos e mascarados; logs sem conteúdo sensível.

## Dívida registrada

- **Outbox** (`XEMNAS_ADAPTER_STATE_DIR/failures.json` + resultado `unavailable`): persistência, retry e `rejected/` com diagnóstico — ticket 11.
- **Diagnóstico de checkpoint/outbox na UI**: ticket 11.
- **Shapes/porta do OpenCode em runtime real**: paths, cursor `Link` e porta default `4096` confirmados na doc/SDK e isolados em `opencode.ts`/`config.ts` — confirmar contra um OpenCode real no dogfood (tickets 18/20).
- **E2E OpenCode fixture → captura persistida** no app real: ticket 11.
- `tool_summary` como JSON compacto — se a extração (12) preferir texto simples, é mudança local em `redact.ts`.
