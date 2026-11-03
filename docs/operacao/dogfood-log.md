# Registro de dogfood (Gate 5, ticket 20)

Semana de uso real do produto para medir **ruído, perdas, latência e tempo de revisão** e decidir a conclusão do MVP (spec §16 Gate 5, §17 critérios finais). **A tabela de métricas é preenchida sozinha; falhas críticas e a decisão de conclusão são do usuário.**

## Métricas automáticas

A tabela abaixo é gerada pelo banco local do app, sem digitação. Uma tarefa agendada diária roda `python tools/dogfood-report.py --write`, que lê o banco só para leitura e refaz o bloco inteiro. Para ver sem gravar, rode o comando sem `--write`.

Só entram números agregados, por dia local. Dias sem atividade não aparecem. Ruído é descartados ÷ decididos. Latência é captura até candidato; revisão é candidato até confirmação. As definições são as do Diagnóstico. MCP mostra as consultas do agente (total e respondidas); "—" quer dizer que o banco ainda não registra isso.

<!-- dogfood:auto:start -->

| Dia | Capturas | Candidatos | Confirmados | Descartados | Ruído | Auto | Desfeitos | Latência p50/p95 | Revisão p50/p95 | Injeções | Tokens | Omitidos | MCP | MCP respondidas | Perdas |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-10-07 | 25 | 67 | 65 | 2 | 3% | 258 | 0 | 59 s / 1.9 min | 5.6 min / 20.6 min | 9 | 2257 | 163 | 0 | 0 | 0 |
| 2026-10-08 | 4 | 1 | 1 | 0 | 0% | 1 | 0 | 17 s / 17 s | 43.3 min / 43.3 min | 2 | 387 | 19 | 0 | 0 | 0 |
| 2026-10-13 | 31 | 3 | 3 | 0 | 0% | 14 | 0 | 17 s / 25 s | 3 s / 3 s | 9 | 2320 | 30 | 0 | 0 | 0 |
| 2026-10-14 | 53 | 72 | 71 | 1 | 1% | 181 | 0 | 1.1 min / 2.1 min | 32 s / 1.8 min | 3 | 627 | 7 | 0 | 0 | 0 |
| 2026-10-15 | 27 | 0 | 0 | 0 | — | 0 | 0 | — | — | 12 | 1623 | 7 | 0 | 0 | 0 |
| 2026-10-17 | 49 | 47 | 38 | 9 | 19% | 98 | 0 | 41 s / 1.4 min | 22 s / 40 s | 14 | 2779 | 31 | 0 | 0 | 0 |
| 2026-10-18 | 6 | 3 | 3 | 0 | 0% | 9 | 0 | 16 s / 16 s | 6 s / 6 s | 0 | 0 | 0 | 0 | 0 | 0 |
| 2026-10-28 | 47 | 35 | 22 | 0 | 0% | 93 | 0 | 42 s / 1.5 min | 31 s / 53 s | 0 | 0 | 0 | 1 | 1 | 0 |
| 2026-10-29 | 13 | 7 | 5 | 0 | 0% | 7 | 0 | 10 s / 14 s | 1.6 h / 1.6 h | 6 | 676 | 0 | 8 | 8 | 0 |
| 2026-11-02 | 1 | 0 | 0 | 0 | — | 0 | 0 | — | — | 0 | 0 | 0 | 0 | 0 | 0 |
| **Total (10 dias)** | 256 | 235 | 208 | 12 | 5% | 661 | 0 | 48 s / 1.9 min | 34 s / 20.6 min | 55 | 10669 | 257 | 9 | 9 | 0 |

Atualizado em 2026-11-02 00:10 (hora local).

<!-- dogfood:auto:end -->

## Baseline de referência (teste automatizado — não é dogfood)

Registrado em 2026-09-29 como ponto de partida: suíte completa verde (application 102, storage-sqlite 61, local-api 26, ai-provider 13+1, desktop 12, contracts 9+1, telemetry 13, domain 4, arch 11/12), deny 0, audit 0, E2Es (`jobs-recovery`, `capture-outbox`, `install-clean`) PASS. Métricas reais começam em zero quando a UI estiver disponível.

## Falhas críticas

| # | Data | O quê | Severidade | Status | Correção / commit |
| --- | --- | --- | --- | --- | --- |
| 1 | 2026-10-28 | Sem captura aceita de 19/10 a 27/10. Os hooks disparavam, mas o app ficou fechado: 217 capturas esperavam em `outbox/pending` (26–27/10) e foram importadas ao abrir o app (17 aceitas, 200 rejeitadas). | Alta | Mitigada (app aberto; rotina diária reabre) | — |
| 2 | 2026-10-28 | Sessões dos agentes OpenBot (Claude Code via Agent SDK, `--setting-sources=user,project,local`) usam o cwd da pasta do agente (`OpenBot\Agents\agent-*`, sem git) como `canonical_path`, embora editem outros repositórios por caminho absoluto. Resultado: 711 capturas em `outbox/rejected` com `project_not_registered`, e o envelope não fica guardado. | Crítica | Corrigida e aprovada no QA (3 rodadas): fora de repo, o hook resolve a identidade real dos arquivos do turno e usa o repo dominante (`hook/root.rs`). Prova ao vivo: 12 capturas de sessões de agentes aceitas no xemnas entre 05:00 e 06:26 UTC de 28/10, `rejected` parado em 711. Dívida P3: `prompt.rs:43` manda `files` absolutos sob alias/subst; `envelope.rs:157` aceita `C:repo` como absoluto; conversão duplicada em `protocol.rs:174`. | não commitado |
| 3 | 2026-10-28 | `dinheiro` não estava registrado; as 16 capturas com cwd nele foram rejeitadas e perdidas ao importar a outbox. | Média | Aberta (registrar pela UI) | — |
| 4 | 2026-10-28 | Bloco automático parado em 06/10: a tarefa do Agendador `xemnas-dogfood-report` roda `--write` em `orca\projects\xemnas-ui`, não neste repositório. | Média | Substituída por rotina OpenBot diária às 23:30 | — |
| 5 | 2026-10-28 | Consultas MCP em 0: o servidor `xemnas` está no `~/.claude.json` e sobe nas sessões, mas nenhum agente foi orientado a chamar `search_context`/`file_context`, e com o app fechado as consultas não teriam resposta. As sessões do OpenBot não carregavam o MCP do `~/.claude.json`; agora ele está registrado como servidor MCP do OpenBot. O `xemnas-mcp` usa o próprio cwd como projeto (`main.rs`), que não é o repositório do agente. | Alta | Corrigida e aprovada no QA: parâmetro `project`, ou toplevel do `path` absoluto, com -32602 quando são incompatíveis. Ainda falta a primeira consulta real. | não commitado |

## Decisão de conclusão (preencher ao fim da semana)

- [ ] Ruído, perdas, latência e tempo de revisão registrados acima.
- [ ] Falhas críticas corrigidas (ou justificadas).
- [ ] Critérios finais §17 reexecutados (auditoria no ticket 20) — **os itens de UI (9, 10, 12, 16, 17) dependem da sessão de front**.
- [ ] Decisão: **concluir o MVP** / **não concluir — bloqueios:** ______________________
- _(Assinado por:)_ ____________________  Data: __________
