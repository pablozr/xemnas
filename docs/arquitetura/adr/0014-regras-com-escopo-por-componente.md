# ADR-0014 — Regras com escopo por componente

**Status:** Vigente. Aceito em 06/10/2026. A herança do item 3 foi refinada pelo [ADR-0016](0016-vinculos-por-estrutura-e-mencao-afirmativa.md).

## Contexto

Uma regra do projeto ("só um humano resolve X") entrava em todo pacote de contexto que casasse
pelo texto, mesmo quando o trabalho era em outro componente. Isso gasta orçamento de tokens e
treina o agente a ignorar o bloco. O grafo ([ADR-0005](0005-grafo-de-entidades.md)) já liga
regras a componentes por arestas `applies_to`.

## Decisão

1. **Regra ligada a componentes só entra no contexto quando a tarefa toca um deles.** A tarefa
   toca um componente quando um arquivo casa com seus padrões ou o texto o menciona. Um
   componente conta como tocado pelos seus subcomponentes. *Emenda (07/10/2026):* uma decisão
   do pacote ligada ao componente deixou de tocá-lo, porque uma decisão arrastava todas as
   regras do componente; e de um componente com mais de `MAX_TIED_RULES` (3) regras ligadas
   entram no máximo 3, as que cobrem conceitos suficientes da tarefa, mais cobertas primeiro
   (depois a oração principal, a mais recente e o id). O nome do componente, que o tocou, não
   conta como tema.
2. **Regra sem ligação continua global.** Ausência de escopo não restringe nada.
3. **Regra derivada herda o escopo da decisão de origem, só o que foi confirmado.** Uma aresta pendente da decisão não conta. Se o texto da regra (enunciado ou citação) nomeia afirmativamente algum dos componentes da decisão, por nome, caminho, dependência ou símbolo (ADR-0016), a regra fica só com esses; se não nomeia nenhum, herda todos os confirmados. A herança é gravada como aresta derivada com autor `inherited` (não como `human`), no mesmo ponto em que a sugestão é confirmada, e o preenchimento de regras antigas é idempotente. Invalidar o vínculo da decisão com um componente invalida, em cascata, o que as regras herdaram dele; o que uma pessoa confirmou na regra fica. Uma aresta existente impede religar, salvo o religar estrutural do ADR-0016 (descartada pelas regras ou pela IA, nunca a removida por uma pessoa).

## Consequências

- Implementado em `crates/application/src/graph/scope.rs`. Custa um par de consultas extra
  (arestas e entidades) por pacote.
- O corpus de contexto ganhou casos de regra com escopo e o piso do portão subiu; o tempo de
  `build_pack` continua dentro do teto (cerca de 2 a 3 ms medidos, teto de 20 ms).
- O resultado depende de o mapa estar ligado: regra sem vínculo vale em todo lugar.

## Alternativas rejeitadas

- **Escopo por palavras da regra:** adivinharia o componente; o vínculo confirmado já existe.
- **Todas as regras sempre:** desperdiça orçamento e dilui o que importa.
