# Embeddings na seleção de contexto

**Data:** 05/10/2026.
**Pergunta:** um modelo de embeddings local, barato o bastante para rodar a cada prompt,
tira a seleção de contexto do teto lexical medido no corpus v4?
**Status:** Aberta. Desenho e protocolo; nenhum modelo baixado ou medido ainda. Continua o
passo 5 de [precisão do contexto](precisao-do-contexto.md).

## Ponto de partida

O corpus v4, escrito às cegas e selado, mede a seleção atual em 0,58 de precisão, 0,62
de cobertura e 4 de 21 casos negativos contaminados (v3: 0,94 / 0,95 / 0). As regras
lexicais depois da ponte PT/EN renderam quatro pontos de precisão no v4 e nenhum de
cobertura ([qualidade do núcleo](../arquitetura/qualidade-do-nucleo.md)). Faltam as
paráfrases sem palavra em comum e o sentido de "do que a tarefa fala", que o léxico não
alcança sem decorar o corpus.

As pesquisas de 30/09 ([embeddings e reranking locais](embeddings-e-reranking-local.md),
[metodologia do benchmark](metodologia-benchmark-semantico.md),
[memória semântica local first](memoria-semantica-local-first.md)) escolheram
`multilingual-e5-small` e EmbeddingGemma via FastEmbed e ONNX Runtime, pensando também na
biblioteca de PDFs. Para o bloco injetado a cada prompt o critério é outro: custo por
consulta perto de zero, nada de runtime nativo (o Smart App Control já bloqueou DLLs
neste Windows) e memória em repouso baixa.

## Candidatos

| Modelo | Como roda | Custo por consulta | Peso | Risco |
| --- | --- | --- | --- | --- |
| [`potion-multilingual-128M`](https://huggingface.co/minishlab/potion-multilingual-128M) (Model2Vec, destilado do BGE-M3, 101 idiomas, 256 dimensões, MIT) | [`model2vec-rs`](https://github.com/MinishLab/model2vec-rs) (MIT): tokenizador e tabela de vetores, Rust puro com `default-features = false` e `fancy-regex` | consulta na tabela e média: milhares de textos por segundo numa thread, segundo o projeto | 512 MB em f32 (`model.safetensors`) e 18,6 MB de tokenizador; a crate lê f16 e i8 (cerca de 128 MB em i8), e o vocabulário pode ser podado | modelo estático não entende ordem nem negação; MTEB médio 47,3, abaixo de modelos de atenção |
| `multilingual-e5-small` (12 camadas, 384 dimensões, MIT) | ONNX Runtime (FastEmbed) ou Candle | milissegundos por consulta em CPU | cerca de 118 MB em int8 | DLL nativa do ONNX Runtime no Windows; mais CPU por prompt |
| Embeddings do provedor | chamada HTTP | uma chamada por prompt | nenhum local | fere o pilar de custo e a latência da injeção; fica fora |

Começa pelo `potion`: se o modelo estático já fechar a distância, o e5 não paga o custo.
O e5 entra só se o `potion` falhar no v4, medido do mesmo jeito.

## Onde o vetor entra

A busca lexical continua sendo a porta: é barata, explicável e já passa no v3. O vetor
entra em dois pontos, cada um medido separado:

1. **Veto semântico (precisão).** Um item achado só por texto fica se a similaridade dele
   com a tarefa chegar a uma fração da melhor similaridade entre os candidatos.
2. **Resgate semântico (cobertura).** Quando o texto não acha nada que cubra a tarefa, a
   decisão mais parecida entra se passar de um piso absoluto e tiver folga sobre a segunda.

Fusão por Reciprocal Rank Fusion (técnica 7 da pesquisa de precisão) é a terceira variante,
se as duas primeiras não bastarem. Ligações do grafo (arquivos tocados) não passam pelo
vetor: o grafo segue sendo o centro.

Na produção, o vetor da decisão é calculado na adoção (como os termos de busca) e guardado
numa tabela própria, 256 valores por decisão; o da tarefa é calculado a cada prompt. A
`application` ganha uma porta `TextEmbedder`; o adaptador com o modelo fica numa crate
própria e é opcional: sem modelo, a seleção é a de hoje.

## Como medir sem se enganar

- **Vetores como fixture.** Um gerador fora do workspace (`tools/`, crate própria, para o
  build do app não pagar o tokenizador) calcula uma vez os vetores das decisões do corpus,
  com o texto guardado, e das 159 tarefas do v3 e do v4, e grava um fixture com o modelo
  anotado, como `context_corpus_terms.json`. Nunca editado à mão.
- **Ajuste só no v3.** Frações e pisos são escolhidos olhando o v3; o v4 só é lido no fim,
  no agregado, e cada variante tentada é registrada, inclusive as que perderem.
- **Aceite:** no v4, precisão e cobertura de pelo menos 0,70 sem mais casos contaminados
  que hoje; o v3 sem cair abaixo de 0,90 / 0,90; `build_pack` com o vetor da tarefa
  pronto sem passar de 5 ms no p95; o custo do vetor da tarefa e a memória do modelo
  carregado medidos e informados. O dogfood (`dogfood_context.rs`) confirma na máquina do
  usuário.
- Se nenhuma variante passar, o resultado é registrado e o modelo não entra.

## Para começar

Baixar o `model2vec-rs` e dependências do crates.io e, do Hugging Face,
`minishlab/potion-multilingual-128M` (`model.safetensors` 512 MB, `tokenizer.json`
18,6 MB, `config.json`) para uma pasta fora do repositório. Os downloads dependem de
autorização do usuário.
