# Roadmap

Tickets abertos e o que já foi entregue. O histórico detalhado de cada entrega está no git.

## Próximos

Em ordem de prioridade:

1. **Dogfood real** — [20](tickets/20-dogfood-e-conclusao.md). A coleta é automática (`tools/dogfood-report.py` + tarefa agendada diária, métricas em [`dogfood-log.md`](../operacao/dogfood-log.md), consultas MCP na tabela `agent_queries`). Falta usar por uma semana no projeto real com o Claude Code e decidir com os dados. Features novas ficam congeladas até lá.
2. **Tickets abertos**:
   - [21](tickets/21-mensagens-do-backend-traduziveis.md) — mensagens do backend traduzíveis.
   - [22](tickets/22-instalacao-e-integracoes-com-um-clique.md) — instalação e integrações com um clique (depois do dogfood).
   - [23](tickets/23-identidade-das-migracoes.md) — identidade das migrações.
   - [25](tickets/25-memoria-ciente-de-branch.md) — memória ciente de branch (experimento, depois do dogfood).
   - [26](tickets/26-fluxos-didaticos-na-visao.md) — fluxos didáticos com C4 na Visão (depois do dogfood).
   - [27](tickets/27-visao-geral-fresca-e-coerente.md) — visão geral fresca e coerente com o código.
   - [28](tickets/28-documentos-por-inteiro.md) — documentos por inteiro (teto de propostas, seções de refinamento).
   - [29](tickets/29-observacoes-de-manifest-sem-churn.md) — observações de manifest sem churn e com a raiz.
   - [30](tickets/30-grafo-verificacao-e-descricoes-vivas.md) — grafo: verificação e descrições vivas.
3. **Resto da fase 4** — [24](tickets/24-grafo-injecao-por-arquivo-e-conflitos.md): injeção por arquivo, conflitos e padrões de caminho obsoletos.

## Entregue

| Item | O que entregou |
| --- | --- |
| MVP 01 — Gate 0: validar GPUI | Spike com shell, lista virtual de 10 mil itens e medições de startup, memória, scroll e acessibilidade. |
| MVP 02 — Gate 0: integrações | SQLite fora da UI, job com progresso, API Axum autenticada e build Windows provados. |
| MVP 03 — Decisão da UI | GPUI aceito para a camada de UI, fixado no rev `24402360` ([ADR-0001](../arquitetura/adr/0001-aceitar-gpui-para-a-camada-de-ui.md)). |
| MVP 04 — Fundação modular | Workspace Rust com dependências direcionais, tracing sanitizado e checagens de arquitetura. |
| MVP 05 — Primitives Quiet Glass | Tokens, controles e estados reutilizáveis, comparados à referência visual. |
| MVP 06 — Projects | Cadastro, listagem e remoção de acompanhamento sem tocar o diretório. |
| MVP 07 — Jobs recuperáveis | Fila persistida com estados explícitos; jobs interrompidos voltam à fila. |
| MVP 08 — Contrato de captura | Capture Envelope e artefatos versionados, com schema e fixtures em Rust e TypeScript. |
| MVP 09 — Ingestão segura | API loopback com token, limites e timeout; receipt idempotente e análise assíncrona. |
| MVP 10 — Adapter OpenCode | Captura no idle desde o checkpoint, sem IA no caminho e sem reasoning ou segredos. |
| MVP 11 — Outbox e diagnóstico | Capturas preservadas com o app fechado, importadas depois; rejeitados com diagnóstico e retenção de 30 dias (`8067a34`). |
| MVP 12 — Extração offline | Filtro de relevância e candidatos de decisão determinísticos por extractor fake. |
| MVP 13 — Perfil de IA consentido | Provider real atrás de `CandidateExtractor`, desligado por padrão, com preview, consentimento e cofre de segredos (`c6712d8`). |
| MVP 14 — Proveniência da análise | Assessments, retries e reprocessamento com perfil, modelo, política e hashes rastreáveis (`c0d11a9`). |
| MVP 15 — Decision Inbox | Revisão de candidatos com evidência, ajuste, confirmação, rejeição e adiamento; tela entregue. |
| MVP 16 — Decisões pesquisáveis | Decisões versionadas sem hard delete, busca FTS5 e controle de concorrência (`f7be0e0`). |
| MVP 17 — Exportação e E2E | Exportação Markdown/JSON após preview e teste do ciclo completo sem mutar o projeto. |
| MVP 18 — Distribuição e upgrade | Build e instalação reproduzíveis no Windows; migrações forward-only e transacionais (ADR-0002). |
| MVP 19 — Hardening operacional | Recovery, payload hostil, redação no motor Rust (`bcbd6af`) e diagnóstico sanitizado (`03e47b3`). |
| Fase 3 — Context Pack manual | Relações e substituição de decisões, claims temporais e Context Pack exportável com preview (`735b493`; [ADR-0003](../arquitetura/adr/0003-fase-3-contexto-recuperavel.md)). |
| Fase 3 — Injeção de contexto | Bloco compacto no pedido do agente, com orçamento, sem repetição na sessão, auditoria e modo sombra; modo por projeto na aba Contexto (`40ead8a`) e métricas no Diagnóstico (`03e47b3`). |
| Fase 4 — Grafo de entidades (parcial) | Componentes, tecnologias e arestas confirmadas; mapa, vizinhança, linha do tempo, lente de arquivo e impacto no backend, telas do grafo e `file_context` no MCP (`313ea94`, `bd45109`; [ADR-0005](../arquitetura/adr/0005-grafo-de-entidades.md)). O resto está no ticket 24. |
| Vínculos por estrutura e menção afirmativa | Menção que ignora o que o texto só cita para excluir, nome do projeto e nome dentro de outro caminho; índice limitado de arquivos (o caminho citado precisa existir); dono por manifest e por símbolo; raiz e CI como componentes; quem confirmou cada vínculo (migração 0047); religar, herança só do confirmado, juiz de vínculos à parte, revalidação e fantasmas ([ADR-0016](../arquitetura/adr/0016-vinculos-por-estrutura-e-mencao-afirmativa.md); commits `af6d807`..`1fb0115` da branch `fix/vinculos-por-estrutura`). |
| Fase 5 — MCP de leitura | Binário `xemnas-mcp` (stdio) com `get_decision`, `search_context` e `file_context`, mais os hooks do Claude Code. Falta confirmar a configuração no OpenCode real. |
| Polaridade como alarme, componentes na extração e idioma da saída | O léxico de negação só decide o inequívoco em inglês e português e qualquer outro gatilho marca o vínculo como polaridade duvidosa (as regras nunca aceitam; vai ao juiz ou à pessoa); o extrator indica os componentes com citação literal e a adoção os grava como sugestão da IA, sem a chamada separada; a Visão sai no idioma do app; raiz e CI sem descrição fixa no banco. [ADR-0017](../arquitetura/adr/0017-polaridade-como-alarme-componentes-na-extracao-e-idioma-da-saida.md); commits `43dc0fe`, `7170b24`, `6f0e67d`, `118cc9e`, `01f6b3f`, `49c5005`, `325c1a4`. |

## Como usar esta pasta

Ticket aberto vive em `tickets/NN-nome.md`. Ao fechar, anote no ticket o que foi feito e o commit, mova a linha para "Entregue" e apague o arquivo no mesmo commit.
