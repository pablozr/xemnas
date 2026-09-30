# 20 — Gate 5: dogfood e decisão de conclusão do MVP

**What to build:** Usar o produto por uma semana, medir atrito e decidir a conclusão do MVP.

**Blocked by:** 19 — Gate 5: recovery, segurança e operação.

**Status: blocked (backend pronto; faltam: semana de dogfood real, UI do front e decisão do usuário)**

- [x] Ruído, perdas, latência e tempo de revisão **têm fonte e registro prontos** (`DiagnosticsDocument.metrics` + [`docs/operacao/dogfood-log.md`](../../../operacao/dogfood-log.md)); números reais dependem do uso da semana; falhas críticas registradas na tabela do registro.
- [x] Critérios finais **reexecutados e auditoria registrada abaixo**; decisão de conclusão **pendente** (é decisão do usuário após o dogfood — §19.13: nunca decidir produto pelo usuário).

## Decisões registradas

- **Métricas de Gate 5 no diagnóstico (delta aprovado)**: seção `metrics` aditiva — `latency_capture_to_candidate_ms{samples,p50,p95}` (join capture↔candidato), `review_time_ms{samples,p50,p95}` (candidato→confirmação), `noise{decided_total,dismissed_ratio}`, `losses{assessments_failed,assessments_skipped,jobs_failed,outbox_rejected}`; percentis via `ORDER BY … OFFSET ROUND((n−1)·p)` sem crate extra; timestamps malformados descartados; vazio ⇒ `0/None`; sanitização cobre a seção (varredura de `SECRET-MARKER-*` no JSON inteiro).
- **README.md criado na raiz** (critério 20): instalação/build/pacote, mapa de dados (`XEMNAS_DATA_DIR` → `%LOCALAPPDATA%\xemnas`, `state/app.db`, `api-token`/`discovery.json`, outbox), integração do adapter, privacidade, recovery, limitações conhecidas, comandos de validação.
- **`docs/operacao/dogfood-log.md`**: metodologia de coleta (campo `metrics` + diário diário + tabela de falhas) e o formulário da decisão de conclusão — a semana só começa com UI utilizável e sessão OpenCode real.
- Auditoria de critérios: ver **Evidências**.

## Evidências — auditoria §17 (reexecutada em 2026-09-29)

| # | Critério | Status | Evidência |
| --- | --- | --- | --- |
| 1 | Cadastrar projeto local | ✅ backend / UI pronta | `Projects::register` (canonicaliza + dedup) com testes; tela `screens/projects` |
| 2 | Captura sem pausar o agente | ✅ | hook do adapter retorna imediato (debounce/background; `adapters/opencode/src/index.ts`), testes de contrato TS, E2E `capture-outbox` PASS |
| 3 | Desktop fechado → outbox → importa depois | ✅ | E2E `capture-outbox.ps1` (receipts 0 → drain → 1) |
| 4 | Repetição não duplica | ✅ | idempotency key + `UNIQUE` + replay E2E + `duplicate_artifact_id` rollback |
| 5 | Processamento pesado não congela | ✅ backend | ASYNC-001, worker de jobs, timeouts da API (testes); prova visual na UI = front |
| 6 | Fake extractor offline determinístico | ✅ | 19 testes de extração + fixtures (3 duráveis/9 triviais) |
| 7 | Provider real sem acoplar domínio | ✅ | crate `ai-provider` (13 testes) + guard `allowed_dependencies` |
| 8 | Nenhum dado externo sem consentimento | ✅ | gate `consent_status`/preview_hash, `choose_extractor`, provider tests (tickets 13/14) |
| 9 | Inspecionar evidência/diff antes de confirmar | ⏳ UI | backend: `Inbox::detail` (artefatos na ordem, diff_summary); tela = front |
| 10 | Editar/confirmar/rejeitar/adiar | ⏳ UI | backend: `Inbox::{adjust,confirm,reject,dismiss_batch,snooze,unsnooze}` (14 testes); tela = front |
| 11 | Confirmar não cria arquivo/commit | ✅ | `full_cycle` (snapshot do project dir antes/depois idêntico, sem `.git`) |
| 12 | Decisões em lista/busca/detalhe com proveniência | ⏳ UI | backend: `Decisions::{list,search,detail,revise}`; tela = front |
| 13 | Exportar exige ação explícita e preview | ✅ backend | `Export::preview` byte-idêntico, destino do chamador (10 testes); diálogo = front |
| 14 | Reinício não perde decisões/capturas/jobs | ✅ | E2E `jobs-recovery.ps1` + teste `restart_keeps_receipt_and_job` + migration transacional |
| 15 | Loopback + token + limites | ✅ | `local-api` 26 testes (401, Origin 403, 413, timeout, discovery) |
| 16 | Teclado e foco visível | ⏳ front | fora do escopo backend (diretiva do usuário) |
| 17 | Quiet Glass sem improviso | ⏳ front | pendências do front reportadas: `ui/icons.rs:57` no guard (arch 11/12), teste WCAG vermelho, fmt com diffs em `apps/**` |
| 18 | Logs/diagnóstico sem conteúdo sensível | ✅ | token nunca logado, panic sanitizado, diagnóstico sanitizado por construção (8 marcadores) |
| 19 | fmt, clippy, testes, auditorias passam | ⏳ quase | reexecutado: clippy 0, deny 0, audit 0, testes verdes (abaixo) **exceto** arch 11/12 (UI) e `cargo fmt --all` com diffs dos arquivos da UI do front |
| 20 | Documentação (instalação/integração/privacidade/recovery/limitações) | ✅ | `README.md` + `docs/**` + ADRs 0001/0002 |

**Suíte completa reexecutada (comandos meus, 2026-09-29):** domain 4, application **102**, storage-sqlite **61**, local-api 26, integration-contracts 9 (+1 ignorado), telemetry 13, ai-provider 13 (+1 ignorado), desktop-gpui 12, architecture **11/1 falha** (`ui/icons.rs:57`, WIP do front — único teste vermelho do workspace); `cargo deny check` 0; `cargo audit` 0 (3 avisos allow-listados); E2Es `jobs-recovery`, `capture-outbox`, `install-clean` PASS. (Aviso de ambiente: o WDAC do host ocasionalmente bloqueia binários recém-buildados — contornado com retry/remoção, nunca por política.)

## Dívida registrada / bloqueios da conclusão

1. **Semana de dogfood real** — exige UI utilizável + sessão OpenCode real; registro pronto em `docs/operacao/dogfood-log.md`.
2. **UI do front** — critérios 9, 10, 12, 16, 17 e as telas Inbox/Decisions/Export/Diagnostics; contratos públicos entregues nos tickets 15–20 (`Inbox`, `Decisions`, `Export`, `Diagnostics` + tipos).
3. **Correções do front pendentes** — `ui/icons.rs:57` (guard), teste WCAG, `cargo fmt` em `apps/**`.
4. **Decisão de conclusão = usuário** (formulário no dogfood-log).
5. **Nada foi commitado** (working tree com ~67 arquivos alterados/novos desde o `Initial commit` 5fd8e9a) — commit exige autorização explícita do usuário.
6. Caminho do pacote release/ZIP sem validação local (WDAC no build release) — CI `package` cobrirá quando houver remote; sem remote, a CI não roda.
