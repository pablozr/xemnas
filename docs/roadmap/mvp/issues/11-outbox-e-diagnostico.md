# 11 — Gate 2: outbox, importação e diagnóstico

**What to build:** Preservar capturas com desktop fechado e importá-las com diagnóstico posteriormente.

**Blocked by:** 09 — Gate 2: ingestão local segura e idempotente; 10 — Gate 2: Adapter OpenCode no caminho online.

**Status:** done

- [x] Escrita/transição são atômicas e pending nunca desaparece silenciosamente.
- [x] E2E comprova importação, rejeição diagnosticável e deduplicação.

## Decisões registradas

- Contrato fixado (§7.2/§5.18 + stack 320–378): raiz `XEMNAS_OUTBOX_DIR` (default `<data>/outbox`); filename `sha256hex(idempotency_key).json`; transições só por rename; `*.tmp` = `<nome>.<pid>.<contador>.tmp`; retenção age-based apenas de `accepted` (`XEMNAS_OUTBOX_ACCEPTED_RETENTION_DAYS`, default 7, faixa 1..=3650, `checked_mul`, fallback 7 — nunca panicar a composition root).
- Rust `crates/application/src/outbox.rs`: `drain(root, api, accepted_retention) -> DrainReport {accepted, rejected: BTreeMap, pending_remaining, sending_recovered, pruned}`; claim `pending → sending` ANTES de ler (arquivo substituído nunca é lido velho e arquivado novo); renames nunca apagam destino (`move_no_replace`: destino regular → `Ok(false)` sem tocar, não-regular → `Err`); recuperação `sending → pending` no início; `*.tmp` órfãos limpos.
- Rejeição grava diagnóstico `{filename, code, detail, rejected_at}` via tmp único (`unique_temporary`) ANTES de remover o reivindicado; sem sucesso → item volta a `pending`; códigos: `invalid_json`, `schema_invalid` (+`schema_detail` seguro), `deserialize_failed`, `invalid_fingerprint`, `artifact_id_conflict`, `ingest_not_found`. Conteúdo do envelope nunca vai para `rejected/`.
- Idade de aceitação carimbada (`stamp_modified` após rename) + conjunto `accepted_now` do drain: item pendente antigo nunca é podado no mesmo drain em que é aceito.
- Transientes `Forbidden`/`Storage` voltam a `pending`, sem teto de tentativas nesta fase.
- `application` sem `tracing` (regra RUST-003): log na composition root (`drain_outbox` em `main.rs`), erro do drain só loga.
- TS: `writePending` nunca lança (tmp + `renameSync`); `unavailable` → outbox; 4xx → `failures.json` + `rejected_4xx`; 2xx → checkpoint. CLI `send-fixture`: `outcome:"outbox"` só com persistência real confirmada (resultado do writer + `existsSync`); caso contrário `outcome:"failed"`, `outbox_path:null`, `error` sanitizado, exit 1. Novo campo `error` só no ramo de falha do stdout JSON.
- E2E `tests/e2e/capture-outbox.ps1`: fases A (fechado → outbox) / B (abre → accepted + rejected `invalid_json` sem segredo, capture_id preservado) / C (rodando → sent, dedup não altera contagens); fases B/C em `try/finally` com `Stop-Process` + limpeza de discovery/token.
- Review: round 1 REPROVADO (5 bloqueantes: claim antes da leitura, retenção podando recém-aceito, overflow de retention, CLI enganoso, E2E sem finally) → corrigidos; round 2 REPROVADO (tmp do rejected fora do contrato) → `unique_temporary` + testes; round 3 **APROVADO**.

## Evidências

- `cargo test` (pacotes backend, sem `architecture`/`desktop-gpui`): **118 pass, 1 ignorado, 0 fail** (relinks intermitentes do SAC `os error 4551` — ambiente, retry resolve).
- `cargo test -p application --locked` pós-fix final: **35 pass** (11 do módulo `outbox`).
- clippy via `RUSTC_WORKSPACE_WRAPPER=clippy-driver` ( `cargo-clippy.exe` bloqueado por SAC): 0 diagnósticos em `application, local-api, storage-sqlite, integration-contracts, desktop-gpui --all-targets`.
- `cargo fmt --all -- --check`: diffs apenas em `apps/**` (sessão de front); `rustfmt --check` limpo em `outbox.rs`.
- `cargo audit` = 0 (3 allow-listados), `cargo deny check` = 0 (11 duplicate warnings aceitos); `Cargo.lock` intocado; zero dependências novas.
- `npm test` (adapters/opencode): **78/78**; `npm run build` = 0.
- E2E `tests/e2e/capture-outbox.ps1` = **RESULT=PASS** (31 asserts; última rodada pós-fix final); E2E `tests/e2e/jobs-recovery.ps1` = **RESULT=PASS** (8 asserts, regressão).
- Guard ARCH-001: 11/12 — única falha é `apps/desktop-gpui/src/ui/icons.rs:61` (código da sessão de front, fora do escopo).

## Dívida registrada

- Rejeitados não têm retenção (diagnóstico pode acumular indefinidamente).
- Transientes sem teto de tentativas (pode reprocessar item permanentemente quebrado a cada boot).
- `tool_summary` gravado como JSON cru no envelope (shape a confirmar no dogfood 18/20).
- Shapes/porta do OpenCode reais a confirmar em 18/20 (E2E usa fixture e servidor fake 127.0.0.1:0).
- Desvios da sessão de front (não bloqueantes, reportados ao usuário): `cargo fmt` vermelho em `apps/**`; guard `no_color_literals_outside_tokens` (`icons.rs:61` `#ffffff`); teste próprio `rendered_text_surfaces_meet_wcag_aa` vermelho.

## Pendências resolvidas depois do fechamento

- Retenção de rejeitados: diagnósticos em `rejected/` são removidos após 30 dias (`DrainPolicy::rejected_retention`) — `8067a34`.
- Teto de transientes: item recusado como `Forbidden` em 5 drains vai intacto para `stalled/`; `outbox::retry_stalled` o devolve a `pending/`; falha de storage não conta — `8067a34`.
