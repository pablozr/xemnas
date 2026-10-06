# 20 — Dogfood e decisão de conclusão do MVP

**What to build:** Usar o produto por uma semana no projeto real com o Claude Code, medir o atrito e decidir a conclusão do MVP com os dados.

**Status: blocked (coleta pronta; falta a semana de uso e a decisão do usuário)**

## Já pronto

- Coleta automática: `tools/dogfood-report.py` e uma tarefa agendada diária preenchem [`docs/operacao/dogfood-log.md`](../../operacao/dogfood-log.md) a partir do banco local (`18b7001`, `9c0f4d9`).
- Consultas do agente pelo MCP ficam na tabela `agent_queries`, para medir se o contexto é usado (`18b7001`).
- Métricas de ruído, perdas, latência e tempo de revisão em `DiagnosticsDocument.metrics`.
- Telas Inbox, Decisões, Exportação e Diagnóstico existem (`apps/desktop-gpui/src/screens/`).
- Auditoria dos 20 critérios do MVP reexecutada em 2026-09-29; os pendentes na época eram só de UI.

## Falta

- [ ] Usar por uma semana no projeto real com o Claude Code, sem mexer no produto (congelamento de features novas).
- [ ] Ler as métricas e as consultas gravadas e registrar falhas críticas na tabela do dogfood-log.
- [ ] Decisão de conclusão: é do usuário, pelo formulário do dogfood-log (nunca decidir produto por ele).

**Aceite:** o formulário do dogfood-log preenchido com números reais e a decisão registrada.
