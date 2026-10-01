# Metodologia do benchmark semântico local

**Data:** 2026-09-30.
**Pergunta:** como comparar E5-small e EmbeddingGemma Q4 em FastEmbed Rust,
com SQLite/FTS5/sqlite-vec e LanceDB embedded, para Windows de 8 GB sem GPU?
**Status:** Aberta. Protocolo executado; [resultados e artefatos](benchmark-semantico-local.md)
registrados separadamente. Não constitui aprovação da stack do produto.

## Escopo e hipóteses

O experimento usa somente `MultilingualE5Small` e `EmbeddingGemma300MQ4`, ambos
via FastEmbed Rust e ONNX Runtime CPU. Não inclui worker Python, reranker,
extração de PDF, OCR, LLM nem criação automática de relações de decisão.

São duas perguntas diferentes: o modelo recupera o contexto relevante, e o
armazenamento devolve os vizinhos com latência e memória aceitáveis? Um índice
correto pode recuperar perfeitamente os vizinhos de um embedding ruim. Por isso,
qualidade semântica e fidelidade do índice devem aparecer em tabelas separadas.
Essa separação adapta a avaliação de relevância do [BEIR](https://arxiv.org/abs/2104.08663)
e a avaliação de recall versus desempenho do
[ANN-Benchmarks](https://arxiv.org/abs/1807.05614).

O alvo de 8 GB é memória total da máquina. Uma execução em máquina de 32 GB
permite medir a memória do processo, mas não demonstra comportamento num
Windows de 8 GB com o app, editor e outros programas abertos. Limitar um
processo também não reproduz a disputa de cache e paginação desse sistema.

## Configuração correta dos modelos

| Item | E5-small | EmbeddingGemma Q4 |
| --- | --- | --- |
| Variante FastEmbed | `MultilingualE5Small` | `EmbeddingGemma300MQ4` |
| Dimensão original | 384 | 768 |
| Query de recuperação | `query: {texto}` | `task: search result \| query: {texto}` |
| Texto indexado | `passage: {texto}` | `title: none \| text: {texto}` |
| Artefato do catálogo | `intfloat/multilingual-e5-small/onnx/model.onnx` | `onnx-community/embeddinggemma-300m-ONNX/onnx/model_q4.onnx` e `model_q4.onnx_data` |

Os prefixos E5 permanecem em inglês inclusive para entradas em português. Seu
exemplo oficial usa mean pooling mascarado e normalização L2, e limita entradas
a 512 tokens. [Model card E5](https://huggingface.co/intfloat/multilingual-e5-small).
Os prompts Gemma distinguem consulta e texto; o prompt `code retrieval` destina-se
a recuperar blocos de código, enquanto decisões em prosa usam `search result`.
[Instruções de prompts Gemma](https://ai.google.dev/gemma/docs/embeddinggemma/inference-embeddinggemma-with-sentence-transformers).

O código `TextEmbedding::embed` do FastEmbed não acrescenta esses prompts. O
harness precisa formatar as entradas. O catálogo seleciona mean pooling e,
para Gemma, o output `sentence_embedding`; o transformador de saída normaliza
os vetores. Q4 usa `QuantizationMode::None` internamente por ser quantização
estática compatível com batches, não por ser um modelo de precisão completa.
[Implementação TextEmbedding](https://github.com/Anush008/fastembed-rs/blob/main/src/text_embedding/impl.rs),
[catálogo](https://github.com/Anush008/fastembed-rs/blob/main/src/models/text_embedding.rs)
e [saída](https://github.com/Anush008/fastembed-rs/blob/main/src/text_embedding/output.rs).

Definir `max_length=512` explicitamente para o comparativo comum, incluindo
prompt e tokens especiais. Registrar comprimento real de cada entrada com o
tokenizer específico: mesmos bytes não significam mesmos tokens. Casos próximos
ao limite devem informar se houve truncamento. Batch padding usa a maior entrada
do lote; misturar textos curtos e longos pode aumentar custo.
[Código do tokenizer FastEmbed](https://github.com/Anush008/fastembed-rs/blob/main/src/common.rs).

Gemma pode ter uma configuração adicional de saída em 256 dimensões, truncando
o vetor e normalizando novamente, identificada como variante do mesmo modelo.
Isso reduz tamanho do índice, não o trabalho principal de inferência. Não
truncar E5 por analogia, nem assumir que 384 dimensões são uma configuração
Matryoshka publicada para Gemma. Os resultados públicos de Gemma incluem
768/512/256/128 dimensões; a tabela QAT Q4_0 não prova a qualidade do artefato
ONNX-community Q4 usado aqui.
[Model card Gemma](https://ai.google.dev/gemma/docs/embeddinggemma/model_card).

## Corpus e julgamentos auditáveis

Criar fixtures versionadas de decisões de arquitetura e código, sem ler o banco
pessoal do usuário. Consultas devem conter paráfrases em português, consultas
em inglês sobre textos em português e vice-versa, identificadores exatos,
termos parecidos em componentes distintos e decisões com alternativas opostas.

Antes da primeira execução, congelar consultas, textos e julgamentos `qrels`.
Cada relevância deve se justificar pelo conteúdo do texto, não pelo resultado
do modelo. Notas possíveis: 0 irrelevante, 1 relacionado, 2 diretamente útil.
Não ajustar os textos ou julgamentos para favorecer o modelo vencedor.

Uma fixture pequena feita nesta sessão é uma avaliação de engenharia do domínio,
com julgamento editorial. Não se apresenta como BEIR/MTEB oficial ou prova de
generalização. Uma consulta sem relevante precisa avaliação distinta: top-k
sempre pode devolver resultados; ausência de resposta não se detecta apenas
com Recall@k.

Separar corpus de qualidade e corpus de escala. Ampliar um corpus com vetores
sintéticos ou cópias permite exercitar I/O e busca, mas não demonstra qualidade
semântica em 100 mil decisões reais. Repetições geram empates; recall de ANN
nesses casos deve aceitar equivalência de distância ou registrar a regra de
desempate e a limitação.

## Métricas semânticas

Calcular por consulta, depois média macro, sem dar peso extra às consultas
com mais relevantes. Publicar os rankings e qrels que permitem recalcular:

- Recall@5 e Recall@10: relevantes encontrados dividido pelo total relevante.
- MRR@10: inverso da posição do primeiro relevante; zero se não encontrado.
- nDCG@10: DCG com ganho `2^relevancia - 1` e desconto `log2(posicao + 1)`,
  dividido pelo DCG ideal dessa consulta.
- Resultados individuais e cortes por idioma/tipo de consulta, quando a
  quantidade de exemplos permitir interpretar esses cortes.

Essas escolhas são inspiradas nas métricas de recuperação do
[BEIR](https://github.com/beir-cellar/beir). Repetir a mesma query 100 vezes melhora
a medida de tempo, mas continua sendo um único exemplo de qualidade.

## Comparação dos armazenamentos

Reutilizar exatamente os mesmos vetores serializados, textos, IDs, consultas e
top-k nos dois armazenamentos. Comparar SQLite `vec0` com LanceDB exhaustive
primeiro; ANN é uma linha adicional com seu próprio recall e custo de construção.

### SQLite

Usar SQLite bundled, FTS5 e extensão `sqlite-vec`, registrando versões efetivas.
Não trocar a extensão por uma função de distância escrita no harness. A
integração Rust documentada registra `sqlite3_vec_init` via
`sqlite3_auto_extension` antes da conexão.
[Integração Rust sqlite-vec](https://alexgarcia.xyz/sqlite-vec/rust.html).

Criar `vec0(embedding float[D] distance_metric=cosine)` e consultar com
`embedding MATCH ? AND k = ?`. O default da extensão é L2, portanto declarar
cosine é necessário. O `k` explícito evita dependência da interpretação de
`LIMIT` que a documentação condiciona a SQLite 3.41+.
[KNN documentado](https://alexgarcia.xyz/sqlite-vec/features/knn.html).

FTS5 deve usar `unicode61 remove_diacritics 2`, sem Porter inglês, e ordenar
`bm25(...)` ascendente, com ID como desempate. Esse BM25 atribui valores menores
a melhores resultados. O corpus lexical não contém os prompts dos modelos.
[FTS5 oficial](https://sqlite.org/fts5.html).

### LanceDB embedded

Usar URI local e SDK Rust. Busca vetorial declara `DistanceType::Cosine` e
`.bypass_vector_index()` na linha exhaustive. Não depender da ausência de
índice como garantia implícita.
[VectorQuery](https://docs.rs/lancedb/latest/lancedb/query/struct.VectorQuery.html).

FTS usa `Index::FTS(FtsIndexBuilder::default())` sobre a coluna de texto,
consultado por `.full_text_search(FullTextSearchQuery::new(...))`. A documentação
atual descreve FTS nativo de Lance com BM25; exemplos legados Tantivy não
descrevem necessariamente o caminho atual do SDK.
[FTS oficial com exemplo Rust](https://docs.lancedb.com/search/full-text-search).
Desabilitar stemming e remoção de stopwords em inglês, mantendo lowercase e
ASCII folding, para aproximar o baseline Unicode do SQLite. Ainda assim,
tokenizers e variantes BM25 podem diferir: comparar qualidade, não exigir
rankings lexicais idênticos.
[FtsIndexBuilder](https://docs.rs/lancedb/latest/lancedb/index/scalar/struct.FtsIndexBuilder.html).

Se executar ANN, registrar tipo de índice, partições, amostragem, `nprobes`,
parâmetros de refinamento e seed quando a API permitir. `IVF_FLAT` separa efeito
da seleção de partições de uma segunda quantização dos vetores; tem custo de
treinamento e memória. A métrica de construção deve ser a mesma da busca.
[IvfFlatIndexBuilder](https://docs.rs/lancedb/latest/lancedb/index/vector/struct.IvfFlatIndexBuilder.html).

ANN recall@10 mede interseção entre top-10 aproximado e top-10 exhaustive,
dividida por 10. Não é o Recall@10 de relevância dos qrels. Publicar os dois
nomes por extenso para não confundir.

### Híbrido

Medir lexical, vetorial e híbrido separadamente. Para um comparativo controlado,
aplicar a mesma função RRF no harness aos top-20 lexical e vetorial de cada
engine: `score(d) = sum(1 / (60 + rank(d)))`, ranks começando em 1 e desempate
estável por ID. Esses valores são escolhas do experimento, não limites do
produto. Se medir híbrido nativo LanceDB, identificá-lo como configuração
adicional; não misturar seu tempo com o híbrido controlado.

Definir a mesma semântica lexical nos dois lados, por exemplo OR entre termos
escapados da consulta. Não entregar texto natural à gramática FTS5 sem escape.
Filtragem por projeto deve ocorrer antes do top-k; filtrar depois pode perder
todos os candidatos válidos. Casos filtrados ficam separados dos não filtrados.

## Tempos e recursos

Registrar build release, lockfile, versão Rust, Windows, CPU, núcleos físicos e
lógicos, RAM, disco, energia e threads configuradas. Desabilitar GPU e medir
um modelo por processo. Compilação/download não entram no tempo de consulta.

Medir separadamente:

1. Primeiro download e bytes totais do cache, incluindo tokenizer e arquivos
   externos ONNX. Registrar repo/revisão SHA, caminhos e hash dos artefatos.
2. Inicialização de um processo novo com cache de modelos já presente e primeira
   inferência. Isso é processo frio; não significa cache frio do Windows.
3. Consulta curta com batch 1, corpus com batch 1 e batch 8, threads 1 e 4.
4. Construção/população dos armazenamentos e de cada índice, disco persistido
   após flush/checkpoint e memória máxima da construção.
5. Busca lexical/vetorial/híbrida com embedding da query pré-calculado, incluindo
   materialização dos resultados. Reportar p50, p95, amostras e throughput.
6. Pipeline query→embedding→busca, medido inteiro; não somar p95 separados.

Usar warmup explícito excluído das estatísticas, ordem pseudoaleatória com seed
fixa, múltiplas consultas diferentes e pelo menos 100 medições aquecidas por
configuração. Repetir configurações em processos novos quando possível. Salvar
amostras brutas e fórmula de percentil. Uma amostra pequena não permite afirmar
um p99 de produção nem uma diferença de décimos de milissegundo consistente.

Capturar memória resident/working set, pico working set e private bytes, e
identificar se o pico foi amostrado ou obtido do contador do sistema. A API
Windows `GetProcessMemoryInfo` expõe dados do processo; RSS não equivale a
consumo total da máquina ou tamanho do download.
[Documentação Microsoft](https://learn.microsoft.com/en-us/windows/win32/psapi/process-memory-usage-information).

Cache do sistema, compilação concorrente, antivirus, temperatura e outros
programas podem alterar o tempo. Não rodar compile e medições simultaneamente.
Manter a configuração de durabilidade dos bancos registrada: uma transação
SQLite por lote não pode ser comparada a fsync a cada linha no outro banco sem
explicar que se estão avaliando configurações distintas.

## Critério de interpretação

O resultado deve oferecer latência, memória, disco e qualidade juntos. Não
escolher o vencedor pela latência ANN quando ela sacrifica recall sem informar.
Para 8 GB, cabe recomendar um orçamento provisório de memória e batch pequeno,
mas aprovação do mínimo suportado exige executar em hardware com esse limite e
com o app completo. Downloads/cache devem permanecer fora de Git; fixtures,
código, lockfile, resultados brutos compactos e relatório ficam versionados.

As APIs acima foram conferidas em fontes primárias em 2026-09-30. Links `main`
e `latest` explicam a pesquisa; o benchmark executável deve fixar suas próprias
versões e registrar divergências entre a versão instalada e essa documentação.

## Harness reproduzível desta execução

O workspace independente `tools/semantic-bench` não altera dependências nem
comportamento do app. A execução é Rust, com FastEmbed 7.1.0, sqlite-vec 0.1.9,
rusqlite 0.40.2, LanceDB 0.39.0 e ONNX Runtime CPU 1.30.0 carregado dinamicamente.
O lockfile fixa as dependências transitivas e `prepare-models.sh` fixa as duas
revisões dos modelos, populando apenas o cache do experimento. O script confere o SHA-256 do
runtime oficial antes de carregar a biblioteca; não há worker Python.

LanceDB 0.39.0 referencia `Error::Http` em `job.rs` mesmo quando a variante está
desabilitada sem `remote`. A primeira tentativa de habilitar essa feature
ampliou o grafo de dependências e foi interrompida antes da medição. A execução
usa uma cópia isolada do tarball publicado, com SHA-256 conferido, e um patch
versionado que condiciona somente os dois ramos de erro HTTP à feature correta.
Não altera busca, métricas, indexação ou persistência. A dependência efetiva é
**LanceDB 0.39.0 com patch de compilação embedded**, não a release intacta.
`prepare-lancedb.sh` gera essa cópia fora do Git, em `/bench/lancedb-patched`;
o build passa `--config 'patch.crates-io.lancedb.path="/bench/lancedb-patched"'`.
Testes e Clippy precisam do mesmo override. É uma limitação de integração da
release, não uma exigência de hospedagem. Adoção no produto ainda deve selecionar
uma release upstream corrigida ou avaliar explicitamente carregar esse patch.

```powershell
& tools/semantic-bench/run.ps1 -WorkDirectory D:\xemnas-semantic-bench
& tools/semantic-bench/summarize.ps1 -RunDirectory D:\xemnas-semantic-bench\run-AAAAmmdd-HHMMSS
```

Docker precisa estar disponível. O build usa cache Cargo em volume nativo e
artefatos no diretório de trabalho. Os modelos são preparados com rede; a etapa
medida desliga a rede, limita CPU a quatro equivalentes e memória a 8 GiB,
sem swap. Os bancos ficam em volume Linux nativo, para que o bind NTFS do
Windows não domine artificialmente a comparação. Cache dos modelos e resultados
ficam no diretório de trabalho. `-SkipBuild` reutiliza o binário existente e só
deve ser usado depois de compilar exatamente a versão que se pretende medir.

A fixture congelada contém 48 decisões fictícias, 24 consultas PT/EN e qrels
binários (duas decisões relevantes por consulta), incluindo alternativas opostas.
É julgamento editorial, não um teste de inferência de relações causais. A escala
de 1 mil/10 mil/100 mil linhas repete esses textos e perturba deterministicamente
os vetores normalizados; só mede armazenamento. Não corresponde a cem mil
decisões semanticamente distintas. A qualidade pública usa as versões completas
PT e EN do Belebele convertido para retrieval pelo MTEB: 488 passagens e 900
consultas por idioma. O script fixa revisão e hashes dos arquivos originais,
preserva atribuição e licença, e não implementa ingestão de documentos do app.

O harness usa Recall@10, MRR@10 e nDCG@10 binários, além de acerto e alternativa
oposta na primeira posição da fixture. Recall@5 e julgamentos graduados são
possibilidades do protocolo acima, não métricas executadas nesta versão.
Consultas percorrem uma rotação determinística, em vez de um sorteio. Há três
processos por modelo em quatro threads e por caso de armazenamento; os casos
de uma thread e os pipelines completos têm um processo cada. A inferência curta
tem 120 amostras medidas por processo; cada operação de armazenamento, 240;
cada perfil longo, 20. Batch 8 tem três tempos de corpus por processo e informa
média, sem percentis. Percentis usam nearest rank, e o resumo publica mediana,
mínimo e máximo dos valores obtidos pelos três processos, sem juntar amostras
como se fossem independentes. O pipeline mede embedding e buscas sequenciais
com fusão RRF; a linha LanceDB desse pipeline é exhaustive, não ANN.

O build nativo Windows foi bloqueado pelo Smart App Control ao carregar uma
DLL de proc-macro. A medição Linux/WSL2 é uma alternativa documentada, não
validação de distribuição no Windows nem certificação do hardware mínimo.
O relatório de resultados deve preservar essa limitação. Os testes automatizados
verificam métricas/RRF, vetores sintéticos e integração real sqlite-vec/FTS5;
o script de resumo valida casos, amostras e IDs e informa concordância entre
os top-10 exatos das duas engines.
