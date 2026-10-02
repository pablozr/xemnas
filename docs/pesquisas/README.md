# Pesquisas

Pesquisas de referência, investigações técnicas e ideias para o futuro. Nada aqui
é escopo aprovado: escopo está em `docs/produto/MVP-SPEC.md`, nos ADRs e nos
planos de `docs/roadmap/`.

## Índice

| Documento | Assunto | Status |
| --- | --- | --- |
| [metodologia-benchmark-semantico.md](metodologia-benchmark-semantico.md) | Protocolo de qualidade, latência e memória para E5/Gemma e SQLite/LanceDB | Aberta. Experimento Rust CPU em execução; Windows bloqueou o build nativo |
| [memoria-semantica-local-first.md](memoria-semantica-local-first.md) | Síntese da recuperação local, direção acordada, custos, latência e experimento mínimo | Aberta. Windows 8 GB/CPU; stack final depende de benchmark |
| [embeddings-e-reranking-local.md](embeddings-e-reranking-local.md) | Runtimes Rust/Python, modelos locais, licenças e empacotamento | Aberta. Shortlist E5-small/EmbeddingGemma; sem execução de modelos |
| [biblioteca-e-busca-local.md](biblioteca-e-busca-local.md) | Índices locais, PDFs/OCR, chunking, citações e memória | Aberta. SQLite/LanceDB em comparação; sem instalação |
| [solucoes-prontas-rag-local.md](solucoes-prontas-rag-local.md) | Produtos RAG locais e frameworks disponíveis no mercado | Aberta. Comparação documental; integração não validada |
| [automacao-do-grafo-fontes.md](automacao-do-grafo-fontes.md) | Auditoria do grafo, automação na adoção e comparação com GraphRAG/Graphiti | Em implementação. Base corrigida e vínculos na adoção entregues; descoberta automática, relações entre decisões e embeddings abertos |
| [ideias-de-produto.md](ideias-de-produto.md) | Revisão, contexto do projeto e próximas ideias do produto | Aberta. A aba Contexto já foi entregue (`40ead8a`); o resto segue em discussão até o dogfood |
| [provedores-e-modelos-de-ia.md](provedores-e-modelos-de-ia.md) | Provedor e modelo para a extração (conta ChatGPT, OpenCode, modelo local, chave de API) | Implementada (ADR-0004): backend, filtro no plugin e tela de IA. Pendente só a assinatura do ID token |
| [gpui-zeron-e-fork.md](gpui-zeron-e-fork.md) | O que trazer do Zeron (movimento, popovers, material) e se vale um fork do GPUI | Em implementação. Etapa 1 sem fork entregue; fork mínimo e divisão das telas abertos |
| [listas-e-grafos-em-escala.md](listas-e-grafos-em-escala.md) | Listas virtuais, revelação sob demanda e grafo com milhares de nós | Em implementação. Índices virtuais, rodapé de revelação e grafo agrupado entregues; backend paginado e demais telas abertas |
| [acabamento-visual-revisao.md](acabamento-visual-revisao.md) | Revisão de acabamento e referências (Raycast, design-engineering, Windows) | Aberta. Revisão feita em capturas; ajustes de lista entregues, fila de refinamentos aberta |
| [glass-na-janela-inteira.md](glass-na-janela-inteira.md) | Material do Windows (Mica, Acrylic) na janela e conteúdo em card | Base de decisão. O glass é opcional (`XEMNAS_BACKDROP`); a regra vigente está em `docs/design/VISUAL-IDENTITY.md` |

## Como usar esta pasta

- **Uma pesquisa por assunto**, com nome descritivo em minúsculas e hífens
  (`provedores-e-modelos-de-ia.md`, não `research-3.md`).
- **Cabeçalho obrigatório:** título, data da pesquisa, a pergunta que ela responde
  e uma linha `**Status:**` com um destes valores:
  - **Aberta**: ideia ou investigação para o futuro; ainda não virou trabalho.
  - **Em implementação**: virou tela ou recurso em andamento; o plano pode viver
    aqui enquanto isso.
  - **Base de decisão**: já implementada e citada por uma regra vigente como
    fundamento.
- **Fontes com link.** O que veio de resumo de busca, ou não pôde ser conferido,
  fica marcado como "(confirmar)".
- **Ao implementar:** leve o que for duradouro para o documento oficial
  (`docs/design/VISUAL-IDENTITY.md`, o design system, `docs/arquitetura/` ou um ADR)
  e apague a pesquisa e as imagens ou protótipos que só ela usava, no mesmo commit.
  Só fica quem uma regra vigente cita como fundamento, com o status
  **Base de decisão**.
- **Atualize este índice** sempre que entrar, mudar de status ou sair uma pesquisa.
