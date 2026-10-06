# 25 — Memória ciente de branch

**What to build:** Uma decisão ou regra tomada num worktree ou numa branch que ainda não
foi juntada entra na memória marcada com essa branch e só vale para sessões nela. Quando a
branch é juntada, a decisão passa a valer para o projeto; quando a branch é descartada, ela
expira. Rascunho fica tratado como rascunho.

**Blocked by:** dogfood ([20](20-dogfood-e-conclusao.md)). Experimento: medir antes de
virar regra do produto.

**Status: open**

## Motivo

Em 06/10/2026, no JevGuard, decisões do adapter do Claude Code foram escritas no worktree
`JevGuard-claude-code` (ADR 0014), enquanto regras de outra frente (OpenCode V2) valiam
para todo o projeto e contradiziam o trabalho do worktree. Ferramentas de agente (Orca,
Claude Code, Codex) abrem um worktree por tarefa, então trabalho em branch é a regra, não a
exceção. Hoje o Xemnas trata como do projeto inteiro tudo o que captura num worktree, e
varrer os documentos dos worktrees foi descartado: traria rascunhos e duplicatas como se
valessem. A importação explícita de um documento cobre o caso pontual; este ticket cobre o
geral.

## Escopo

- [ ] **Branch na captura.** O hook e a importação registram a branch e o worktree de
  origem (`git rev-parse --abbrev-ref HEAD`, já há resolução por diretório em cache).
- [ ] **Validade por branch.** Decisões e regras nascidas fora da branch principal ganham
  um qualificador de branch; a seleção de contexto só as entrega a sessões cuja branch
  bate (ou à branch principal depois do merge).
- [ ] **Promoção e expiração.** Um passo periódico (pode ser parte do "Sonho",
  [pesquisa](../../pesquisas/sonho-consolidacao-da-memoria.md)) confere se a branch foi
  juntada (commit alcançável a partir da principal) e promove; se a branch sumiu sem
  merge, encerra a validade e diz por quê.
- [ ] **Na interface.** A decisão mostra a branch de origem e o estado (em branch,
  promovida, expirada).
- [ ] **Medição.** Corpus com tarefas em branches diferentes: a decisão de uma branch não
  contamina outra; depois do merge, chega a todas. Portão de contexto sem regressão.

**Aceite:** uma decisão tomada num worktree aparece nas sessões daquele worktree e não nas
outras; depois do merge, aparece em todas; uma branch descartada não deixa regra em vigor.
