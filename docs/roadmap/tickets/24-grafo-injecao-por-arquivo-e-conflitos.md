# 24 — Grafo: injeção por arquivo e conflitos por entidade

**What to build:** Fechar o que resta da fase 4 do grafo de entidades ([ADR-0005](../../arquitetura/adr/0005-grafo-de-entidades.md)). Mapa, vizinhança, linha do tempo, lente de arquivo, impacto (`crates/application/src/graph/query.rs`) e a ferramenta `file_context` do MCP já existem.

**Blocked by:** dogfood (ticket 20): só decidir com os dados se vale a pena.

**Status: open**

- [ ] Injeção por arquivo no Claude Code (`PostToolUse` nas ferramentas de edição), como o OpenCode já faz depois de cada edição (`tool.execute.after`, gatilho `edit` em `adapters/opencode/src/context.ts`); avaliar também antes da edição (`tool.execute.before`, `PreToolUse`). Medir se melhora o uso do contexto antes de ligar por padrão.
- [ ] Conflitos: decisões ativas conflitantes na mesma entidade e alerta quando o "reconsiderar quando" de uma decisão talvez já tenha acontecido.
- [ ] Verificação de padrão de caminho que deixou de casar qualquer arquivo (renomeações).
  - Parte feita pelo [ADR-0016](../../arquitetura/adr/0016-vinculos-por-estrutura-e-mencao-afirmativa.md): a atualização do mapa já aponta (`stale_components`) o componente não declarado cujo padrão literal não existe mais, com o dono dos arquivos citados. Falta cobrir padrões com `*` que deixaram de casar, a tela para aposentar e o aviso quando um arquivo renomeado deixa de casar.

**Aceite:** cada item com teste de assertividade e de desempenho (`python tools/core-quality.py`), sem custo de IA.
