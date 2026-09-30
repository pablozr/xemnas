# ADR-0003: Fase 3 — contexto recuperável e Context Pack manual

- **Status:** aceito
- **Data:** 2026-09-30
- **Contexto:** o backend do MVP está completo (tickets 01–19) e o ticket 20 aguarda o dogfood. O roteiro de `stack-e-arquitetura-rust-gpui.md` põe a Fase 3 (Context Claims, relações, Context Packs manuais, timeline) antes do acesso por agentes (Fase 5, MCP). SCOPE-001 exige decisão explícita para sair do escopo do MVP; o usuário aprovou iniciar a Fase 3 em paralelo ao fechamento do MVP.
- **Decisão:** implementar no backend, nesta ordem:
  1. **Relações entre decisões** (`supersedes`, `depends_on`, `conflicts_with`), criadas só por ação humana. Substituir uma decisão marca a anterior como `superseded` e registra a relação; nada é apagado.
  2. **Context Claims** tipados (`assumption`, `constraint`, `goal`, `convention`), com `valid_from`/`valid_until` e status `active`/`retired`. Nascem por ação explícita do usuário, opcionalmente a partir de uma Engineering Decision (citada como origem). Nenhuma IA cria ou confirma Claims nesta fase.
  3. **Context Pack manual**: `ContextProvider::build_pack(ContextRequest)` recebe Project, tarefa, data de referência e orçamento de tamanho, e devolve uma seleção pequena e citável de decisões vigentes e Claims válidas naquela data. A seleção é lexical (FTS5 + filtros, AD-14) e determinística. O pack é temporário: não é persistido; exportar para Markdown/JSON exige ação explícita, preview e destino escolhido, como na exportação de decisões.
- **Fica fora desta fase:** servidor MCP e endpoint HTTP de pack (Fase 5), embeddings/vector DB/GraphRAG, biblioteca global (Fase 4), assistente (Fase 6), injeção automática de contexto em agentes, Claims extraídas por IA, e as telas de timeline/grafo (a timeline é uma consulta do backend; a UI fica com o front).
- **Consequências:**
  - `domain` passa a ter regras reais (relações e validade temporal), mantendo ARCH-001;
  - duas migrations forward-only (relações; Claims), seguindo DATA-001;
  - o Context Pack é o contrato que a Fase 5 vai expor por MCP sem reescrever a seleção;
  - a utilidade do pack deve ser medida no uso real antes de qualquer automação.
- **Alternativas rejeitadas:** começar pelo MCP (entregaria a porta sem conteúdo útil); guardar Claims como campos de texto das decisões (sem validade temporal nem relação própria); grafo genérico de nós e arestas (a spec pede relações tipadas e auditáveis, com o grafo como projeção).
