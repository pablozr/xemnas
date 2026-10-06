# ADR-0013 — Contexto sem embeddings: busca lexical mais grafo

**Status:** Vigente. Aceito em 06/10/2026.

## Contexto

O ADR-0003 deixou embeddings e busca vetorial fora do contexto até medir a busca lexical.
Entre 05 e 06/10/2026 foram medidos dois modelos pequenos em Rust puro (`potion-multilingual-128M`
e `multilingual-e5-small`) num corpus selado de decisões do próprio projeto, num Windows de 8 GB
sem GPU. O protocolo e os números estão em [busca semântica local](../../pesquisas/busca-semantica-local.md).

## Decisão

1. **A seleção de contexto continua lexical (FTS5) mais o grafo.** Nenhum embedding, banco
   vetorial ou reranqueador entra no caminho do contexto.
2. **O motivo é medido.** Nenhum modelo chegou a 0,70 de precisão e de cobertura no corpus
   selado sem aumentar os casos contaminados. As decisões de um projeto dividem o mesmo
   vocabulário, e a folga entre acerto e erro se sobrepunha.
3. **O ganho vem do grafo.** Vínculos por menção e pela IA e regras com escopo
   ([ADR-0014](0014-regras-com-escopo-por-componente.md)) melhoraram o contexto onde os
   vetores não melhoraram.
4. **Reabrir só com evidência:** um corpus em que léxico e grafo falhem e a semântica seja o
   que falta, ou a biblioteca de documentos entrar no roadmap.

## Consequências

- Menos memória, nenhuma dependência nativa nova e nenhum custo de CPU em repouso, de acordo
  com o pilar de desempenho e baixo custo.
- Os portões de assertividade do contexto (`tools/core-quality.py`) medem esta seleção; o piso
  sobe quando ela melhora.
- Tarefas cujo texto não compartilha palavra nem componente com a decisão certa continuam sem
  resposta. É um limite conhecido, não um defeito a esconder.

## Alternativas rejeitadas

- **Resgate pelo vetor mais parecido, troca do item lexical, seleção só por vetor:** nenhuma
  variante que mantinha o corpus de ajuste acima de 0,90 melhorou o corpus de validação; só
  pelo vetor ficou abaixo do léxico.
- **FastEmbed/ONNX Runtime:** a DLL nativa foi bloqueada pelo Smart App Control, e o
  benchmark nunca rodou.
