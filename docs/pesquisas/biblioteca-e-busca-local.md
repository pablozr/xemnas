# Biblioteca de conhecimento e busca local

**Data:** 2026-09-30
**Status:** Aberta
**Pergunta:** quais tecnologias existentes permitem busca híbrida e ingestão de
PDFs/livros no desktop Rust/Windows, incluindo máquinas com 8 GB de RAM e CPU
sem GPU dedicada, sem exigir infraestrutura hospedada?

Esta pesquisa complementa a [auditoria do grafo](automacao-do-grafo-fontes.md).
Integra a [síntese de arquitetura e custos](memoria-semantica-local-first.md).
É uma comparação para orientar um experimento, não uma troca da stack vigente.
Não foram instalados bancos, modelos ou parsers nem medidos tempos nesta pesquisa.

## Encaixe no produto

O [vocabulário](../produto/CONTEXT.md) já distingue Knowledge Library, Knowledge
Source e Project Knowledge Link. A biblioteca é global ao usuário e pode informar
vários Projects. Uma recomendação de livro não vira Context Claim ou Engineering
Decision vigente automaticamente. Vincular uma fonte a um Project significa
disponibilidade para consulta, não adoção de todas as suas recomendações.

A [stack vigente](../arquitetura/stack-e-arquitetura-rust-gpui.md) determina núcleo
Rust, SQLite/rusqlite como fonte estruturada e arquivos grandes no content store.
Vetores são uma evolução futura. `crates/storage-sqlite/Cargo.toml` usa rusqlite
0.40 com `bundled`; não existe incompatibilidade demonstrada com sqlite-vec.

Proposta: manter decisões, relações, vigência, fontes e citações canônicas em
SQLite; o índice vetorial é uma projeção reconstruível. Assim o mecanismo de
busca pode mudar sem mudar o significado do domínio.

## O mercado: banco, índice e geração são coisas diferentes

Um modelo gera embeddings; um índice procura vetores próximos; o catálogo guarda
documentos, autoria, permissões e citações; a busca combina resultados. Escolher
um banco vetorial não fornece automaticamente leitura de PDFs, geração local ou
relações corretas entre decisões.

| Opção | Forma local e integração | Ganhos | Limitações para xemnas |
| --- | --- | --- | --- |
| sqlite-vec | Extensão C embutida em SQLite, bindings Rust/Python, Windows | Reaproveita banco existente, SQL e filtros, sem daemon | Pré-1.0; fixar versão e testar migração, recall e custo das consultas |
| SQLite Vec1 | Extensão ANN em C; build MSVC documentado | ANN próximo ao catálogo SQLite, sem serviço | Versão 0.7; documentação admite testes insuficientes; IVFADC envolve treino; instruções SIMD precisam respeitar CPU alvo |
| LanceDB OSS | Embedded local, SDKs Rust/Python/TS | Vetores, metadados, FTS, híbrida, índices e versionamento de tabelas | Segundo formato de armazenamento; manutenção, versões antigas e publicação atômica da projeção precisam ser definidas |
| Qdrant Edge | Biblioteca embedded Rust/Python | HNSW, híbrida, filtros, quantização, funcionamento offline | Beta; otimização/indexação manual síncrona; exige worker e recuperação bem definidas |
| QdrantClient local | Implementação local do client Python, persistência por path | Experimentos pequenos e compatibilidade com API de servidor | Busca exata brute force; docs/código orientam usar server para mais escala; não confundir com Edge |
| Qdrant Server | Processo/servidor local ou cloud, API Rust gRPC | Operação e indexação automática, caminho para uso compartilhado | Processo/serviço adicional; quickstart Docker não é bom requisito obrigatório para desktop Windows de 8 GB |
| Chroma PersistentClient | Persistência local via Python | Ecossistema de protótipos RAG, documentos e metadados | Referência oficial orienta uso local/desenvolvimento e server para produção; empacotamento Python e acesso Rust precisam ser resolvidos |
| FAISS | Biblioteca C++ com interface Python, CPU/GPU | Índices exatos e ANN bem conhecidos | É biblioteca de índices; catálogo, metadados, transações, exclusões e recuperação ficam sob responsabilidade do app |

Fontes primárias da tabela: [sqlite-vec](https://github.com/asg017/sqlite-vec),
[Vec1 e roadmap](https://sqlite.org/vec1/doc/trunk/doc/vec1.md),
[LanceDB OSS](https://github.com/lancedb/lancedb),
[Qdrant Edge](https://qdrant.tech/documentation/edge/),
[comparação Edge/Server](https://qdrant.tech/documentation/edge/edge-vs-qdrant-cluster/),
[código QdrantLocal](https://github.com/qdrant/qdrant-client/blob/master/qdrant_client/local/qdrant_local.py),
[quickstart Qdrant](https://qdrant.tech/documentation/quickstart/),
[referência Chroma](https://docs.trychroma.com/reference/python),
[FAISS](https://github.com/facebookresearch/faiss).

### Pontos que uma comparação antiga perderia

sqlite-vec documenta `vec0` KNN e consultas escalares exatas. O repositório atual
também tem arquivos DiskANN e IVF: não é correto afirmar que nunca poderá ter
ANN. A pesquisa não demonstrou que esses caminhos constituam uma API estável de
produção. O benchmark inicial deve fixar release e implementação realmente usada.
[KNN documentado](https://alexgarcia.xyz/sqlite-vec/features/knn.html),
[código DiskANN](https://github.com/asg017/sqlite-vec/blob/main/sqlite-vec-diskann.c).

sqlite-vec oferece integração direta com rusqlite bundled, compilando a extensão
C estaticamente e registrando-a na conexão. Antes de escolher, validar a combinação
exata no Windows e a extensão registrada em cada conexão necessária; não instalar
um segundo SQLite arbitrariamente. A consulta com `LIMIT` exige SQLite 3.41+;
`k = N` é a alternativa documentada.
[Integração Rust](https://alexgarcia.xyz/sqlite-vec/rust.html),
[consultas](https://alexgarcia.xyz/sqlite-vec/features/knn.html).

Qdrant Edge é distinto do local mode do client. Edge tem engine compartilhado com
Server e ANN HNSW; `optimize()` é manual e síncrono, com pontos novos pesquisáveis
por brute force até indexação. Portanto precisa rodar fora da thread da UI. A
documentação marca beta. Há wheel Python oficial 0.8.0 para Windows x86-64,
Apache-2.0; tamanho do wheel de 11,2 MB não é consumo de memória em execução nem
prova de que a integração Rust já compila nesta stack.
[Operações Edge](https://qdrant.tech/documentation/edge/edge-vs-qdrant-cluster/),
[distribuição Windows](https://pypi.org/project/qdrant-edge-py/).

LanceDB demonstra busca híbrida com vetores e FTS, filtros e fusão RRF. A documentação
possui exemplos mais completos em Python/TS; a paridade necessária em Rust deve
ser verificada no spike, não presumida por existir SDK. RRF é fusão de rankings,
não um modelo neural que entende a relação entre duas decisões.
[Busca híbrida](https://docs.lancedb.com/search/hybrid-search),
[SDK Rust](https://docs.rs/lancedb/latest/lancedb/index.html).

## PDFs e livros: extração antes de embeddings

PDF pode conter texto nativo ou apenas imagens. Extrair texto de imagem exige OCR;
extrair as palavras não garante ordem correta de leitura em colunas ou tabelas.
O caminho rápido e o caminho de alta fidelidade precisam ser separados.

| Ferramenta | Uso proposto | Custo operacional local e limite |
| --- | --- | --- |
| pdf-extract | Primeiro experimento Rust para PDF com texto nativo | Extrai por página; não fornece OCR/entendimento de layout por si; medir livros com colunas e fontes incomuns |
| pdfium-render | Extração/posição e renderização Rust para citações e preview | Empacotar Pdfium DLL compatível no Windows; wrapper não inclui engine; não resolve OCR por si |
| Docling | Worker Python opcional para tabelas, layouts complexos, scans e vários formatos | Modelos e dependências maiores; CPU suportada, GPU opcional; pré-baixar artefatos para offline; executar sob demanda |
| PyMuPDF/PyMuPDF4LLM | Alternativa Python para texto, geometria, Markdown e OCR | AGPL ou licença comercial; definir compatibilidade de distribuição antes de escolher; separar repo não elimina automaticamente obrigações |
| Markdown/HTML/EPUB | Preferir conteúdo estruturado quando disponível | Preservar headings, capítulos, código e tabelas; EPUB não tem paginação física estável como PDF |

Fontes: [pdf-extract por página](https://docs.rs/pdf-extract/latest/pdf_extract/),
[pdfium-render e linking](https://docs.rs/pdfium-render/latest/pdfium_render/index.html),
[licença do wrapper MIT/Apache-2.0](https://docs.rs/crate/pdfium-render/latest/source/Cargo.toml),
[Docling offline e limites](https://docling-project.github.io/docling/usage/advanced_options/),
[aceleradores CPU/CUDA](https://docling-project.github.io/docling/_generated/examples/run_with_accelerator/),
[formatos Docling](https://docling-project.github.io/docling/usage/supported_formats/),
[PyMuPDF recursos e licença](https://pymupdf.readthedocs.io/en/latest/about.html).

Proposta para 8 GB: extração nativa por página primeiro; detectar página vazia ou
texto claramente ruim; oferecer OCR daquela página/documento, com progresso e
cancelamento. Um job de ingestão pesado por vez, batches pequenos e sem carregar
livro inteiro em memória por conveniência. Não manter Docling, embedder, reranker
e modelo generativo grandes todos residentes. Parser pesado pode encerrar após o
job; pesos permanecem em disco para a próxima execução. Limites de threads e
modelos compartilhados no tempo precisam de benchmark, pois descarregar reduz RAM
e acrescenta latência na próxima carga.

Docling tem pipeline nativo sem modelos de layout/OCR e opções para controlar
features, threads, tamanho e páginas. Serviços remotos exigem opt-in na API;
baixar pesos é uma operação distinta de transmitir documentos. Escolher explicitamente
as opções em vez de presumir que qualquer pipeline seja sempre offline.
[Opções avançadas](https://docling-project.github.io/docling/usage/advanced_options/).

## Pipeline e identidade reproduzível

Proposta de pipeline:

```text
arquivo original + hash → catálogo/versionamento → extração citável
    → limpeza → chunks estruturais → embeddings em batches
    → índice lexical + vetorial → publicação da geração pronta
```

Preservar os originais e a extração estruturada permite melhorar chunking/modelo
sem repetir OCR. Embeddings são gerados uma vez por texto/modelo/configuração,
não em toda consulta. Documentos alterados recebem nova versão; mudanças de modelo
geram outro índice, publicado apenas quando estiver consistente.

Metadados mínimos propostos por chunk:

- source ID, versão, hash do original, título, autoria, idioma, data e origem;
- PDF: índice físico da página e rótulo impresso quando conhecido, caixas/offsets;
  EPUB/HTML: capítulo, recurso e âncora/offset; Markdown: heading e linhas;
- extractor/version/options, hash da extração, chunker/version/options;
- texto original citável, texto contextualizado usado no embedding e hashes;
- model ID/revision, tokenizer, dimensão, tipo numérico, normalização, prefixos
  de query/document, quantização e versão da geração do índice;
- coleção, Project Knowledge Links e estado de exclusão/indexação.

Dimensão igual não implica espaços comparáveis: dois modelos de 768 dimensões
podem produzir coordenadas incompatíveis. Query e documentos precisam do mesmo
contrato de modelo, incluindo instruções/prefixos e normalização.

Chunking inicial proposto: respeitar capítulos, headings, parágrafos, blocos de
código e tabelas, com limite de tokens do modelo. Repetir cabeçalhos de tabelas
quando dividir. Não indexar um livro inteiro num vetor; não cortar cegamente a cada
N caracteres. O tamanho ideal deve ser avaliado com perguntas reais. Docling já
oferece HybridChunker estrutural e sensível a tokens; não precisamos implementar
essa técnica integralmente do zero.
[Chunkers oficiais](https://docling-project.github.io/docling/concepts/chunking/).

## Busca federada com autoridade preservada

Proposta para cada pergunta: buscar decisões/claims válidas do Project e trechos
da biblioteca autorizada como conjuntos distintos; combinar resultados com pesos
e budgets separados; rerank limitado; montar Context Pack com citações e tipos.
O assistente deve conseguir dizer: “o projeto adotou X; o livro recomenda Y”.
Uma distância vetorial não decide qual deles tem autoridade.

Filtros por Project, coleção, idioma, data e tipo devem acontecer antes ou durante
a recuperação. Um postfilter sobre top-k global pode devolver poucos resultados
e esconder uma decisão normativa. O catálogo define acesso/escopo mesmo quando o
índice usa outra tecnologia.

## Custo de armazenamento e memória

A conta abaixo é aritmética, não benchmark: `chunks × dimensões × 4 bytes` para
float32. MB usa base decimal; MiB usa 1.048.576 bytes.

| 100.000 chunks | Vetores crus em bytes | MB | MiB |
| --- | ---: | ---: | ---: |
| 384 dimensões | 153.600.000 | 153,6 | 146,5 |
| 768 dimensões | 307.200.000 | 307,2 | 293,0 |
| 1.024 dimensões | 409.600.000 | 409,6 | 390,6 |

Um milhão de chunks multiplica esses números por dez. Esses valores excluem
PDFs originais, texto, FTS, metadados, páginas/imagens OCR, estruturas ANN,
IDs, WAL, cache, versões antigas, cópias durante compactação e pesos dos modelos.
Não são o total de RAM nem o total de disco. Float16/int8/binário podem reduzir
armazenamento quando suportados, mas exigem medir perda de qualidade e overhead;
quantizar o modelo e quantizar o índice são operações diferentes.

Em máquina de 8 GB, Windows, UI, browser/agente e demais apps disputam memória.
Um banco menor não garante que OCR ou inferência caibam. Definir limite de cache,
fila, batch e concorrência e medir pico de working set incluindo subprocessos.
Manter vetores em disco não impede páginas acessadas de entrarem em cache de RAM.

Local embedded elimina aluguel mensal do banco; continua havendo custo de
desenvolvimento, distribuição de modelos/DLLs, disco, CPU, energia e suporte.
Servidor Python local também não exige hospedagem. Cloud é opção para sincronização
ou capacidade remota, introduzindo rede, privacidade e despesa recorrente.

## Latência e experimento recomendado

Separar ingestão de consulta. Tempo de consulta = embedding da pergunta + busca
lexical/vetorial + fusão + rerank + montagem do pack; resposta generativa acrescenta
outra etapa. Tempo de ingestão = extração/OCR + chunking + embeddings + indexação.
O custo de importar um livro não deve bloquear a aprovação de uma decisão.

Não há promessa de milissegundos ou aprovação instantânea nesta pesquisa. Benchmarks
publicados em hardware diferente não comprovam resultado na máquina mínima.
Medir carga fria e quente, p50/p95, pico de RAM, CPU, tamanho em disco, tempo de
importação, recall@k, relevância com citações e cancelamento/retomada.

Shortlist proposta:

1. **sqlite-vec + FTS existente** como baseline de integração mínima. Experimentar
   10 mil e 100 mil chunks, partições por Project/coleção e busca global.
2. **LanceDB embedded** como concorrente para biblioteca maior e ANN; confirmar
   Windows/Rust, filtros e publicação consistente de índice.
3. **Qdrant Edge** como concorrente adicional se beta for aceitável no experimento;
   validar pico durante `optimize()` e distribuição Rust no Windows.
4. **Vec1** em trilha experimental, pois ANN SQLite é interessante, mas o próprio
   roadmap ainda registra testes insuficientes. Não escolhê-lo só por ser SQLite.

Comparar com o mesmo modelo, chunks, corpus, perguntas e hardware. Primeiro provar
recuperação/citações e uso de RAM; depois otimizar. Chroma/QdrantClient local são
úteis para protótipo Python; Server/Cloud apenas quando houver motivo concreto.
FAISS vale quando controlar índice justificar construir toda a camada operacional.

Validação pendente: build e empacotamento real Windows, comportamento de crash e
backup, licenças transitivas/modelos, qualidade PT/EN/código e números na máquina
de 8 GB. A recomendação é condicional a esses resultados.
