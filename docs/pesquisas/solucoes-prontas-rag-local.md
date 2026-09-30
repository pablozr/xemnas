# Soluções prontas para biblioteca e RAG local

**Data:** 2026-09-30
**Pergunta:** o que aproveitar do mercado para uma biblioteca universal de PDFs/livros e um assistente no xemnas, mantendo Windows, 8 GB de RAM e ausência de GPU como piso?
**Status:** Aberta. Comparação documental; nenhuma solução instalada ou incorporada.

Complementa a [síntese de arquitetura e custos](memoria-semantica-local-first.md).

## Conclusão de integração

Já existem produtos completos para conversar com documentos e bibliotecas para
compor nossa própria implementação. Produto pronto é útil como referência e
protótipo; importar todo seu backend também importa contas, armazenamento,
interface, atualização e políticas que podem duplicar o xemnas. A recomendação
desta pesquisa é manter o domínio e o grafo no Rust e selecionar componentes
substituíveis de ingestão, embedding e ranking. Esta é uma avaliação arquitetural,
não uma capacidade prometida pelos fornecedores.

Local-first não exige hospedar servidor na internet. Um processo auxiliar Python
pode morar na máquina, receber tarefas do app por IPC/HTTP em loopback e terminar
quando o app fechar. Repositório separado é escolha de organização; não implica
VM, assinatura ou conta cloud. Embeddings locais e inferência generativa local
são capacidades diferentes: o piso de 8 GB não autoriza concluir que um LLM
grande, OCR e indexação concorrente caberão junto com Windows, IDE e app.

## Produtos completos

### AnythingLLM

- Desktop instalável em Windows/macOS/Linux, com defaults locais e variante
  Docker separada. Windows oferece x64 e ARM64; instalação é por usuário e pode
  baixar runtimes. [Desktop](https://docs.anythingllm.com/installation-desktop/overview),
  [Windows](https://docs.anythingllm.com/installation-desktop/windows).
- Integra embedders locais e LanceDB; a preferência de busca com reranking está
  documentada para LanceDB. O fornecedor relata 100–500 ms extras em seus testes,
  sem configuração suficiente para transformar isso em SLO no nosso piso.
  [RAG](https://docs.anythingllm.com/chatting-with-documents/introduction).
- OCR de scans/imagens usa `tesseract.js` no collector, com cache local e idiomas
  configuráveis. Isso confirma implementação, não qualidade em todos os PDFs nem
  paridade de cada distribuição. [Código OCR](https://raw.githubusercontent.com/Mintplex-Labs/anything-llm/master/collector/utils/OCRLoader/index.js).
- Documenta developer API para gestão, embedding e chat, com `/api/docs` por
  instância. A página genérica não estabelece sozinha porta, descoberta ou
  compatibilidade do Desktop: validar isso na versão escolhida antes de prometer
  um adaptador Rust. [API](https://docs.anythingllm.com/features/api).
- Código do repositório é MIT; serviços Cloud e extras Desktop Pro são ofertas
  distintas, não necessários para avaliar o núcleo local. Não pressupor que a
  licença do código cubra modelos, marcas ou todos os extras do instalador.
  [Licença](https://github.com/Mintplex-Labs/anything-llm/blob/master/LICENSE).

**Encaixe:** boa referência para experiência de biblioteca desktop. Não escolher
o produto inteiro como runtime invisível antes de testar contrato e consumo.

### GPT4All LocalDocs

- Aplicativo Windows/macOS/Linux. LocalDocs indexa pastas em trechos com embeddings
  Nomic no dispositivo e mostra fontes. GPU não é obrigatória; CPU precisa de
  AVX/AVX2 e RAM para o modelo escolhido. [Quickstart](https://docs.gpt4all.io/gpt4all_desktop/quickstart.html),
  [LocalDocs](https://docs.gpt4all.io/gpt4all_desktop/localdocs.html),
  [FAQ](https://docs.gpt4all.io/gpt4all_help/faq.html).
- Há API HTTP local OpenAI-compatible. LocalDocs pode alimentar essa API e retornar
  referências, incluindo página do PDF; **ativar coleções atualmente exige a UI**,
  não há operação equivalente documentada na API. [API server](https://docs.gpt4all.io/gpt4all_api_server/home.html).
- A tabela mostra Phi-3 Mini com 4 GB de RAM e Llama 3 8B com 8 GB. Esses números
  são requisitos de modelos, não garantia de orçamento total ou latência. Não
  assumir OCR de scans ou reranker configurável: não foram estabelecidos pelas
  páginas examinadas. [Modelos](https://docs.gpt4all.io/gpt4all_desktop/models.html).
- Código MIT; modelos têm licenças próprias, inclusive exemplos não comerciais.
  [Licença do código](https://raw.githubusercontent.com/nomic-ai/gpt4all/main/LICENSE.txt).

**Encaixe:** melhor prova simples da experiência local sem Docker; dependência da
UI limita sua adoção como motor programático de biblioteca do xemnas.

### Open WebUI

- É aplicação web com backend, instalável por Python no Windows ou Docker/WSL,
  não uma biblioteca Rust. A instalação Python pode evitar Docker.
  [Quickstart](https://docs.openwebui.com/getting-started/quick-start/).
- RAG combina embeddings, BM25 e CrossEncoder; oferece parsers como Tika,
  Docling e Mistral OCR. Escolher parser/provedor local é necessário para manter
  os dados locais; existir integração cloud não a torna obrigatória.
  [RAG e parsing](https://docs.openwebui.com/features/chat-conversations/rag/).
- API documenta autenticação e integração, mas é descrita como experimental;
  um adaptador precisa fixar versão e verificar contrato.
  [Endpoints](https://docs.openwebui.com/reference/api-endpoints/).
- Desde v0.6.6 a licença inclui condições de marca; o próprio projeto informa
  que a licença atual não é OSI-approved. Há exceções descritas por número de
  usuários e licenciamento empresarial. Não registrar como BSD puro nem assumir
  redistribuição com marca xemnas irrestrita. [Licença vigente](https://docs.openwebui.com/license/).

**Encaixe:** laboratório para comparar retrieval e parsers. Copiar o produto inteiro
para distribuir em cada desktop seria uma decisão distinta, com custo de pacote,
compatibilidade e licença; não há benchmark aqui que garanta o piso de 8 GB.

### Khoj

- Assistente pessoal com busca semântica e PDFs. Self-host via Docker ou Python;
  as instruções Windows incluem CPU. Embeddings podem ser gerados localmente e
  chat pode operar offline; a página de privacidade também declara telemetria.
  [Produto](https://github.com/khoj-ai/khoj),
  [Self-host](https://docs.khoj.dev/get-started/setup/),
  [Privacidade](https://docs.khoj.dev/privacy/).
- Documenta cross-encoder e indexação incremental. O benchmark publicado de
  reranking inferior a 2 s para 15 resultados foi medido em **2022 num Mac M1**;
  não é evidência de performance no Windows mínimo. [Performance](https://docs.khoj.dev/miscellaneous/performance/).
- AGPL-3.0. Uma integração ou redistribuição precisa verificar obrigações concretas;
  HTTP e repositório separado não são prova automática de independência jurídica.
  OCR de scans e contrato de API de ingestão não foram estabelecidos nesta pesquisa.
  [Licença](https://raw.githubusercontent.com/khoj-ai/khoj/master/LICENSE).

**Encaixe:** referência de assistente/biblioteca; não primeira escolha para embutir.

### RAGFlow

- Plataforma de ingestão/retrieval; a documentação 0.27.2 lista mínimo de 4 cores,
  16 GB RAM e 50 GB disco. Isso **excede nosso piso de 8 GB**. Compose inclui
  Elasticsearch/Infinity, MySQL, MinIO e Redis.
  [Requisitos](https://ragflow.io/docs/v0.27.2/build_docker_image),
  [Configuração](https://ragflow.io/docs/v0.27.2/configurations).
- Integra modelos locais de chat, embedding e rerank; chat tem APIs HTTP/Python.
  [Modelos locais](https://ragflow.io/docs/v0.27.2/deploy_local_llm),
  [Integração](https://ragflow.io/docs/v0.27.2/using_chat_conversations).
- O anúncio 0.24 registra suporte PaddleOCR-VL. É evidência de integração de OCR,
  não benchmark de precisão/consumo em livros escaneados no nosso hardware.
  [Anúncio](https://ragflow.io/blog/ragflow-0.24.0-memory-api-knowledge-base-governance-and-agent-chat-history).
- Núcleo Apache-2.0; verificar dependências e modelos separadamente. Há documentos
  1.0.0-rc1 emergentes, mas não misturar suas promessas com a versão examinada.
  [Licença](https://raw.githubusercontent.com/infiniflow/ragflow/main/LICENSE).

**Encaixe:** comparar ingestão de livros complexos em laboratório, não runtime padrão.

## Bibliotecas de orquestração opcionais

| Opção | O que oferece | Uso possível |
| --- | --- | --- |
| LangChain | Loaders, splitters, embeddings, vector stores e retrieval modular | Adaptadores num worker Python; não delegar aprovação/validade do grafo |
| LlamaIndex | Pipeline de biblioteca e exemplo explicitamente local com Ollama e embeddings Hugging Face | Protótipo da base de PDFs; framework local é distinto de LlamaCloud/LlamaParse |
| Haystack | Componentes/pipelines, embeddings Sentence Transformers, rankers | Pipeline explícito e avaliável, sem precisar serviço gerenciado |

Fontes: [LangChain retrieval](https://docs.langchain.com/oss/python/deepagents/retrieval),
[LlamaIndex local](https://developers.llamaindex.ai/python/framework/getting_started/starter_example_local/),
[Haystack embedder](https://docs.haystack.deepset.ai/docs/sentencetransformersdocumentembedder),
[Haystack ranker](https://docs.haystack.deepset.ai/docs/sentencetransformerssimilarityranker).
As licenças dos núcleos são [MIT](https://raw.githubusercontent.com/langchain-ai/langchain/master/LICENSE),
[MIT](https://raw.githubusercontent.com/run-llama/llama_index/main/LICENSE) e
[Apache-2.0](https://raw.githubusercontent.com/deepset-ai/haystack/main/LICENSE), respectivamente.

## Shortlist e custo

1. **Referência desktop:** GPT4All LocalDocs e AnythingLLM.
2. **Laboratório de retrieval:** Open WebUI Python; RAGFlow somente em máquina adequada.
3. **Composição do xemnas:** componentes locais pequenos; framework Python só se
   simplificar ingestão/avaliação e passar orçamento de RAM/instalação.

Na arquitetura recomendada, hospedagem de inferência é **US$ 0 de servidor externo
obrigatório**: processamento e armazenamento ficam no dispositivo. Permanecem
custo de energia, espaço, downloads e manutenção/distribuição. Cloud de chat é
opcional e pago por uso/assinatura conforme provedor; preços dessas ofertas não
foram cotados aqui. Produto self-hosted não significa VM gratuita se for hospedado.

Validação pendente: consumo do worker frio/quente, latência p50/p95 e duração da
ingestão/OCR no Windows 8 GB sem GPU, com IDE aberta. Nenhuma documentação de
produto substitui esse ensaio; nenhum runtime foi instalado ou executado.
