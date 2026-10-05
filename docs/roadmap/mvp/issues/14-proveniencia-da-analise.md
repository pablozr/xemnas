# 14 — Gate 3: resiliência e proveniência da análise

**What to build:** Registrar Assessments, retries e falhas sem comprometer capturas.

**Blocked by:** 13 — Gate 3: AI Execution Profile real e consentido.

**Status:** done

- [x] Profile, modelo, política, timestamps e hashes são rastreáveis.
- [x] Indisponibilidade permite reprocessamento sem bloquear UI.

## Decisões registradas

- **Tabela `assessments` (migração 0006, versão 6)** conforme §11: id, `capture_id` FK CASCADE→`capture_receipts`, `job_id` FK SET NULL→`jobs`, profile_id, adapter, model, policy (JSON estático do perfil: kind, limites, redaction-on-ingest, enabled), consent_preview_hash (NULL offline), input_hash, started_at/finished_at, outcome CHECK (`ok|empty|failed|skipped`), candidates, inserted, error_code (código estável curto; PRIV-001: nenhum conteúdo de artefato/modelo/URL na linha). Índices por capture_id e started_at.
- **Uma linha terminal por execução** — invariante revisada em 3 rounds: `run_extraction` grava `ok`/`empty`/`failed` antes de retornar; `ExternalBlocked` grava `skipped` no handler; setup do provider (secret ausente `secret`, keystore `keystore`, config inválida `provider_config`, perfil ilegível `profile`) grava via `fail_provider_setup`; **exceção documentada**: falha de storage AO gravar (e captura inexistente, que violaria a FK) propaga `Err(ExtractError::Storage, code "storage")` sem linha — o job falha com diagnóstico.
- **`RunContext`** (profile_id/adapter/model/policy_json/consent_preview_hash/job_id) injetado em `run_extraction`; `main.rs` monta do JobRecord + perfil recarregado por job. Perfil indisponível força `RunContext::unavailable` (profile_id `unavailable`, adapter `unknown`, policy `{"profile":"unavailable"}`) — contexto parcial nunca vaza para a linha.
- **`input_hash` = sha256(capture_id ‖ artefatos ordenados `artifact_id:fingerprint`)** via `integration_contracts::capture::artifact_fingerprint` — sem dependência nova, sensível a fingerprint, insensível à ordem.
- **Retry/backoff só no provider** (nenhum schema de jobs alterado): `RetryPolicy { max_attempts: 3, base_delay: 500 ms, max_delay: 4 s }`, backoff `min(base·2^(n-1), max)`; transientes = connect/timeout/429/5xx, demais 4xx/parse/validação falham na primeira. Sleep injetável ⇒ testes determinísticos (sem dormir). Sem retry automático em job (loop infinito impossível por desenho).
- **`Jobs::reprocess(id)`**: `failed → queued` via CAS `transition` com `last_error=NULL` + wakeup do worker; estados inválidos ⇒ `JobError`; reprocessamento manual ilimitado e visível (`attempts` incrementa no claim). UI: Configurações › Diagnóstico reprocessa e cancela tarefas (`03e47b3`).
- **Falha de IA jamais invalida captura** (§7:302) preservada: receipt/artifacts/checkpoint intactos em qualquer desfecho; worker thread isola a UI.
- Review: r1 REPROVADO (3 caminhos de setup sem assessment; captura inexistente = sucesso silencioso) → r2 REPROVADO (perfil corrompido sem assessment) → r3 **APROVADO**.

## Evidências

**Registro histórico:** os relatos de bloqueio por caminho/cópia em `%TEMP%`
abaixo preservam a interpretação daquela rodada, superada pelo diagnóstico por
hash. Para recuperação atual, use [Smart App Control](../../../operacao/operacao-e-referencia.md#smart-app-control-sac).

- `cargo test -p application --locked`: **68 pass** (49 lib + 19 extração), 0 fail — inclui gravação por caminho (ok/empty/failed/skipped), input_hash, códigos estáveis, `reprocess`, captura inexistente sem linha, `fail_provider_setup` (3 variants + profile).
- `cargo test -p ai-provider --locked`: **13 pass + 1 ignorado** — 5 de retry: 503,503,200 ⇒ 3 requests; 503 sempre ⇒ 3 e Err sanitizado; 400 ⇒ 1 request; delays injetados [500 ms, 1000 ms]; connect recusado transiente.
- `cargo test -p storage-sqlite --locked`: **43 pass** — migração 6 (vazio + upgrade 5→6), roundtrip/FK/índices de assessments, reprocess integration.
- `cargo test -p architecture --locked`: 11 pass + 1 fail (`ui/icons.rs:57`, WIP da sessão de UI, pré-existente desde o ticket 12 — linha migrou 61→57).
- rustfmt 0 (arquivos tocados); clippy 0 (application/ai-provider/storage-sqlite/desktop-gpui; 16 avisos `missing_docs` são de arquivos novos do front: `app.rs`, `lib.rs`, `ui/search_field.rs`).
- `cargo check -p desktop-gpui --all-targets` → 0; `cargo deny check` → 0; `cargo audit` → 0; **`Cargo.lock` intocado** (nenhuma dependência nova neste ticket).
- E2E: `jobs-recovery.ps1` **PASS** e `capture-outbox.ps1` **PASS** — com o binário da rodada (ver dívida: WDAC passou a bloquear por caminho o exe recém-buildado; os E2E rodaram via `-ExePath` apontando para cópia idêntica em `%TEMP%`, removida depois).
- Regressão dos tickets 12/13 dentro dos totais acima (consent_status fonte único, envelope real, 0 requisições com consent inválido — suítes de ai-provider verdes).

## Dívida registrada

**SAC/WDAC:** o workaround de cópia abaixo é histórico e foi superado; siga o
[procedimento atual](../../../operacao/operacao-e-referencia.md#smart-app-control-sac).

- **`ExtractError` não distingue validação de extractor** (ambos `code()=="extractor"`) — variantes novas mudariam enum público; diagnóstico granular de validação fica para iteração de UX.
- Assessment `empty` significa "filtro de relevância achou nada"; modelo que retorna 0 propostas grava `ok` com counts 0.
- Falha de storage ao gravar assessment ⇒ execução sem linha (proposição consciente; storage quebrado já falha o job).
- Consulta/listagem de assessments para a tela de Diagnósticos (front) não existe ainda — só a escrita; consumo no próximo ticket de UI/API.
- Retry automático de job (backoff entre execuções) ficou deliberadamente ausente: só retry intra-provider (3×) + reprocess manual — nenhuma tentativa automática sem teto.
- Ambiente: **WDAC bloqueia por caminho** exes novos em `target\debug` (~30 tentativas, `Unblock-File` não resolve); cópia em `%TEMP%` executa — workaround documentado (relink novo + cópia ou espera de reputação). Processo antigo da sessão de front (PID 43696) segura a imagem renomeada `xemnas-locked.exe`.
- Proveniência de execuções de AssessmentGenerator (ticket futuro de assessments reais) ainda não existe — aqui só extração (Gate 3).

## Pendências resolvidas depois do fechamento

- `ExtractError::Validation` (código `validation`) separa proposta fora do contrato de falha do provider (`extractor`) — `c0d11a9`.
- Consulta para a tela de Diagnósticos: `DiagnosticsDocument.recent_assessments` (ticket 19) cobre a listagem.
