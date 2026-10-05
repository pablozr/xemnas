# Pesquisas

Pesquisas de referência, investigações técnicas e ideias para o futuro. Nada aqui
é escopo aprovado: escopo está em `docs/produto/MVP-SPEC.md`, nos ADRs e nos
planos de `docs/roadmap/`.

## Índice

| Documento | Assunto | Status |
| --- | --- | --- |
| [precisao-do-contexto.md](precisao-do-contexto.md) | Precisão do bloco entregue ao agente: evidência, diagnóstico, dez técnicas por custo, medição e plano | Em implementação. v3: 0,94 / 0,95; corpus v4 às cegas: 0,58 / 0,62; próximo passo, embeddings |
| [busca-alem-do-lexico.md](busca-alem-do-lexico.md) | Depois da ponte PT/EN e antes dos embeddings: sementes pela menção, expansão do documento na extração, PageRank no grafo e o que fica de fora | Em implementação. Termos de busca como último recurso: 0,76 / 0,95 |
| [embeddings-no-contexto.md](embeddings-no-contexto.md) | Modelo estático local (Model2Vec) como veto e resgate na seleção de contexto, medido no v4 selado | Aberta. `potion` reprovado no v4 (0,67 / 0,60 / 4); `e5-small` pendente de autorização |
| [direcao-memoria-contexto-baixo-atrito.md](direcao-memoria-contexto-baixo-atrito.md) | Direção aprovada: memória descritiva/normativa, context router, baixo custo e revisão por exceção | Aberta. Algoritmos e metas pendentes; política normativa vigente preservada |
| [ideias-para-ciclo-avaliacao-real.md](ideias-para-ciclo-avaliacao-real.md) | Sete incrementos com fontes primárias, casos contrastivos, qualificadores, admissão de contexto e experimentos | Aberta. Pesquisa documental; experimentos e implementação não executados |
| [melhorias-apos-avaliacao-real.md](melhorias-apos-avaliacao-real.md) | Dezessete melhorias priorizadas, evidências, critérios e sequência após pesquisas e retestes no ripgrep | Aberta. Proposta consolidada; implementação e hipóteses de valor pendentes |
| [20-ideias-dores-reais-devs.md](20-ideias-dores-reais-devs.md) | Vinte ideias adicionais com dores, evidência empírica, diferenças, prioridades e pilotos | Aberta. Síntese concluída; demanda e eficácia não validadas |
| [dores-dev-cognicao-fontes.md](dores-dev-cognicao-fontes.md) | Oito estudos de cognição, interrupções, compreensão e investigação | Aberta. Fontes consultadas; propostas sem experimento próprio |
| [dores-dev-conhecimento-equipe-fontes.md](dores-dev-conhecimento-equipe-fontes.md) | Oito estudos de onboarding, revisão, coordenação e dívida técnica | Aberta. Pesquisa exploratória; parte das fontes teve acesso parcial |
| [dores-dev-ia-verificacao-fontes.md](dores-dev-ia-verificacao-fontes.md) | Oito fontes de IA, produtividade, compreensão, segurança e autonomia | Aberta. Evidência contextual; sem validação no Xemnas |
| [dores-dev-contratos-reproducao-fontes.md](dores-dev-contratos-reproducao-fontes.md) | Oito estudos de linguagem, contratos, requisitos, alertas e reprodução | Aberta. Pesquisa exploratória; hipóteses não executadas |
| [oportunidades-produto-memoria-decisional.md](oportunidades-produto-memoria-decisional.md) | Dez propostas de expansão, comparação com capacidades atuais, prioridades e experimentos | Aberta. Síntese concluída; propostas não implementadas nem demanda validada |
| [produtos-memoria-agentes-fontes.md](produtos-memoria-agentes-fontes.md) | Oito famílias de memória/contexto para agentes, versões, fontes primárias e oportunidades | Aberta. Comparação documental concluída; experimentos pendentes |
| [produtos-inteligencia-engenharia-fontes.md](produtos-inteligencia-engenharia-fontes.md) | Dez produtos de engenharia, colisões competitivas e hipóteses para o Xemnas | Aberta. Comparação documental concluída; demanda não validada |
| [plano-design-vitrine.md](plano-design-vitrine.md) | Plano por superfície, capturas reais, fases, critérios de qualidade e roteiro de demonstração | Aberta. Plano concluído; implementação e mudanças de padrão pendentes |
| [design-vitrine-fontes.md](design-vitrine-fontes.md) | Apps GPUI verificados e inspirações de produtos de ponta, com aplicação e limites | Aberta. Catálogo consultivo concluído |
| [escalabilidade-renderizacao-fontes.md](escalabilidade-renderizacao-fontes.md) | Diagnóstico de grafos/blocos, GPUI, layout, virtualização, LOD e microbenchmark reproduzível | Aberta. Pesquisa concluída; implementação e benchmark integrado pendentes |
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
| [inspiracao-gpui-e-mercado.md](inspiracao-gpui-e-mercado.md) | Apps em GPUI (Zeron e outros) e produtos do mercado: mudanças visuais e ideias de produto | Em implementação. Estudo feito; V1 (pausa com janela inativa), V4 (tempo relativo) e dicas da paleta entregues |
| [plano-design-vitrine.md](plano-design-vitrine.md) | Plano por superfície, capturas reais, fases, critérios de qualidade e roteiro de demonstração | Aberta. Plano concluído; implementação e mudanças de padrão pendentes |
| [design-vitrine-fontes.md](design-vitrine-fontes.md) | Apps GPUI verificados e inspirações de produtos de ponta, com aplicação e limites | Aberta. Catálogo consultivo concluído |
| [escalabilidade-renderizacao-fontes.md](escalabilidade-renderizacao-fontes.md) | Diagnóstico de grafos/blocos, GPUI, layout, virtualização, LOD e microbenchmark reproduzível | Aberta. Pesquisa concluída; implementação e benchmark integrado pendentes |
| [medir-eficacia-do-contexto.md](medir-eficacia-do-contexto.md) | Como saber se o contexto injetado ajuda o agente: pesquisa, métricas, experimentos e fases; opinião sobre agentes autônomos e conhecimento entre projetos | Aberta. Proposta de medição; nada implementado |
| [exibicao-e-aprovacao.md](exibicao-e-aprovacao.md) | Como exibir sem sobrecarregar e como aprovar sem cansar: evidência, diagnóstico das telas, regras de exibição e aprovação automática por faixas | Aberta. Pesquisa documental e proposta; nada implementado nem testado com pessoas |
| [analise-do-produto-hoje.md](analise-do-produto-hoje.md) | O que está fraco no produto hoje, por área, e a Visão em C4 (contexto, contêineres, componentes e fluxo dinâmico) | Aberta. Análise sobre capturas da release; proposta não implementada |
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
