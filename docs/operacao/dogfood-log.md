# Registro de dogfood (Gate 5, ticket 20)

Semana de uso real do produto para medir **ruído, perdas, latência e tempo de revisão** e decidir a conclusão do MVP (spec §16 Gate 5, §17 critérios finais). **A tabela de métricas é preenchida sozinha; falhas críticas e a decisão de conclusão são do usuário.**

## Métricas automáticas

A tabela abaixo é gerada pelo banco local do app, sem digitação. Uma tarefa agendada diária roda `python tools/dogfood-report.py --write`, que lê o banco só para leitura e refaz o bloco inteiro. Para ver sem gravar, rode o comando sem `--write`.

Só entram números agregados, por dia local. Dias sem atividade não aparecem. Ruído é descartados ÷ decididos. Latência é captura até candidato; revisão é candidato até confirmação. As definições são as do Diagnóstico. MCP mostra as consultas do agente (total e respondidas); "—" quer dizer que o banco ainda não registra isso.

<!-- dogfood:auto:start -->

| Dia | Capturas | Candidatos | Confirmados | Descartados | Ruído | Auto | Desfeitos | Latência p50/p95 | Revisão p50/p95 | Injeções | Tokens | Omitidos | MCP | MCP respondidas | Perdas |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-10-06 | 27 | 13 | 13 | 0 | 0% | 28 | 0 | 1.4 min / 1.8 min | 9 s / 56 s | 1 | 292 | 21 | — | — | 8 |
| **Total (1 dia)** | 27 | 13 | 13 | 0 | 0% | 28 | 0 | 1.4 min / 1.8 min | 9 s / 56 s | 1 | 292 | 21 | — | — | 8 |

Atualizado em 2026-10-06 12:07 (hora local).

<!-- dogfood:auto:end -->

## Baseline de referência (teste automatizado — não é dogfood)

Registrado em 2026-09-29 como ponto de partida: suíte completa verde (application 102, storage-sqlite 61, local-api 26, ai-provider 13+1, desktop 12, contracts 9+1, telemetry 13, domain 4, arch 11/12), deny 0, audit 0, E2Es (`jobs-recovery`, `capture-outbox`, `install-clean`) PASS. Métricas reais começam em zero quando a UI estiver disponível.

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
