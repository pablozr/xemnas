# Precisão do contexto entregue ao agente

**Data:** 05/10/2026.
**Pergunta:** como reduzir o ruído do bloco `<xemnas-context>` sem perder o que o agente
precisa, com custo baixo e medindo cada passo?
**Status:** Em implementação. Passo 1 (corpus v3) feito: linha de base 0,23 de precisão.
Passo 2: peso por campo e cobertura mínima de termos (duas palavras significativas em comum,
lista de palavras ignoradas ampliada) levaram a precisão a 0,63 e a contaminação de 9 para
1 de 24, com cobertura de 0,76 para 0,58 (troca intencional; a ponte PT/EN deve recuperar).
O corte relativo ao melhor resultado não mudou nenhum número no desenvolvimento e ficou de
fora. A busca que o agente pede sob demanda é exploratória e não usa o corte.
Passo 3: foco do grafo (num componente largo, itens que não falam da tarefa saem quando
outros falam) levou a precisão a 0,70 (holdout 0,73). O caso que motivou a regra é do
holdout, que nesse ponto deixa de ser prova independente. Regras com escopo ficam para
quando o portão medir regras permanentes.
Passo 4: ponte PT/EN sem modelo (`crates/application/src/terms.rs`): remoção de acentos,
plurais dos dois idiomas, terminações verbais do português, `-ing`/`-ed` do inglês e um
glossário bilíngue de vocabulário geral de software; a busca de texto pede o radical como
prefixo. Precisão 0,75, cobertura 0,91 (desenvolvimento 1,0), holdout 0,70 / 0,78 / 0.
As terminações verbais vieram de um caso do holdout. Falta a meta de precisão. Antes do passo 5
(embeddings), as opções sem modelo estão em [busca além do léxico](busca-alem-do-lexico.md);
a expansão do documento na adoção, como último recurso, levou a 0,76 / 0,95. Sinônimos da
tarefa como um só conceito e cobertura relativa ao melhor resultado (a partir de 4
conceitos): 0,83 / 0,95, holdout 0,92 / 0,89. Oração principal (o item precisa se apoiar
nela, tanto quanto o melhor resultado): 0,94 / 0,95 / 0 contaminados, holdout 0,96 / 0,89.
Corpus v4, escrito às cegas e selado: 0,58 / 0,62 / 4 de 21. Os ganhos depois da ponte
PT/EN eram, na maior parte, ajuste ao v3; o próximo passo se decide pelo v4 e pelo dogfood. Passo 5 em
[embeddings no contexto](embeddings-no-contexto.md): o modelo estático `potion` como veto
levou o v4 a 0,67 / 0,60 / 4, abaixo do aceite, e não entra.
Ganho provavelmente otimista: falta um corpus v4 com holdout novo. A medição usa o portão de
[qualidade do núcleo](../arquitetura/qualidade-do-nucleo.md).

Complementa [medir a eficácia do contexto](medir-eficacia-do-contexto.md) (se o contexto
ajuda o agente), [memória semântica local first](memoria-semantica-local-first.md) e
[embeddings e reranking locais](embeddings-e-reranking-local.md) (como rodar modelos no
desktop). Esta pesquisa trata de outra pergunta: **o que entra no bloco**.

## Por que a precisão importa mais que a cobertura

- **Um item irrelevante já atrapalha.** Modelos perdem acerto com uma única informação
  irrelevante no enunciado ([Shi et al., ICML 2023](https://arxiv.org/abs/2302.00093)).
- **Quase-acertos atrapalham mais que contexto longo.** O estudo de "context rot" da
  Chroma (jul/2025, 18 modelos) mostra que um trecho parecido mas errado reduz o acerto e
  que quatro deles pioram ainda mais ([resumo](https://www.zenml.io/llmops-database/context-rot-evaluating-llm-performance-degradation-with-increasing-input-tokens)).
  É exatamente o nosso ruído: decisões do mesmo projeto, com palavras em comum.
- **Posição conta.** Informação no meio de um contexto longo é a menos usada
  ([Liu et al., 2023](https://cs.stanford.edu/~nfliu/papers/lost-in-the-middle.arxiv2023.pdf)).
  Nosso bloco é curto (300 tokens), então o efeito é menor, mas a ordem ainda importa.
- **O princípio de engenharia de contexto da Anthropic:** o menor conjunto de tokens de
  alto sinal que maximiza o resultado
  ([resumo do guia](https://agentic-ai.readthedocs.io/en/latest/ContextEngineering/anthropic/)).
- **E a pesquisa anterior do Xemnas** mostrou que só ajuda o que é específico e fora do
  padrão; visões gerais não ajudam e custam tokens ([medir a eficácia](medir-eficacia-do-contexto.md)).

Conclusão: para o Xemnas, **abster-se é melhor que entregar um item fraco**.

## Diagnóstico do Xemnas hoje

`ContextPacks::build_pack` (`crates/application/src/context.rs`):

1. Decisões e regras que o **grafo** liga aos arquivos da tarefa entram primeiro.
2. Depois, busca de texto do SQLite (FTS5, tokenizador `unicode61`, que já remove acentos)
   com **qualquer palavra** da tarefa ligada por `OR`; até 50 resultados por `bm25`.
3. Os itens entram em ordem até encher o orçamento. **Não há limiar de relevância**: uma
   decisão que divide só uma palavra com a tarefa entra.
4. Regras do tipo restrição e convenção entram **sempre** (permanentes), mesmo sem
   relação com a tarefa.
5. Não há radical das palavras (português e inglês), peso por campo (pergunta, escolha,
   motivo) nem abstenção explícita.

Portão de hoje: precisão **0,65** (17 de 26), cobertura **0,94** (17 de 18), **2 de 15**
casos negativos contaminados. De onde vem o ruído no corpus:

| Caso | O que acontece | Causa provável |
| --- | --- | --- |
| `terminal-color` (negativo) | 1 item em 2 das 3 consultas | uma palavra solta em comum ("terminal", "Override") |
| `database`, `override` | 1 item a mais | correspondência parcial de poucas palavras |
| `polarity` | itens a mais e uma consulta em inglês sem resultado | ruído parcial e falta de ponte entre inglês e português |

**Limite do corpus:** 10 famílias e 30 consultas, sintéticas. Ajustar só por ele arrisca
"decorar" o corpus. O primeiro passo do plano amplia o corpus antes de calibrar.

## Técnicas, da mais barata à mais cara

| # | Técnica | De onde vem | Ataca | Custo | Risco |
| --- | --- | --- | --- | --- | --- |
| 1 | **Cobertura mínima de termos**: exigir que uma fração das palavras significativas da tarefa apareça (ex.: 2 de 3, ou 50% em consultas longas) | `minimum_should_match` do Elasticsearch ([guia](https://www.baeldung.com/ops/elasticsearch-minimum-should-match)) | palavra solta | muito baixo (só a consulta) | perder casos com sinônimo; mitigado pela #3 |
| 2 | **Corte relativo ao melhor resultado**: descartar o que tem pontuação bem abaixo do primeiro, e abster quando nem o primeiro passa de um mínimo | seleção adaptativa sem top-k fixo ([exemplo recente](https://arxiv.org/pdf/2608.07152)); avaliador de recuperação do [CRAG](https://arxiv.org/pdf/2401.15884) | itens fracos no fim da lista | muito baixo | limiar calibrado no corpus; precisa de holdout |
| 3 | **Peso por campo no bm25**: a pergunta e a escolha valem mais que o motivo | `bm25(fts, w1, w2, w3)` do [FTS5](https://www.sqlite.org/fts5.html) | coincidências no texto longo do motivo | muito baixo | nenhum relevante |
| 4 | **Radical das palavras PT e EN + glossário do projeto** ("cache"/"caching", "avaliação"/"evaluation") | Snowball em Rust ([rust-stemmers](https://docs.rs/rust-stemmers)); o `porter` do FTS5 é só inglês | consulta em inglês sobre decisão em português | baixo (crate pequena, sem modelo) | radical agressivo junta palavras diferentes; medir |
| 5 | **Proximidade no grafo**: o que o grafo liga aos arquivos ou partes citadas na tarefa sobe; o que só tem coincidência de texto precisa de mais evidência | "node distance reranker" do [Graphiti/Zep](https://help.getzep.com/graphiti/working-with-data/searching); reforço de 10× a 50× por menção no [repo map do Aider](https://github.com/NousResearch/hermes-agent/issues/535) | ruído lexical; aproveita o grafo como centro da memória | baixo (o grafo e as menções já existem) | depende da qualidade dos vínculos |
| 6 | **Regras com escopo**: regra permanente só entra quando o escopo bate com os arquivos da tarefa | regras por caminho do [Cursor](https://cursor.com/docs/rules) e do Claude Code ([comparação](https://dev.to/rulestack/how-cursor-claude-code-and-codex-actually-load-your-project-rules-and-why-yours-get-ignored-1l1j)) | orçamento gasto com regras sem relação | baixo | regra realmente global perde espaço; manter as marcadas como globais |
| 7 | **Busca híbrida com embeddings locais**, fundida por Reciprocal Rank Fusion (k=60) | [Cormack et al., 2009](https://bigdataboutique.com/blog/reciprocal-rank-fusion-how-it-works-and-when-to-use-it); modelos estáticos [Model2Vec](https://huggingface.co/minishlab/potion-multilingual-128M) (milissegundos em CPU) ou `multilingual-e5-small` via [fastembed-rs](https://docs.rs/fastembed) | sinônimos e paráfrases | médio (modelo de 30 a 120 MB, índice de vetores) | download, memória em 8 GB; build nativo já foi bloqueado no Windows ([metodologia](metodologia-benchmark-semantico.md)) |
| 8 | **Reranker local (cross-encoder)** nos 10 a 20 primeiros | `bge-reranker-v2-m3` via fastembed-rs | ordem fina | alto (CPU por consulta) | latência; só se 1 a 7 não bastarem |
| 9 | **Juiz de IA para ambiguidade** | já existe (router opcional, até 8 consultas por dia) | casos que nada local resolve | custo de API, opcional | latência e consentimento; continua opcional |
| 10 | **Ordem do bloco**: o mais relevante primeiro | Liu et al., 2023 | uso do que entrou | nulo | nenhum |

As técnicas 1 a 6 cabem no pilar de baixo custo: são código e consulta SQL, sem modelo
novo. A 7 tem o melhor potencial para sinônimos, mas só vale depois de medir o que as
anteriores entregam, como já recomendam as pesquisas de [memória semântica](memoria-semantica-local-first.md).

## Como medir

- **Ampliar o corpus antes de calibrar**: de 10 para cerca de 30 famílias, incluindo casos
  reais do próprio Xemnas e da [avaliação no ripgrep](../operacao/avaliacao-repositorio-real.md),
  com consultas em PT, EN e com distrator. Separar desenvolvimento e holdout por família,
  como já faz o corpus atual.
- **Métricas**: precisão, cobertura e casos contaminados (portão atual) e, para a ordem,
  precisão ponderada pela posição, como a "context precision" do
  [RAGAS](https://docs.ragas.io/en/stable/concepts/metrics/available_metrics/context_precision/).
  Medir também tokens por bloco e latência p95.
- **Regra do portão**: cada técnica entra só se melhora a precisão **no holdout** sem
  baixar a cobertura do piso; o piso sobe no mesmo commit.

## Plano proposto

1. **Ampliar o corpus** (sem mexer no algoritmo) e publicar a nova linha de base.
2. **Pacote barato**: técnicas 3 (peso por campo), 1 (cobertura mínima) e 2 (corte relativo
   e abstenção), uma por commit, medindo cada uma no portão.
3. **Grafo no centro**: técnica 5 (proximidade no grafo, usando as menções de partes na
   tarefa) e 6 (regras com escopo).
4. **Ponte PT/EN**: técnica 4 (radical e glossário).
5. **Só se a meta não for atingida**: técnica 7 (embeddings estáticos primeiro, depois
   E5-small), com orçamento de latência e memória fixado antes.

**Meta sugerida** (a confirmar com o corpus ampliado): precisão ≥ 0,85, cobertura ≥ 0,90
e nenhum caso negativo contaminado, com `build_pack` p95 abaixo de 5 ms.
