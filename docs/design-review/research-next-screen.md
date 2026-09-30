# Próxima tela: Decisões

**Escolha posterior do usuário:** variante C, documento + índice temporal, com acabamento próximo do Linear. A recomendação inicial por A abaixo é histórica. Veja a [direção escolhida e refinada](decisions-c-design.md).

## Protótipo visual produzido após a pesquisa

- [Abrir o estudo interativo](http://127.0.0.1:8769/prototype-decisions.html?variant=A), com três estruturas no mesmo shell; setas na barra inferior alternam A/B/C e preservam a opção na URL. Todos os dados são fictícios e só existem em memória.
- **A — lista + documento (recomendado):** relação com a Revisão atual, leitura e busca contínuas. [Imagem](decisions-layout-a.png).
- **B — catálogo + detalhe por abertura:** melhor para comparar registros, mas exige abrir o documento para compreender a escolha. [Imagem](decisions-layout-b.png).
- **C — documento + índice temporal:** favorece leitura prolongada, com maior mudança na orientação da navegação. [Imagem](decisions-layout-c.png).
- [Histórico](decisions-history.png), [exportação](decisions-export.png) e [janela compacta](decisions-compact.png) também têm exemplos visuais.

Arquivo autônomo: [prototype-decisions.html](prototype-decisions.html). Pode ser aberto diretamente no navegador ou servido com `python -m http.server 8769 --bind 127.0.0.1 --directory docs/design-review` a partir da raiz do projeto. A barra de variantes é uma ferramenta do estudo e não integra o produto.

Conferência no navegador: três variantes; seleção de snapshot anterior somente leitura; preview de exportação explicitando versão atual; busca por justificativa e estado sem resultados; histórico no detalhe do catálogo; URL do seletor; largura compacta de 1180 px sem overflow horizontal; nenhum erro JavaScript. O roteiro está em [prototype-decisions-qa.js](prototype-decisions-qa.js). Isso valida o estudo visual, não a implementação GPUI.

O preview de exportação no estudo é abreviado; a implementação deve apresentar integralmente os bytes retornados por `Export::preview`. O seletor de destino do estudo não grava arquivos. Metadados, versões e corpos de evidência no mock são exemplos e precisam ser substituídos pelos contratos reais na implementação.

Pesquisa realizada em 29/09/2026. Recomendação de produto e layout, sem implementação GPUI nesta etapa. Tudo que aparece como **proposta** é inferência de design para Xemnas; as referências não prescrevem nossas dimensões ou identidade visual.

## Recomendação

Implementar **Decisões**, a memória consultável das escolhas já confirmadas. É a continuação natural de Revisão: hoje Confirmar cria um registro real, mas falta uma superfície nativa para reencontrar a escolha, entender sua justificativa, consultar versões e exportar. A especificação exige lista, busca, proveniência, histórico e exportação manual; o ticket 16 registra backend concluído e tela pendente. Fontes locais: [MVP-SPEC §7.6 e §8](../MVP-SPEC.md), [ticket 16](../mvp-plan/issues/16-decisoes-pesquisaveis.md), [Decisions](../../crates/application/src/decisions.rs), [Export](../../crates/application/src/export.rs).

**Proposta preferida: lista + documento**, mantendo o shell existente. O usuário navega pelas escolhas no mesmo lugar em que revisou seus candidatos; o detalhe ganha caráter documental, sem a barra de rejeição/adiamento que pertence à Inbox. Isso preserva orientação espacial e concentra a interface no conteúdo durável.

Não antecipar Superseder, Apagar, Favoritar, tags temáticas automáticas, colaboração ou painel de métricas. Nenhuma dessas ações está disponível no contrato atual de Decisões. O domínio possui `Superseded`, mas não uma transição MVP que o defina; histórico é preservado, sem delete. Fonte: [DecisionStatus, DecisionStore e Decisions](../../crates/application/src/decisions.rs).

## Backend disponível e lacunas reais

| Necessidade da tela | Disponível hoje | Ajuste necessário |
| --- | --- | --- |
| Lista de decisões do projeto | `Decisions::list(DecisionFilter)` com `project_id`, status, cursor e limite; ordem por confirmação decrescente | Montar Entity/screen GPUI e invalidar resultados ao trocar projeto |
| Buscar todo o projeto | `Decisions::search(SearchQuery)` em pergunta, escolha e justificativa, FTS5 | Debounce, geração de requisição, destaque de snippet; distinguir busca no banco do filtro local da Inbox |
| Detalhe | `Decisions::detail` com escolha, justificativa, quatro arrays, proveniência, links e revisões completas | Leitor GPUI; não inventar conteúdo ausente |
| Revisar | `Decisions::revise` com campos opcionais; snapshot e índice atualizados numa transação | Editor multiline e listas; preservar campos intocados; cancelar sem escrita |
| Histórico | `DecisionDetail.revisions`, mais recentes primeiro, snapshots completos | Vista somente leitura com versão/data e conteúdo daquele snapshot |
| Evidência | Detalhe contém `artifact_id`, tipo e posição, não o corpo | Compor caso de uso para resolver os links por `capture_id` usando o port já existente `InboxStore::artifacts`; não consultar SQLite diretamente na view |
| Exportar | `Export::preview` Markdown/JSON sem escrita; `Export::write` com destino e overwrite explícitos | Seletor nativo de arquivo, preview, tratamento de arquivo existente e escrita em tarefa de background |
| Contagem completa da biblioteca | `DecisionStore` não oferece `count` | Não colocar badge total no mock. Exibir itens carregados; caso se queira total, adicionar port/use case + SQL COUNT e teste proporcional |
| Busca paginada | `SearchQuery` só oferece limite; resultado sem cursor | Usar limite explícito, por exemplo 50, e informar “Até 50 resultados”. Não fabricar botão Carregar mais para search |
| Ordenação por relevância | Consulta atual não tem ORDER BY de ranking | Não declarar “mais relevantes”. Se necessário futuramente, definir ranking estável e testar |
| Exportar versão antiga | `Export::preview` recarrega decisão atual | No histórico, exportação deve indicar versão atual ou ficar indisponível; exportar um snapshot antigo exige contrato próprio |

Fontes da tabela: [application/decisions.rs](../../crates/application/src/decisions.rs), [storage-sqlite/decisions.rs](../../crates/storage-sqlite/src/decisions.rs), [application/inbox.rs](../../crates/application/src/inbox.rs), [application/export.rs](../../crates/application/src/export.rs). O limite padrão é 50, teto 100: [constantes Inbox](../../crates/application/src/inbox.rs). Os handlers atuais usam geração de escopo para Inbox; a nova tela precisa conservar essa garantia: [shell](../../apps/desktop-gpui/src/app.rs), [InboxScreen](../../apps/desktop-gpui/src/screens/inbox.rs).

## Referências oficiais e o que aproveitar

1. **Raycast — lista com detalhe.** O componente documenta painel à direita do item selecionado e recomenda concentrar informação adicional no detalhe; metadados podem ter rótulos e separadores. Aproveitar a relação seleção/leitura e a economia de informação na lista, sem copiar a aparência do launcher. [Raycast API — List](https://developers.raycast.com/api-reference/user-interface/list). A experiência File Search também descreve lista ampliada e painel de metadados: [Raycast Manual — File Search](https://manual.raycast.com/file-search).
2. **Linear — busca com escopo compreensível.** A documentação distingue busca no workspace e busca na view atual. Aproveitar rótulo explícito sobre onde se busca; Xemnas Decisões buscará no projeto inteiro, enquanto Revisão mantém o filtro dos itens carregados. O comportamento FTS e limites serão os nossos, sem prometer operadores do Linear. [Linear — Search](https://linear.app/docs/search).
3. **Linear — densidade ajustada ao trabalho.** Display options associa lista/board, informação exibida e organização dos registros. Aproveitar hierarquia de linhas para uma coleção consultável. Não adicionar opções de grouping/sort sem necessidade e contrato correspondente. [Linear — Display options](https://linear.app/docs/display-options).
4. **Zed — identidade de workspace e ferramentas discretas.** Project Panel documenta seleção e destaque automático do arquivo ativo, além de navegação por teclado. Appearance distingue fonte da interface e do código, e explicita tab bar, gutter e scrollbar. Aproveitar orientação contínua, fonte de código independente e affordances de arquivo. [Zed — Project Panel](https://zed.dev/docs/project-panel), [Zed — Appearance](https://zed.dev/docs/appearance).
5. **Notion — histórico que permite inspecionar versões.** A documentação descreve selecionar uma versão para ver o conteúdo daquele momento. Aproveitar seleção explícita de snapshot com data e versão. Não copiar restauração, bookmarks ou autoria: o backend Xemnas não oferece essas ações/metadados. [Notion — Version history](https://www.notion.com/help/duplicate-delete-and-restore-content).

As fontes sustentam padrões de interação. A paleta, os tokens, os tamanhos e a composição abaixo são decisões propostas para o projeto, fundamentadas também na [identidade Quiet Glass](../../VISUAL-IDENTITY.md).

## Layout A — lista + documento (recomendado)

### Shell e lista

- Manter barra nativa, projetos persistentes de **248 px**, cabeçalho nome/caminho e abas **Revisão · Decisões · Detalhes**. Não repetir a contagem de Revisão na nova tela. Decisões começa sem badge global.
- Coluna de biblioteca **320 px** em janela padrão. Cabeçalho “Decisões”, busca “Buscar neste projeto”, legenda discreta “Pergunta, escolha e justificativa”. Busca de 36–40 px; quantidade abaixo como “12 carregadas” ou “8 resultados”, nunca total inferido.
- Linhas de aproximadamente **88–104 px**: pergunta em 15 px até duas linhas, escolha em 13 px até duas linhas, data de confirmação + `v2` em metadado de 11 px. A badge com bolinha “Confirmada” pode ficar no detalhe; não repetir em toda linha quando todos os registros têm o mesmo estado.
- Seleção conserva fundo lavanda mineral e traço lateral de 2 px sob hover; foco visível é independente da seleção. Linha inteira clicável. Ícone SVG de decisão/documento discreto, sem três badges competindo com o título.
- Rolagem própria, lista virtualizada. Carregar mais só no modo de listagem quando houver cursor. Mostrar “Mais antigas” é possível como copy do mesmo comando, sem mudar ordenação.

### Documento

- Barra do detalhe fixa, **56–64 px**, com ações **Revisar** e **Exportar**. Leitura é a atividade principal: ambas podem ser contornadas/discretas; lavanda forte é reservada a Salvar nova revisão/Exportar no fluxo correspondente.
- Corpo com padding **32 px**, largura de leitura máxima proposta **760 px**, alinhado à esquerda dentro do espaço disponível. Nada de card envolvendo todo o documento.
- Título/pergunta **24/32 px**; logo abaixo badge com bolinha verde e texto **Confirmada**, `Versão 2`, data de confirmação. A data de atualização fica na história/proveniência quando ajudar, sem duplicar várias datas no topo.
- Ordem: **Escolha** (ênfase editorial, 18/26), **Justificativa** (15/23), depois **Escopo**, **Premissas**, **Consequências**, **Reconsiderar quando**. As quatro listas são disclosures com contagem quando houver itens. Valor vazio diz “Não registrado” ao abrir a seção; não inventar premissas nem mostrar badges vazias.
- **Evidências** reaproveita painel já feito: tabs com SVG de arquivo, origem registrada, gutter e rolagem independente; conteúdo virtualizado. Não rotular “código” uma fonte de conversa/transcrição.
- **Proveniência** em disclosure abaixo das evidências: projeto/localização, candidato e captura quando registrados; IDs copiáveis com affordance explícita. Ausência de capture_id deve ser honesta. Não preencher nome do autor, hash Git ou sessão OpenCode se o contrato não fornecer.
- **Histórico · 2 versões** é ação/seção real porque o detalhe traz todas as revisões; count é daquele registro, não uma métrica global.

### Histórico

Proposta: subvista inline no mesmo painel, mantendo a seleção da biblioteca. Cabeçalho “Histórico de revisões”, voltar para a versão atual. Lista curta v2/v1 com data; snapshot selecionado mostra todas as propriedades daquele momento. “Versão atual” / “Versão anterior · somente leitura” são textos explícitos. Não desenhar diff verde/vermelho como se existisse um comparador implementado; uma comparação campo a campo pode ser estudada depois. Exportar permanece associado à versão atual, com copy inequívoca. Fonte do conteúdo: `DecisionDetail.revisions`.

### Revisão e exportação

**Revisar:** editor no painel de documento, campos multiline para pergunta/escolha/justificativa; listas com adicionar/remover item para os quatro arrays; limites do contrato (500/1000/4000 caracteres e arrays até 50 itens de 1000). Rodapé fixo “Cancelar” e **Salvar nova revisão**. Formulário não sobrescreve dados antes de salvar. Antes do submit, validar e revelar erro ao lado do campo. Duplicidade bloqueada enquanto salva. Falha preserva rascunho; conflito atual pode chegar como NotFound, então oferecer recarregar com aviso, sem chamar toda falha de “foi apagada”.

**Exportar:** modal contextual proposto de **720 × até 640 px**, compactável; seletor Markdown/JSON, preview monoespaçado rolável, quantidade de bytes real, destino escolhido pelo usuário e CTA **Exportar**. Escolher destino pode abrir seletor nativo; cancelar não escreve. Arquivo existente pede substituição explícita. Sucesso mostra caminho e ação copiar caminho se implementada. Não desenhar botão “Commitar” nem sugerir exportar automaticamente no repositório. `Export::preview` gera a versão atual; manter o documento renderizado até a escrita e regenerar se o usuário solicitar dados atualizados, para não mudar silenciosamente o preview.

## Alternativas para comparação

**B — catálogo/tabela + abertura do detalhe:** melhor para dezenas de decisões e comparação de pergunta, versão e data; pior para ler justificativa longa e reencontrar contexto sem abrir cada linha. Precisa restringir colunas a campos reais. Recomendado apenas se dogfood revelar predominância de busca/triagem sobre leitura. Pode funcionar em janela curta com lista plena e documento ao abrir.

**C — documento + índice temporal:** agrupa a coleção por data de confirmação e favorece leitura narrativa. Requer cabeçalhos calculados a partir das datas reais, sem inventar eventos ou métricas. Mais frágil durante paginação e busca: agrupamentos incompletos precisam ficar claros e ordem temporal não pode fingir relevância. Não priorizar sobre A nesta etapa.

## Responsividade e escalabilidade

Na janela de **1180 × 760** manter sidebar248, reduzir biblioteca para **280 px**, padding de documento para24, corpo flexível de aproximadamente600px. Ações permanecem fora da rolagem; listas/metadados quebram linha; tabs de arquivo rolam horizontalmente. Não criar um quarto painel lateral permanente para histórico.

Se no futuro a largura mínima ficar abaixo de aproximadamente1000px, proposta de master/detail: biblioteca ocupa o workspace e seleção abre documento com **Voltar às decisões**, preservando busca/scroll/seleção. Isso é um modo proposto, não suporte existente. Na altura curta, o título não deve consumir metade da janela; histórico, editor e export preview precisam de rolagem própria.

Para search, debounce proposto de180–250ms; descarte de resposta antiga por query+projectgeneration. Consulta vazia retorna à listagem. Sanitização que gere vazio mostra orientação suave, sem erro técnico. A busca cobre o banco do projeto, não apenas a página atual. Paginação keyset da lista, virtualização de linhas, cache do detalhe selecionado e renderização de uma evidência por vez; históricos enormes hoje vêm completos no detalhe e podem exigir port paginado futuramente. Não prometer histórico ilimitado sem custo.

## Estados e critérios de aceite

| Estado | Comportamento esperado |
| --- | --- |
| Projeto sem decisões | “Nenhuma decisão confirmada ainda.” + “Revise os candidatos deste projeto para construir sua memória.” + Ir para Revisão, sem CTA criar manual inexistente |
| Busca sem hits | “Nenhuma decisão encontrada.” + Limpar busca; preservar query e não mostrar detalhe fora dos resultados |
| Carregando lista | Skeleton curto de linhas, região acessível ocupada; não exibir zero provisório |
| Carregando detalhe | Skeleton da pergunta/justificativa; seleção permanece identificada |
| Falha de lista/detalhe | Mensagem de produto + Tentar novamente; diagnóstico técnico só log sanitizado |
| Evidência indisponível | Manter link e origem registrada quando houver; “Conteúdo desta fonte indisponível.”; não substituir por mock |
| Revisando | Rascunho, ações fixas, salvar bloqueia duplicidade; cancelar não escreve |
| Falha de revisão/export | Rascunho/preview mantido e recuperação específica; nenhuma confirmação visual antecipada |

Aceite: decisão confirmada na Inbox aparece nesta aba sem reiniciar; troca de projeto não vaza conteúdo; search encontra justificativa de registro não carregado; cada revisão salva gera nova versão e histórico anterior íntegro; fontes mantêm origem real; export exige preview/destino e não modifica working tree automaticamente; teclado alcança lista, ações, tabs e disclosures com foco visível; snapshots antigos são somente leitura; janela padrão/compacta e conteúdo muito longo têm QA nativo. Mock HTML é material de decisão de layout e não comprova esses comportamentos do produto.
