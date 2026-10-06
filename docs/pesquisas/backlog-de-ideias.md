# Backlog de ideias

**Data:** 2026-10-06.
**Pergunta:** o que, das pesquisas e propostas anteriores, ainda está aberto e é
concreto o bastante para virar trabalho?
**Status:** Aberta. Nada aqui é escopo aprovado; escopo está nos ADRs e em
`docs/roadmap/`. Cada linha diz de onde veio (documento já apagado; ver o git). Ideias
de mercado sem tarefa concreta ficam em `mercado-e-ideias.md`.

Já entregue e por isso fora da lista: modo automático (ADR-0012), revisão por
exceção, faixa "desde a última visita", evidência antes do motivo, desfazer em
Rejeitar e Adiar, fila única de aprovação, Contexto em quatro entradas, calibração da
confiança, Visão em contêineres e fluxo dinâmico, `agent_queries` e o relatório
automático do dogfood (`tools/dogfood-report.py`).

## Medir se o contexto ajuda

- Painel Eficácia em Contexto: funil entregue, absorvido (citado, aberto) e arquivo
  do escopo tocado, com tamanho de amostra e custo em tokens, e ação "tirar do
  contexto" para o ocioso (medir-eficacia-do-contexto).
- Holdout determinístico por sessão, comparação com intervalo e "poucos dados"
  abaixo de um mínimo (medir-eficacia-do-contexto).
- Canários e repetição offline (`xemnas bench`) com agentes em modo não
  interativo, custo declarado antes (medir-eficacia-do-contexto).
- Ranquear e podar o pack pela utilidade medida, com exploração controlada
  (medir-eficacia-do-contexto).
- Sinais de uso no ranking: decisão aberta por `get_decision` sobe (busca-alem-do-lexico).
- Recibo de contexto: por que cada item entrou e o que ficou fora, para diagnosticar
  uma resposta ruim (melhorias-apos-avaliacao-real, oportunidades de produto).
- Painel de saúde da memória: decisão nunca citada, componente com atividade e sem
  decisão, regra órfã (inspiracao-gpui-e-mercado).

## Regras e verificação

- Verificadores automáticos de regra sobre o diff da sessão: `forbid_added`,
  `forbid_dependency`, `forbid_edit`, `require_with` (medir-eficacia-do-contexto;
  ver também `ciclo-correcao-verificacao-enforcement.md`).
- Comparar regras do mesmo escopo juntas na revisão e apontar propostas equivalentes
  (melhorias-apos-avaliacao-real, itens 7 e 8).
- Casos contrastivos com identidade de execução; invariância da consulta por idioma
  e distração (ideias-para-ciclo-avaliacao-real, incrementos 1 e 5).
- Admitir ausência de resposta sem esconder contexto parcial útil (ideias-para-ciclo,
  incremento 2).
- Escolha e ressalva como unidade de entrega: o orçamento nunca corta só a ressalva
  (ideias-para-ciclo, incremento 3).
- Próxima ação conforme a causa do estado vazio; destino de cada captura
  (recebida, em processamento, descartada, candidato, erro) (melhorias, itens 4 e 6).

## Contexto e busca

- PageRank personalizado no grafo, só se menções e termos deixarem ruído
  (busca-alem-do-lexico, técnica 3).
- Glossário de sinônimos aprendido do projeto, quando houver volume real
  (busca-alem-do-lexico, técnica 4).
- Contexto por branch, só com caso real de decisão de experimento confundida com
  vigente (melhorias, "O que adiar").
- Exportar resumo para `AGENTS.md` por ação explícita e com prévia (ideias-de-produto).
- Importar ADRs e `AGENTS.md` existentes no primeiro uso (ideias-de-produto,
  analise-do-produto-hoje).
- Filtrar a documentação pelo que a pessoa dispensa e mostrar em Contexto o que entra
  e por quê (ideias-de-produto).
- Mesmo pack semântico para dois agentes, com os campos que cada adaptador perde
  (produtos-memoria-agentes-fontes).

## Grafo e modelo

- `as_of` reconstruir padrões, nomes e texto antigos, não só os dados atuais
  (automacao-do-grafo-fontes, P2).
- Constraint única para identidade de entidade por nome no banco
  (automacao-do-grafo-fontes, P2).
- Entidades de tecnologia descobertas automaticamente (só vêm das decisões hoje) e
  medição de recuperação em casos reais antes de qualquer biblioteca nova
  (automacao-do-grafo-fontes, passos 3 e 6).
- Interações entre componentes propostas pela IA, com rótulo e protocolo nas setas
  da Visão, entrando em Sugestões; nível Componentes e visões salvas do Mapa
  (analise-do-produto-hoje).
- Radar de premissas: dez condições de "reconsiderar quando" ligadas a fontes,
  triagem manual antes de detector (melhorias, item 16).
- Retomada orientada à tarefa, reusando Contexto e Visão (melhorias, item 15);
  pacote transferível e verificável como experimento (melhorias, item 17).
- Revisão: agrupar por sessão e mostrar conflito no candidato; fila dividida por
  origem e confiança; adiar com data (ideias-de-produto, inspiracao-gpui-e-mercado).

## Desempenho e escala

- Backend com janela de tempo e cursor para o Mapa (hoje entrega o mapa inteiro)
  (listas-e-grafos-em-escala).
- Página de componente com "Ver todas" por seção e filtro; linha do tempo global
  virtual com cabeçalho por dia; Revisão e Contexto como listas virtuais
  (listas-e-grafos-em-escala).
- Grafo: expandir grupo no lugar, layout incremental e multinível para 10 mil nós
  (~2,9 s hoje, em fundo), nível de detalhe por zoom nos rótulos, índice espacial
  para o hit-test só se a varredura linear pesar (escalabilidade-renderizacao-fontes).
- Investigar a semeadura de 5000 decisões (mais de 100 s; suspeita de custo
  quadrático em `create_entity`), que uma importação grande real também sofreria
  (listas-e-grafos-em-escala).
- Medir a pintura do grafo com zoom aproximado, onde o corte pela viewport deveria
  valer (escalabilidade-renderizacao-fontes).

## Interface

- Paleta: fuzzy com recentes e rodapé de dicas (inspiracao-gpui-e-mercado, V2).
- Hover com fade de 150 ms generalizando `hover_tint` (V3).
- Anel de orçamento de contexto com popover (V5); seções recolhíveis na lista de
  projetos (V6); acento escolhido pela pessoa com checagem de contraste (V7).
- Painéis redimensionáveis com largura persistente (V10); modo tabela para todas as
  decisões (V11); testes de interface headless com asserções de foco (V14).
- Escala sutil ao pressionar onde o GPUI permitir; raios concêntricos em cartões com
  chips; capturas comparáveis com Zeron, Linear e Raycast (acabamento-visual-revisao).
- Dividir os arquivos grandes por feature: `screens/map/`, `screens/context/` e o
  shell separando paleta, barra de projeto e roteamento (gpui-zeron-e-fork).
- Fade de borda por pixel e blur real exigem fork mínimo do Zed oficial; só se o
  design pedir vidro de verdade (gpui-zeron-e-fork).
- Sons opcionais desligados por padrão e laço da marca como indicador de carga: medir
  com `XEMNAS_PERF` antes de adotar (inspiracao-gpui-e-mercado, V8 e V12).

## Produto, depois do dogfood

- Guardião de mudança (`xemnas check` sobre diff ou arquivo) e "Pergunte ao projeto"
  só com decisões citadas (inspiracao-gpui-e-mercado).
- Máquina do tempo do projeto no Mapa e no Contexto, apoiada em `as_of`
  (inspiracao-gpui-e-mercado).
- Perspectivas salvas do grafo; resumo semanal exportável; publicar como site
  estático; placar por agente e por custo (inspiracao-gpui-e-mercado).
- Padrões entre projetos e assistente de decisão com agentes autônomos: ver
  "Conhecimento geral do dev" em `mercado-e-ideias.md`; depende de medir antes se o
  contexto de outros projetos ajuda (ideias-de-produto, medir-eficacia-do-contexto).

## Provedores de IA

- Verificar a assinatura do ID token da conta ChatGPT (único pendente do ADR-0004)
  (provedores-e-modelos-de-ia).
