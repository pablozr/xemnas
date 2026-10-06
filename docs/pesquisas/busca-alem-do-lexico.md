# Busca além da coincidência de palavras, antes dos embeddings

**Data:** 05/10/2026.
**Pergunta:** depois da ponte PT/EN, o que mais dá para fazer sem embeddings para o bloco
`<xemnas-context>` achar paráfrases e cortar distratores?
**Status:** Técnica 2 implementada (`crates/application/src/search_terms.rs`). Contados
sempre, os termos subiram a cobertura (0,91 → 0,95) e derrubaram a precisão (0,75 → 0,66):
termos genéricos ("secret storage", "credential") puxam distratores. Como último recurso,
só quando nenhuma decisão fala da tarefa com as próprias palavras: precisão 0,76,
cobertura 0,95, holdout 0,73 / 0,89. O modelo gerou quase só termos em inglês. Técnica 1
entregue em 06/10/2026 pelas menções com apelidos derivados dos pacotes; 3 pendente. Continua o plano de
[precisão do contexto](precisao-do-contexto.md), entre o passo 4 (feito) e o passo 5
(embeddings).

## Ponto de partida

Portão de [qualidade do núcleo](../arquitetura/qualidade-do-nucleo.md), corpus v3:
precisão 0,75, cobertura 0,91, 1 de 24 casos negativos contaminados, holdout 0,70 / 0,78.
Meta: precisão ≥ 0,85, cobertura ≥ 0,90, nenhum contaminado, p95 < 5 ms.

O que sobra tem duas causas:

| Problema | Exemplos do corpus | Por que o léxico não resolve |
| --- | --- | --- |
| Paráfrase sem palavra em comum | "adicionar coluna no banco" e "evoluir o esquema"; "integração contínua" e "CI" | nenhuma palavra compartilhada, e glossário geral não cobre o vocabulário de cada projeto |
| Distrator do mesmo vocabulário | "cache das observações sem tocar no cache da API" traz as duas decisões de cache | as palavras batem; falta saber de **qual parte** do projeto a tarefa fala |

## "Trocar o FTS pelo BM25" já está feito

O FTS5 é o índice invertido; a ordem vem da função `bm25()` dele, com peso por campo
(pergunta 3, escolha 2, motivo 1). O que o FTS5 não deixa é ajustar `k1` e `b`. Um BM25
próprio em memória cabe (um projeto tem centenas de decisões), mas só ganharia controle de
pontuação, e pontuação não é o problema: as faltas não têm palavra em comum, e o ruído tem
palavras demais em comum. Fica fora até um caso medido pedir.

## Técnicas

| # | Técnica | De onde vem | Ataca | Custo | Risco |
| --- | --- | --- | --- | --- | --- |
| 1 | **Sementes pela menção na tarefa**: componentes, tecnologias e partes cujo nome ou apelido aparece no texto da tarefa entram como se fossem arquivos tocados | o próprio grafo (ADR-0005): `entity_aliases` e o casamento de menções de `graph/mention.rs`, hoje usados só para ligar decisões | distrator do mesmo vocabulário: "cache das observações" vira a parte `observations`, e o que o grafo liga a ela sobe | muito baixo: consulta em memória sobre o snapshot que `context_for_files` já monta | apelido genérico ("api") vira semente demais; o portão de menções já mede precisão 0,93 |
| 2 | **Expansão do documento na extração** (doc2query): a IA que já extrai a decisão grava também até 8 termos de busca, em PT e EN, com sinônimos e siglas; vão para uma coluna própria do índice, com peso menor | Doc2Query ([Nogueira et al.](https://arxiv.org/abs/1904.08375)); filtrar termos inventados rendeu 16% a mais no BM25 ([Doc2Query--](https://arxiv.org/pdf/2301.03266)); revisão recente em [Mackenzie et al., SIGIR 2024](https://jmmackenzie.io/pdf/mzzm24-sigir.pdf) | paráfrase: "esquema" ganha "coluna", "tabela", "schema"; "CI" ganha "integração contínua", "pipeline" | quase nulo na consulta; na captura, alguns tokens de saída na chamada que já existe; decisões antigas pedem uma rodada em lote | termo inventado puxa a decisão para tarefas alheias; mitigado por limite de termos, peso menor e pela regra de cobertura (dois conceitos) |
| 3 | **Relevância espalhada pelo grafo** (PageRank personalizado): as sementes (arquivos e menções) recebem peso, que escorre pelas ligações até decisões e regras; a pontuação do grafo e a do texto se juntam por Reciprocal Rank Fusion | [HippoRAG, NeurIPS 2024](https://proceedings.neurips.cc/paper_files/paper/2024/file/6ddc001d07ca4f319af96a3024f6dbd1-Paper-Conference.pdf) (até 20% em perguntas de vários saltos) e [HippoRAG 2](https://www.emergentmind.com/topics/hipporag-2) | distrator e item ligado só de longe; troca o "ligado ou não" de hoje por uma nota | baixo: grafo de centenas a milhares de nós, poucas iterações | grafo esparso em projeto novo não ajuda; precisa de 1 primeiro |
| 4 | **Sinônimos aprendidos do próprio projeto**: palavras que aparecem juntas nas decisões e capturas do usuário viram um glossário local | análise de contexto local e coocorrência (família do [pseudo-feedback de relevância](https://arxiv.org/pdf/2108.11044)) | vocabulário próprio do projeto que o glossário geral não tem | baixo, recalculado em fundo | precisa de volume real; o corpus sintético não mede |
| 5 | **Pseudo-feedback (RM3)**: os melhores resultados da primeira busca sugerem palavras para uma segunda | [RM3 e fusão em busca de produtos, TREC 2025](https://trec.nist.gov/pubs/trec34/papers/DUTH.product.pdf); robusto e simples segundo [Li et al.](https://arxiv.org/pdf/2108.11044) | cobertura | baixo (duas consultas) | em coleção pequena os primeiros resultados são justamente os distratores: amplia o ruído |
| 6 | **Sinais de uso**: decisão que o agente abre com `get_decision` ou que o usuário confirma como útil sobe | prática comum em busca | ordem | baixo | precisa de registro de uso; viés de popularidade |
| 7 | **Expansão da consulta por IA** (query2doc): a IA escreve um texto hipotético e busca com ele | [Query2doc, EMNLP 2023](https://arxiv.org/abs/2303.07678) (3% a 15% no BM25) | paráfrase | **uma chamada de IA por prompt** | fere o pilar de custo e a latência da injeção; o roteador opcional já cobre ambiguidade |

## Como cada uma se mede sem se enganar

- **1 e 3** são algoritmo puro; entram no portão como os passos anteriores. O corpus precisa
  dar nome e apelido aos componentes (os atuais, como `outbox` e `desktop-ui`, e os que
  um projeto real teria, como as observações e a API), sem olhar as consultas do holdout.
- **2** depende do que o modelo gera. Escrever os termos à mão seria decorar o corpus.
  Proposta: gerar os termos **uma vez** com o modelo real, vendo só o texto de cada decisão
  (nunca as consultas), e versionar como fixture do corpus, com o prompt e o modelo
  anotados. O portão mede com e sem a coluna; o teste de contrato garante que a extração
  grava os termos.
- **4, 5 e 6** precisam de dados reais (dogfood); ficam para depois do corpus.

## O que um mapa real tem hoje

Conferido em 05/10/2026, só leitura, no banco local do único projeto registrado: 6
componentes, todos descobertos do workspace e nomeados pelo pacote (`@jevguard/core`,
`docs`, `.jev`...), **nenhum apelido e nenhuma parte**. Com nomes assim, uma tarefa como
"recalcular o cache das observações" não cita entidade nenhuma, e a técnica 1 quase nunca
dispararia. Ela só rende quando o mapa tiver nomes como as pessoas escrevem ("observações",
"API", "outbox"), o que hoje depende do usuário cadastrar apelidos.

Consequência: medir a técnica 1 com um mapa rico no corpus mostraria um ganho que o uso
real não teria. A técnica 2 alcança qualquer decisão, com ou sem mapa. Uma variante une as
duas: a mesma expansão na extração pode propor apelidos em PT e EN para os componentes
citados, que o usuário confirma como qualquer sugestão do mapa.

## Recomendação

**Revista após conferir o mapa real:** a técnica 2 passa a vir primeiro. A técnica 1 vem
depois dela e da proposta de apelidos, quando os mapas tiverem nomes para casar.

1. **Sementes pela menção** (técnica 1): a mais barata e a que usa o grafo como centro da
   memória. Ataca o ruído, que é o que segura a precisão.
2. **Expansão na extração** (técnica 2): resolve paráfrase pagando na captura, não em cada
   prompt. Exige migração (coluna de termos e índice FTS refeito), campo novo na saída da
   extração e uma tarefa em lote, na fila de sugestões, com o mesmo consentimento e limite
   diário do roteador, para as decisões que já existem.
3. **PageRank personalizado** (técnica 3), só se 1 e 2 deixarem ruído.
4. Embeddings (passo 5 do plano anterior) só depois, se a meta ainda não for batida.

Descartadas por ora: BM25 próprio (sem problema de pontuação a resolver), RM3 (amplia o
ruído em coleção pequena) e query2doc (custo por prompt).
