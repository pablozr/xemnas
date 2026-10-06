# Busca semântica local: o que foi considerado, medido e por que não entrou

**Data:** 30/09/2026 a 06/10/2026 (consolidada em 06/10/2026).
**Pergunta:** embeddings e busca vetorial locais, rodando num Windows de 8 GB sem GPU,
melhoram o contexto entregue ao agente e servem de base para uma futura biblioteca de
PDFs e livros?
**Status:** Aberta. Para o contexto, **reprovados**: nenhum modelo pequeno separou as
decisões do projeto no corpus selado. A biblioteca de documentos não foi iniciada.

Esta nota substitui seis pesquisas (`embeddings-no-contexto`,
`metodologia-benchmark-semantico`, `memoria-semantica-local-first`,
`embeddings-e-reranking-local`, `biblioteca-e-busca-local`, `solucoes-prontas-rag-local`)
e a ferramenta `tools/semantic-bench`. As versões completas, com todas as fontes, estão no
histórico do git até o commit que as removeu.

## O que foi medido no contexto (05 e 06/10/2026)

Protocolo: vetores gerados uma vez como fixture, limiares ajustados só nos corpora de
ajuste (v3 e v5, escritos às cegas) e o v4 selado lido uma vez, no agregado. Aceite: v4
com precisão e cobertura de pelo menos 0,70, sem mais casos negativos contaminados.

| Modelo | Como rodou | Custo por tarefa | Melhor resultado no v4 (precisão / cobertura / contaminados) |
| --- | --- | --- | --- |
| Só a busca lexical (referência) | | | 0,58 / 0,62 / 4 de 21 |
| [`potion-multilingual-128M`](https://huggingface.co/minishlab/potion-multilingual-128M), estático | [`model2vec-rs`](https://github.com/MinishLab/model2vec-rs), Rust puro | 0,1 ms p50; carga de 3 s, pico de 1,5 GB | veto: 0,67 / 0,60 / 4 |
| [`multilingual-e5-small`](https://huggingface.co/intfloat/multilingual-e5-small) | Candle, Rust puro | 25 ms p50, 55 ms p95; pico de 1,15 GB | veto: 0,60 / 0,60 / 4 |

Também foram testados resgate (o mais parecido entra quando o léxico não acha nada),
troca (o vetor substitui o item lexical) e seleção só pelo vetor: nenhuma variante que
mantinha o v3 acima de 0,90 melhorou o v5, e só pelo vetor ficou abaixo do léxico.

**Por quê:** as decisões de um projeto dividem o mesmo vocabulário. No v5 o e5 pôs a
decisão certa em primeiro em 27 de 65 tarefas, e a folga entre o primeiro e o segundo nos
negativos (até 0,03) se sobrepunha à dos acertos. O que melhorou o contexto depois veio do
grafo: ligações por menção e pela IA, e regras com escopo.

## O que foi considerado para a biblioteca de documentos (30/09/2026)

- **Modelos:** integrar um modelo pré-treinado, nunca treinar. Ponto de partida
  multilingual-e5-small versus EmbeddingGemma quantizado; BGE-M3 e Qwen3-Embedding como
  candidatos de qualidade, não como mínimo do desktop.
- **Runtime:** FastEmbed/ONNX Runtime em CPU resolveria sem Python, mas a DLL nativa do
  ONNX Runtime foi bloqueada pelo Smart App Control neste Windows, e o benchmark nunca
  rodou. Candle e Model2Vec rodam em Rust puro.
- **Armazenamento:** sqlite-vec com o FTS existente como linha de base; LanceDB embedded
  para uma biblioteca maior; Qdrant Edge e Vec1 como trilhas experimentais.
- **Produtos prontos** (AnythingLLM e outros): úteis como referência, mas importar o
  backend traria contas, armazenamento e políticas que duplicam o Xemnas. Manter o domínio
  e o grafo em Rust e escolher componentes substituíveis.
- **Python local opcional** para OCR e documentos, por IPC em loopback, sem servidor
  hospedado.
- **Orçamento:** 8 GB é o total da máquina com sistema, IDE e app; um job de inferência
  por vez, lote pequeno, medir pico de RAM antes de prometer qualquer coisa.

## O que reabriria o assunto

- Um corpus em que a busca lexical e o grafo falhem e a semântica seja o que falta, medido
  como acima.
- A biblioteca de PDFs e livros entrar no roadmap: aí o protocolo de benchmark (latência,
  memória, disco e recall juntos, na máquina de 8 GB) volta a valer, partindo de
  sqlite-vec e de um modelo em Rust puro.
