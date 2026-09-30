# 19 — Gate 5: recovery, segurança e operação

**What to build:** Endurecer jobs, outbox, API, logs e diagnóstico contra falhas e ameaças previstas.

**Blocked by:** 18 — Gate 5: distribuição e upgrade reproduzíveis.

**Status: done (aprovado)**

- [x] Crash, payload hostil, caller não autorizado, traversal, symlink, segredo e duplicação são testados.
- [x] Diagnóstico exportado é sanitizado.

## Decisões registradas

- **Auditoria ameaça→teste com lacunas fechadas por teste novo** (deltas em `local-api/tests/ingest.rs`): symlink/junction escape (`a_symlink_that_escapes_the_registered_project_is_forbidden` — host nega privilégio de symlink ⇒ fallback `mklink /J` junction; link que resolve para fora do projeto registrado ⇒ 403 e nada persiste; teste falha alto se não puder criar o link), projeto removido entre check e insert, schema-failure não loga conteúdo do cliente, fingerprint adulterado/duplicado, stalled body timeout, outbox drain sem duplicar. Demais ameaças já cobertas: 401+token nunca logado (:335), Origin 403 (:370), malformed 400/422 (:559), não-registrado/não-canonicalizável 403 (:600), body 413 (:721), replay/idempotency/rollback, restart, timeout, panic sanitizado (`jobs.rs`), `last_error` sem marcador de payload (`jobs.rs:1129`), redação no adapter TS (testes `redact.test.ts` etc.).
- **Diagnóstico exportado (`application/src/diagnostics.rs` + porta `DiagnosticsStore` + impl sqlite)**: `Diagnostics::new(store, settings).export() -> DiagnosticsDocument` — só estrutura (schema/versions, counts por status/outcome/state, outbox, recent_jobs com `error_code` mapeado `interrupted|failed` nunca `last_error` cru, recent_receipts, recent_assessments, ai_profile, runtime).
- **Sanitização por construção testada**: marcadores `SECRET-MARKER-{ARTIFACT,RATIONALE,JOB,TOKEN,OUTBOX,PATH,PW,QS}` plantados em conteúdo de artefato, justificativa de decisão, `last_error`, arquivo `api-token`, outbox, **nome do diretório do projeto** e **credenciais de endpoint** ⇒ nenhum aparece no JSON; campos estruturais corretos + round-trip serde.
- **Duas correções pós-review (r1)**: (1) `canonical_path` removido do documento → `project_id` opaco via `LEFT JOIN projects` (caminho pode conter usuário/segredo); (2) `endpoint_host` parser manual **só hostname** (descarta userinfo/porta/query/fragmento/path; sem host ⇒ `None`) — nunca credencial de URL.
- **Limitação registrada (não é bug desta rodada): não há redação no Rust** — a redação vive no adapter OpenCode (`adapters/opencode/src/redact.ts`, testes TS); `redaction_on_ingest` expressa a premissa da fronteira. Caller malicioso postando segredo cru em `/v1/captures` persiste o segredo. Motor de redação novo = escopo futuro (não inventado aqui).
- Review: r1 REPROVADO (path integral + userinfo no provider) → r2 **APROVADO**.

## Evidências

- `cargo test -p application --locked`: **101 pass** (50 lib + 6 decisions + 2 diagnostics + 10 export + 19 extração + 14 inbox).
- `cargo test -p storage-sqlite --locked`: **60 pass** (inclui teste de sanitização do diagnóstico com 8 marcadores).
- `cargo test -p local-api --locked`: **20 pass** (19 originais + symlink).
- `cargo test -p ai-provider --locked`: 13 + 1 ignorado; `architecture`: 11 + 1 (`ui/icons.rs:57`, WIP do front); `cargo test --workspace --locked` exit 101 única falha esperada.
- rustfmt 0; clippy 0 (application/storage-sqlite/local-api + desktop check 0); `cargo deny check` 0; `cargo audit` 0; **`Cargo.lock` intocado**.
- **E2E `jobs-recovery.ps1` PASS + `capture-outbox.ps1` PASS**.

## Dívida registrada

- Tela Diagnostics (botão exportar diagnóstico): feita em Configurações › Diagnóstico (`03e47b3`, 30/09/2026); contrato: `Diagnostics::new(store, settings)`, `export()`, tipos `DiagnosticsDocument/DiagnosticsError` + structs de seção reexportadas.
- Redação de conteúdo somente no adapter (limite de confiança da fronteira — acima).
- `recent_assessments.error_code` é a coluna de código curto como armazenada (valor hostil escrito ali passaria; não é conteúdo substantivo — contrato de código).
- `Diagnostics::new` toma `(store, settings)` (perfil/segredo fora da porta de storage) e `export()` é `Result`.

## Pendências resolvidas depois do fechamento

- Redação no motor Rust: `application::redact` reaplica as regras do adapter em toda captura (API e outbox) antes de persistir; o fingerprint gravado é o do conteúdo redigido — `bcbd6af`. Continua limitada a padrões conhecidos.
