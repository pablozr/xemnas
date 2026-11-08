# 33 — Ponteiros de documentos na injeção

**What to build:** Depois que documentos deixam de entrar na injeção automática, devolver ao agente só um ponteiro para o documento certo, sem parafrasear o conteúdo.

**Blocked by:** depois do dogfood (ticket 20); precisa dos itens de origem documental fora da injeção (já entregue).

**Status: open**

- [ ] No máximo 2 linhas `doc: <caminho> — <título>` quando termos do prompt casarem com o título ou os cabeçalhos do índice `project_documents`, dentro de 300 tokens.
- [ ] Nunca conteúdo do documento; nada quando não há casamento.
- [ ] Modo de falha: documento arquivado (`historico`, `legacy`, `spike`, ...) não vira ponteiro.

**Aceite:** portão de precisão no corpus de injeção (ponteiro certo quando o prompt cita o assunto, nenhum quando não cita) e `prepare` com p95 ≤ 20 ms medido com `XEMNAS_PERF=1`.
