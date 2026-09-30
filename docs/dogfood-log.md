# Registro de dogfood (Gate 5, ticket 20)

Semana de uso real do produto para medir **ruído, perdas, latência e tempo de revisão** e decidir a conclusão do MVP (spec §16 Gate 5, §17 critérios finais). **Este arquivo é preenchido durante a semana; a decisão de conclusão é do usuário.**

## Como coletar

1. **Métricas automáticas** — durante o uso, abrir a tela Diagnostics (sessão de front) e exportar o diagnóstico; a seção `metrics` traz:
   - `latency_capture_to_candidate_ms` — p50/p95 de `received_at → created_at` do candidato (ruído de cadência);
   - `review_time_ms` — p50/p95 de `created_at → confirmed_at` (tempo de revisão humana);
   - `noise.dismissed_ratio` — fração de candidatos descartados entre os decididos (ruído percebido);
   - `losses` — `assessments_failed/skipped`, `jobs_failed`, `outbox_rejected` (perdas).
   Fonte: `Diagnostics::export()` (`application/src/diagnostics.rs`) — sanitizado, seguro para anexar ao registro.
2. **Diário qualitativo (uma linha por dia)** — data, o que aconteceu, o que incomodou, quase-acidentes, tempo gasto revisando.
3. **Falhas críticas** — cada falha: o que quebrou, severidade, correção aplicada (ou por que não corrigiu).

## Baseline de referência (teste automatizado — não é dogfood)

Registrado em 2026-09-29 como ponto de partida: suíte completa verde (application 102, storage-sqlite 61, local-api 26, ai-provider 13+1, desktop 12, contracts 9+1, telemetry 13, domain 4, arch 11/12), deny 0, audit 0, E2Es (`jobs-recovery`, `capture-outbox`, `install-clean`) PASS. Métricas reais começam em zero quando a UI estiver disponível.

## Diário da semana

| Data | Latência p50/p95 (ms) | Revisão p50/p95 (ms) | Ruído (dismissed_ratio) | Perdas | Observações / atritos | Falha crítica? |
| --- | --- | --- | --- | --- | --- | --- |
| _(início: ____-__-__)`*`* | | | | | | |

`*` A semana só começa quando houver UI utilizável (Inbox/Decisions) e ao menos uma sessão OpenCode real capturando.

## Falhas críticas

| # | Data | O quê | Severidade | Status | Correção / commit |
| --- | --- | --- | --- | --- | --- |
| | | | | | |

## Decisão de conclusão (preencher ao fim da semana)

- [ ] Ruído, perdas, latência e tempo de revisão registrados acima.
- [ ] Falhas críticas corrigidas (ou justificadas).
- [ ] Critérios finais §17 reexecutados (auditoria no ticket 20) — **os itens de UI (9, 10, 12, 16, 17) dependem da sessão de front**.
- [ ] Decisão: **concluir o MVP** / **não concluir — bloqueios:** ______________________
- _(Assinado por:)_ ____________________  Data: __________
