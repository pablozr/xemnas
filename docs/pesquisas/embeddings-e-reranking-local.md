# Embeddings e reranking locais para o xemnas

**Data:** 2026-09-30.
**Status:** Aberta. Pesquisa; nenhum runtime ou modelo foi instalado ou validado no app.
**Pergunta:** Como executar embeddings e reranking no desktop Rust/Windows,
com custo e latência controlados, permitindo uma futura biblioteca de PDFs e livros?

Complementa a [síntese de arquitetura e custos](memoria-semantica-local-first.md).

## Conclusão proposta

Não precisamos implementar ou treinar uma rede de embeddings. Devemos integrar um
modelo pré-treinado e um runtime existente. Tokenização, pooling, prompts e
normalização fazem parte do contrato desse modelo; a infraestrutura própria trata
fila, cache, identidade dos vetores, armazenamento, seleção de candidatos e avaliação.

O requisito mínimo informado é **Windows com 8 GB de RAM e sem GPU dedicada**.
Nesse perfil, a primeira comparação deve ser
**multilingual-e5-small em ONNX CPU versus EmbeddingGemma quantizado**. E5 fornece
uma referência simples e multilíngue; EmbeddingGemma é uma alternativa recente
orientada a dispositivos, com contexto maior. BGE-M3 e Qwen3-Embedding-0.6B entram
como candidatos de qualidade, não como requisitos mínimos do desktop.
Essa é uma recomendação de engenharia inferida das opções abaixo, ainda sem benchmark.
SO, UI, banco, cache e editor/compilação coexistem nos mesmos 8 GB: o tamanho do
arquivo quantizado não é orçamento total de memória. Batch inicial deve ser pequeno,
com apenas um job de inferência; a capacidade real depende de medição. Modelos
4B/8B e residência simultânea de LLM, embeddings e reranker não são defaults propostos.

Rust + ONNX Runtime pode resolver produção sem Python. Um worker Python local,
inclusive publicado por outro repositório, continua uma opção legítima para
documentos, experimentação e modelos pouco suportados em Rust. Repositório separado
não implica servidor remoto ou cobrança de hospedagem de inferência.

## O que já existe

| Caminho | Capacidades verificadas | Consequência para o xemnas |
| --- | --- | --- |
| FastEmbed Rust + ONNX Runtime | Embeddings, reranking, arquivos locais, tokenizers e catálogo de modelos; DirectML documentado | Primeiro candidato para adapter Rust; precisa empacotar runtime/modelos e verificar versão publicada |
| FastEmbed Python + ONNX Runtime | Biblioteca de embeddings sem exigir PyTorch | Worker Python pode continuar relativamente focado; não elimina runtime nativo/modelos |
| Candle | Framework Rust com CPU e CUDA, modelos específicos e quantização | Útil para Qwen3; cobertura de arquitetura e aceleração precisam ser verificadas por modelo |
| Ollama | Embeddings por API local, batch, modelos gerenciados e distribuição Windows | Integração/protótipo simples; introduz outro processo e gerenciamento de versões |
| llama.cpp | CPU, quantização, CUDA, Vulkan/SYCL; build Windows documentado | Candidato para GGUF e futuro LLM local; integração por biblioteca ou processo controlado |
| Sentence Transformers | Embeddings e CrossEncoder; PyTorch e alternativas ONNX/OpenVINO | Melhor laboratório de qualidade e maior flexibilidade; bundle Python/PyTorch pode ficar maior |
| Text Embeddings Inference (TEI) | Serviço Rust, batching, CPU/GPUs; instalação local e containers | Bom servidor de inferência; não presumir bundle Windows simples com base no guia Linux/containers |

Fontes primárias: [FastEmbed Rust](https://github.com/Anush008/fastembed-rs),
[FastEmbed Python](https://github.com/qdrant/fastembed),
[Candle](https://github.com/huggingface/candle),
[Ollama embeddings](https://docs.ollama.com/capabilities/embeddings),
[llama.cpp](https://github.com/ggml-org/llama.cpp),
[build Windows do llama.cpp](https://github.com/ggml-org/llama.cpp/blob/master/docs/build.md),
[Sentence Transformers](https://sbert.net/docs/installation.html),
[backends e eficiência](https://sbert.net/docs/sentence_transformer/usage/efficiency.html),
[TEI](https://github.com/huggingface/text-embeddings-inference).

O `main` consultado do FastEmbed Rust declara 7.1.0, Rust mínimo 1.88 e
`ort` 2.0.0-rc.13. Portanto, não basta copiar exemplos sem conferir o tag/crate a
ser fixado. Features de DirectML pertencem ao caminho ONNX; a feature `cuda`
mostrada nesse manifest habilita Candle. Não são automaticamente o mesmo backend.
[Manifest consultado](https://raw.githubusercontent.com/Anush008/fastembed-rs/main/Cargo.toml).

## Shortlist de modelos

Contexto abaixo significa limite do modelo, não tamanho recomendado de chunk.
Licença do runtime não substitui a licença dos pesos.

| Modelo | Dimensão e contexto | Licença dos pesos | Uso proposto |
| --- | --- | --- | --- |
| multilingual-e5-small | 384; 512 tokens | MIT | Referência CPU para decisões e chunks curtos PT/EN |
| multilingual-e5-base | 768; 512 tokens | MIT | Comparar se small perder recuperação relevante |
| EmbeddingGemma 300M | 768, ou 512/256/128 via MRL; 2.048 tokens | Gemma | Candidato para biblioteca e código, testando quantização |
| BGE-M3 | 1.024 dense; 8.192 tokens; dense/sparse/multivetor | MIT | Challenger de recuperação multilíngue e documentos |
| Qwen3-Embedding-0.6B | até 1.024; 32K tokens; MRL | Apache-2.0 | Challenger com instruções e recuperação de código |
| bge-reranker-v2-m3 | Score por par, não vetor de índice | Apache-2.0 | Reranker multilíngue opcional para poucas candidatas |
| Qwen3-Reranker-0.6B | Score por par; 32K tokens | Apache-2.0 | Challenger de reranking; medir custo por par |
| mmarco-mMiniLMv2-L12-H384-v1 | CrossEncoder multilíngue; aproximadamente 0,1B parâmetros no card | Apache-2.0 | Primeiro candidato compacto de rerank; medir PT/EN e comparar sem rerank |

Dados e licenças conferidos nos cards:
[E5-small](https://huggingface.co/intfloat/multilingual-e5-small/raw/main/README.md),
[E5-base](https://huggingface.co/intfloat/multilingual-e5-base),
[EmbeddingGemma](https://huggingface.co/google/embeddinggemma-300m),
[BGE-M3](https://huggingface.co/BAAI/bge-m3),
[Qwen3 embedding](https://huggingface.co/Qwen/Qwen3-Embedding-0.6B),
[BGE reranker](https://huggingface.co/BAAI/bge-reranker-v2-m3),
[Qwen3 reranker](https://huggingface.co/Qwen/Qwen3-Reranker-0.6B).

Para 8 GB, avaliar primeiro o [CrossEncoder mMARCO MiniLM](https://huggingface.co/cross-encoder/mmarco-mMiniLMv2-L12-H384-v1)
em 5/10 pares curtos, antes do BGE/Qwen maior. Ele foi treinado em MS MARCO
traduzido; isso não garante qualidade em arquitetura/código. Não integra o
catálogo de rerankers Rust consultado: usar arquivos locais/adapter próprio ou
worker de referência, com conformidade contra Sentence Transformers.

Jina também oferece alternativas atuais: v5-text-nano tem 239M parâmetros,
768 dimensões com MRL e exportações por tarefa ONNX/GGUF. Porém, seus pesos
são CC-BY-NC-4.0, com uso comercial remetido ao fornecedor. O card descreve
32K no texto e 8.192 na tabela: confirmar limite efetivo da variante antes de
adotá-la. O reranker v2 multilíngue (278M no card) tem a mesma restrição
não comercial. São opções de avaliação/licenciamento, não defaults gratuitos
de redistribuição comercial.
[Jina v5 nano](https://huggingface.co/jinaai/jina-embeddings-v5-text-nano),
[Jina reranker v2](https://huggingface.co/jinaai/jina-reranker-v2-base-multilingual).

### Artefatos e footprint: o formato importa

Os tamanhos são arquivos publicados, em unidades decimais, não RAM medida nem
tamanho completo do instalador. Não se deve baixar o repositório inteiro contendo
todas as variantes; selecionar arquivos necessários com revisão fixa.

| Artefato lido | Tamanho publicado | Limitação |
| --- | --- | --- |
| E5-small ONNX original | 470 MB; tokenizer JSON 17,1 MB | Catálogo Rust aponta esse original por padrão |
| E5-small ONNX INT8 | 118 MB | Variante nomeada `avx512_vnni`; testar compatibilidade/desempenho no CPU alvo |
| E5-base ONNX original / INT8 | 1,11 GB / 279 MB | Mesma ressalva sobre quantização alvo e qualidade |
| EmbeddingGemma ONNX Q4 | 197 MB de dados + aproximadamente 0,52 MB de grafo | Conversão `onnx-community`; testar provider e fidelidade |
| EmbeddingGemma no Ollama | 622 MB | Outra distribuição; não confundir com Q4 ONNX |
| BGE-M3 ONNX original | 2,27 GB de dados + grafo/tokenizer | Contexto longo pode exigir mais memória de trabalho |
| Qwen3-Embedding-0.6B GGUF oficial Q8 / F16 | 639 MB / 1,2 GB | `--pooling last` no exemplo oficial; Q4 oficial não confirmado aqui |

Fontes de arquivos:
[E5-small ONNX](https://huggingface.co/intfloat/multilingual-e5-small/tree/main/onnx),
[E5-base ONNX](https://huggingface.co/intfloat/multilingual-e5-base/tree/main/onnx),
[EmbeddingGemma ONNX](https://huggingface.co/onnx-community/embeddinggemma-300m-ONNX/tree/main/onnx),
[EmbeddingGemma Ollama](https://ollama.com/library/embeddinggemma),
[BGE-M3 ONNX](https://huggingface.co/BAAI/bge-m3/tree/main/onnx),
[Qwen GGUF oficial](https://huggingface.co/Qwen/Qwen3-Embedding-0.6B-GGUF).

O catálogo Rust consultado aponta E5 small/base originais e EmbeddingGemma
Q4/INT8. O BGE-M3 joint INT8 vem de `gpahal`, e o BGE reranker v2-m3 ONNX de
`rozgo`: conversões de terceiros usadas pela biblioteca, que precisam de hash,
licença/proveniência e teste contra o original antes de distribuição.
Qwen3 embedding está documentado via Candle; não foi confirmado Qwen3 reranker
pronto no catálogo Rust. GGUF oficial de Qwen3 reranker e GGUF de EmbeddingGemma
não foram confirmados nesta investigação, portanto não são requisitos propostos.
[Catálogo de embeddings](https://raw.githubusercontent.com/Anush008/fastembed-rs/main/src/models/text_embedding.rs),
[BGE joint](https://raw.githubusercontent.com/Anush008/fastembed-rs/main/src/models/bgem3.rs),
[rerankers](https://raw.githubusercontent.com/Anush008/fastembed-rs/main/src/models/reranking.rs).

### Não trocar apenas o arquivo dos pesos

- E5: `query: ` e `passage: ` inclusive em português; mean pooling com máscara
  de atenção e normalização L2. Textos maiores são truncados em 512 tokens.
- EmbeddingGemma: prompts distintos de consulta/documento; usa a configuração
  própria do modelo. Não suporta ativações float16 no modelo original; não
  selecionar fp16 só pelo nome/tamanho. MRL requer renormalização após corte.
- BGE-M3: não exige instrução na consulta. Dense não representa sozinho os
  recursos sparse e multivetor; cada representação precisa de armazenamento/score.
- Qwen3 embedding: instrução de tarefa na consulta, pooling do último token e
  L2; documentos não precisam da mesma instrução.
- Qwen3 reranker: template de pares e logits `yes`/`no` na implementação de
  referência. Um chat genérico não é substituto equivalente.

Essas regras vêm dos cards citados. O app deve oferecer `embed_query` e
`embed_document`, com política versionada de preparação de entrada. O usuário
não precisa escrever esses prefixos. Um teste de conformidade deve comparar
vetores/ranking com a implementação de referência em textos fixos.

## Windows e aceleração

CPU é o baseline que evita pressupor NVIDIA. DirectML requer DirectX 12 e
compatibilidade entre operadores/modelo/provider. A documentação registra
restrições de execução paralela e memory pattern. GPU disponível não implica
ganho para consulta curta ou modelo quantizado: medir sessão, transferências e
fallbacks. [ONNX Runtime DirectML](https://onnxruntime.ai/docs/execution-providers/DirectML-ExecutionProvider.html).

Ollama distribui ZIP Windows para integração, além do instalador. O guia de
hardware distingue NVIDIA/AMD e caminhos experimentais; precisamos validar a
placa alvo. Não presumir GPU universal.
[Windows](https://docs.ollama.com/windows), [hardware](https://docs.ollama.com/gpu).

Sentence Transformers pode usar PyTorch no Windows; o instalador oficial oferece
CPU/CUDA e delimita suporte por plataforma. Um pacote Python CUDA não deve ser
o requisito básico para todos os usuários. [PyTorch](https://pytorch.org/get-started/locally/).

## Worker local e possibilidade de outro repositório

Proposta de seam: portas de `application` para embeddings/reranking; adapters
Rust ou IPC sem regras do grafo dentro do processo de inferência. O worker recebe
texto e configuração e devolve vetores/scores. O xemnas conserva decisões,
permissões de corpus, evidências e transações.

| Distribuição | Benefício | Custo próprio |
| --- | --- | --- |
| Rust in-process | Sem ambiente Python; comunicação direta | Runtime/DLLs e modelos; falhas nativas compartilham processo; carga fora da UI |
| Executável Rust separado | Isolamento e encerramento/reinício controlados | Protocolo/versão e cópia de dados |
| Python FastEmbed worker | Ecossistema Python sem PyTorch obrigatório | Python, wheels ONNX, empacotamento e atualização |
| Python Sentence Transformers worker | Laboratório/modelos e ingestão mais flexíveis | PyTorch ou backend alternativo, dependências e possível footprint maior |
| Ollama/llama.cpp local | Gestão ou GGUF, possível compartilhamento com LLM | Processo extra e competição de RAM/VRAM; contrato/versionamento |

Um repo Python separado pode produzir releases versionadas de um worker Windows
distribuído junto ao app. Usar stdin/stdout com protocolo enquadrado ou named
pipe evita exigir HTTP. Se usar loopback, o serviço fica local e sob ciclo de vida
do app; não existe mensalidade de servidor de inferência por isso. Há custos de
CI, assinatura, downloads, armazenamento de releases e manutenção. São custos
de distribuição, separados do consumo local de energia/CPU/RAM.

PyInstaller empacota interpretador e dependências. `onefile` extrai arquivos
temporários ao iniciar; `onedir` evita essa extração. Isso não reduz tamanho dos
pesos nem transforma CUDA em dependência portável automaticamente.
[PyInstaller](https://www.pyinstaller.org/en/stable/operating-mode.html).

Fixar versão do worker, protocolo, runtime, modelo/revisão, tokenizer, prompts,
dimensão, quantização e chunker. Hash do conteúdo permite reutilização; mudar
modelo exige índice separado e reindexação. Verificar downloads por arquivo e
revisão, com modo offline após provisionamento.
[Download de modelos e revisões](https://huggingface.co/docs/hub/models-downloading).

## Custo e latência: o que medir

Não há latência confiável para o xemnas sem hardware e corpus definidos.
Benchmarks publicados pelos fornecedores não foram convertidos em promessa de
milissegundos para este app. Proposta:

1. Máquina Windows real de 8 GB sem GPU dedicada; registrar CPU, RAM, GPU/VRAM,
   drivers, memória disponível, paginação e energia. Repetir com editor e tarefas
   usuais abertas; não inferir experiência a partir de workstation de pesquisa.
2. E5-small e EmbeddingGemma Q4; só depois base/BGE/Qwen se a qualidade justificar.
3. Medir cold start (processo + sessão + carga) separado de warm p50/p95.
4. Decisões de 128/256/512 tokens; chunks de documentos de 256/512 e, para
   modelos compatíveis, 1.024; batches 1/4/8 e quantidade de threads controlada.
   Batch 32 só se houver margem de RAM demonstrada; cancelar/reduzir lote sob pressão.
5. Ingestão: chunks/s, tempo total, RAM/VRAM de pico, disco e bateria; consulta:
   tokenização + embedding + busca + rerank + seleção + eventual LLM separados.
6. Rerank compacto mMARCO com 5/10 pares e limite de tokens; comparar sem rerank.
   BGE/Qwen e 20 pares somente se ganho justificar RAM/latência; medir carga
   sequencial e simultânea de embedding/reranker, incluindo paginação.
7. Qualidade PT/EN em decisões/código e livros: Recall@k, nDCG e casos difíceis
   anotados; falsos positivos de relações avaliados separadamente.
8. UI responsiva, cancelamento, fila retomável, offline e instalação limpa sem
   Python/Ollama já instalados. Quantização precisa passar a mesma avaliação.

Manter o worker aquecido quando houver uso evita recarregar por chunk; descarregar
após ociosidade poupa RAM. Indexação em lotes e prioridade baixa pode competir
menos com compilação. São escolhas a medir, não vantagens universais.

Embedding de documento é pago em computação na ingestão e reaproveitado; consulta
gera só o vetor da consulta. Reranker executa cada par escolhido e não deve avaliar
o corpus inteiro. Seu score mede relevância, não prova dependência/conflito entre
decisões. A análise de relações continua uma etapa distinta, com evidência e
possível custo de LLM local/remoto.

## Limitações e decisões ainda abertas

| Limitação | Próximo passo |
| --- | --- |
| Nenhum modelo/runtime executado | Benchmark autorizado em hardware alvo |
| Perfil de 8 GB definido; CPU e corpus ainda desconhecidos | Identificar máquina e datasets PT/EN reais |
| Catálogo `main` muda | Fixar release/tag e matriz de features antes do spike |
| Conversões de terceiros | Conferir hashes, proveniência e equivalência de ranking |
| Reranker pode dominar consulta | Comparar ganho de qualidade e p95 com teto de pares |
| Python/serviço Windows sem bundle testado | Spike de instalação limpa e startup |
| PDFs escaneados e layouts complexos | Pesquisa de ingestão/OCR separada; embedding não extrai PDF |
| Troca de modelo invalida comparação de vetores | Namespace do índice e migração/reindexação em background |
| Latência total inclui análise de relações/LLM | Medir etapas isoladas e deixar adoção durável com enriquecimento assíncrono |

Recomendação final desta pesquisa: experimentar runtime Rust ONNX e um worker
Python de referência sobre o mesmo corpus, sem decidir outro repositório ou
hospedagem antes de medir. Manter a possibilidade de provider local intercambiável
é duradouro; escolher um modelo por leaderboard, sem avaliação do produto, não é.
