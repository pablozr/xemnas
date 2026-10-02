# Vitrine do Xemnas: referências de produto e viabilidade em GPUI

**Data:** 02/10/2026.
**Pergunta:** que padrões de aplicativos GPUI e produtos de referência podem melhorar
o Xemnas sem perder a identidade Quiet Glass nem sacrificar escala e clareza?
**Status:** Aberta. Pesquisa concluída; catálogo consultivo, sem mudanças de interface.

## Método e fronteira

Foram consultados sites, documentação e código dos próprios responsáveis pelos
produtos. A matriz distingue observação documental de proposta nossa: tudo em
“Aplicação no Xemnas” é interpretação, ainda não uma decisão de produto. Não se
testou nenhum aplicativo externo em sessão autenticada. Página de marketing,
catálogo e README não comprovam desempenho nem equivalem a auditoria de interação.
As fontes foram abertas em 02/10/2026; links para `main` e sites podem mudar.

A regra local continua sendo [VISUAL-IDENTITY.md](../design/VISUAL-IDENTITY.md).
A pesquisa [GPUI, Zeron e fork](gpui-zeron-e-fork.md) já registra os mecanismos
de movimento e material incorporados. Esta investigação não propõe substituir
Inter, Bricolage, lavanda, tokens, coluna de leitura ou controles compartilhados.
O [plano de execução por tela](plano-design-vitrine.md) usa este catálogo junto
das capturas atuais.

## Aplicativos GPUI: o que está confirmado

| Referência | Evidência primária e observação | Aplicação no Xemnas | Limite e decisão proposta |
| --- | --- | --- | --- |
| **Zed** — GPUI confirmado | O relato da [nova interface de configurações](https://zed.dev/blog/settings-ui) explica navegação por categorias, escopo de usuário/projeto, grupos de Tab e estado próximo do controle. O [sistema de painéis](https://zed.dev/blog/new-panel-system) oferece recolhimento e zoom de painéis pela paleta. | Ensinar o escopo de cada ajuste; dar espaço à tarefa ativa; garantir ordem de foco e retorno à origem. | Adotar previsibilidade e foco. Não transformar o Xemnas em um IDE com painéis arbitrariamente acopláveis. Novas APIs precisam existir na revisão de GPUI fixada pelo app. |
| **GPUI Component / GPUI Kit**, Longbridge — biblioteca GPUI confirmada | A [URL antiga](https://longbridge.github.io/gpui-component/) anuncia a mudança para [GPUI Kit](https://gpui-kit.com/). O site diferencia componentes com estilo de `gpui-base`, que reutiliza foco, seleção, overlays e virtualização mantendo apresentação própria. | Usar como catálogo de comportamento e comparação: entradas, listas, seletores e overlays completos. | Avaliar exemplos antes de adicionar dependência; não importar uma segunda gramática de botões ou temas. Promessas do site sobre listas enormes não são benchmark do Xemnas. |
| **Longbridge Pro** — produto declarado pelo mantenedor da biblioteca | O [catálogo oficial App Stories](https://gpui-kit.com/apps/) identifica o terminal de negociação como produto comercial GPUI Kit com cotações, gráficos e informação densa nas três plataformas. | Separar conteúdo continuamente atualizado de chrome estável; aplicar hierarquia de ferramenta a informações reais. | É comprovação publicada pelo fornecedor, sem inspeção do código proprietário. Não copiar ticker, dashboard financeiro nem inventar atividade. |
| **Zeron** — GPUI via fork confirmado | O [Cargo.toml](https://github.com/zeronsh/zeron/blob/main/Cargo.toml) fixa `gpui` no [zui](https://github.com/zeronsh/zui). A [pesquisa local anterior](gpui-zeron-e-fork.md) examinou motion, popover e glass em revisões fixadas. | Continuidade entre abertura e saída, efeitos com responsabilidade compartilhada e diagnóstico do custo de animação. | A etapa sem fork já existe no Xemnas. Não tratar blur, fade ou shader de zui como API do GPUI oficial; não reabrir adoção do fork como requisito do plano. |
| **Loungy** — GPUI confirmado, projeto arquivado | O [README do autor](https://github.com/MatthiasGrandl/loungy) declara GPUI e launcher inspirado em Raycast/Alfred. O repositório está arquivado desde 25/08/2026; o autor declara falta de desenvolvimento ativo e limitações de acessibilidade. | Referência histórica para resultado pesquisável, seleção imediata e ação contextual numa superfície pequena. | Aproveitar a ideia de interação. Não adotar dependências antigas nem reproduzir suas limitações; não apresentá-lo como base atual de acessibilidade ou suporte Windows. |

O [App Stories](https://gpui-kit.com/apps/) amplia a busca para ferramentas de
desenvolvimento, terminal/rede, sistema e produtividade. Entre os exemplos
publicados estão Zedis (Redis), tty7 (terminal), OpenLogi (periféricos), disktree
(treemap) e Cellar (banco de dados). Esses são **candidatos de segunda rodada**:
o catálogo confirma a declaração dos mantenedores, mas esta pesquisa não inspecionou
código nem capturas individuais deles. Não atribuímos desempenho, acabamento ou
padrões específicos a esses produtos. Para o Xemnas, as famílias mais pertinentes
são navegador de dados, ferramenta local e canvas de estrutura; listar dezenas de
nomes sem validar não melhora o plano.

## Produtos de inspiração: sem alegação de GPUI

| Referência | Observação apoiada pela fonte | Aplicação no Xemnas — proposta | Adaptar ou rejeitar |
| --- | --- | --- | --- |
| **Linear** | A equipe descreve no [refresh de março de 2026](https://linear.app/now/behind-the-latest-design-refresh) controles em posições previsíveis, lateral mais discreta, ícones menores e redução de divisórias. Também relata comparação incremental por flags. | Uma mesma anatomia de cabeçalho, metadados e ações em Revisão, Decisões e Mapa; deixar o documento e o item selecionado dominarem a atenção. | Adotar coerência, alinhamento e comparação antes/depois. Não copiar tipografia, tema ou esconder texto necessário em abas só de ícone. |
| **Raycast** | O [Action Panel](https://manual.raycast.com/action-panel) organiza ação principal, grupos contextuais e busca de ações. A [busca](https://manual.raycast.com/search-bar) tem foco imediato e modo compacto. | A paleta deve indicar projeto/item e oferecer ações reais sobre a seleção, com atalhos descobríveis e retorno de foco. | Adotar arquitetura de ação. Não copiar janela de launcher como shell inteiro nem mostrar comandos impossíveis no estado atual. |
| **Superhuman** | A [equipe explica a paleta](https://blog.superhuman.com/how-to-build-a-remarkable-command-palette/): mesma porta de entrada, busca tolerante, relevância contextual e preservação de foco ao fechar; o texto também registra exceções. | Tornar Revisão um fluxo rápido de ler, agir e continuar, com atalhos visíveis e seleção estável após confirmação. | Adotar aprendizagem por uso. Não tornar atalho a única forma de executar nem duplicar uma paleta concorrente com Ctrl K. |
| **Obsidian** | A [documentação de Graph view](https://obsidian.md/help/plugins/graph) distingue grafo global de local. A página retornou conteúdo reduzido neste ambiente; profundidade, filtros e controles completos permanecem **(confirmar)** antes de especificar paridade. | Tratar visão geral como orientação e vizinhança como explicação da entidade selecionada. | Adotar a distinção de escala. Não dizer que o grafo global sozinho oferece compreensão ou que suas configurações foram auditadas aqui. |
| **Figma** | O [guia de zoom](https://help.figma.com/hc/en-us/articles/360041065034-Adjust-your-zoom-and-view-options) separa zoom de canvas da escala de UI, inclui enquadramento e foco no objeto vinculado. O [guia de teclado](https://help.figma.com/hc/en-us/articles/360040328653-Use-Figma-products-with-a-keyboard) documenta navegação do canvas. | Enquadrar tudo, enquadrar seleção e retornar da leitura à posição anterior; manter controles com tamanho legível ao ampliar o grafo. | Adotar navegação espacial. Não transformar Mapa em editor gráfico livre; níveis de detalhe semânticos são proposta nossa, não alegação sobre implementação interna do Figma. |
| **GitHub Primer** | A [orientação da Timeline](https://primer.style/product/components/timeline/accessibility/) exige ordem compreensível, marcadores consistentes, tipo também em texto, teclado e adaptação à largura. | Revisar timeline e relações: evento, estado, título, origem e acesso ao detalhe; condensar manutenção sem apagar significado. | Adotar semântica e critérios de aceitação. Não portar JSX/CSS para Rust nem confiar que o canvas herda semântica acessível de uma lista. |
| **Windows / Fluent** | O [guia de materiais](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/materials) distingue Mica, Acrylic transitório e Smoke modal. O [guia de APIs WinUI](https://learn.microsoft.com/en-us/windows/apps/develop/ui/materials) explicita mecanismos diferentes e fallbacks sólidos. | Chrome discreto, leitura sólida e overlays com hierarquia; manter o produto bonito com transparência desativada. | Adaptar os princípios. APIs XAML e WinUI não provam blur por componente em GPUI; material não deve ser condição para legibilidade. |
| **Things** | A [apresentação oficial](https://culturedcode.com/things/features/) descreve campos adicionais recolhidos até serem necessários, organização do conteúdo e transições entre lista e tarefa. | Documento de decisão com resposta e origem imediatamente visíveis; metadados raros em ação secundária, mantendo conteúdo essencial presente. | Adotar divulgação progressiva. Não esconder evidência e conflito, nem imitar papel branco contrariando Quiet Glass. |
| **Craft** | O [guia de estilo](https://support.craft.do/en/write-and-edit/styling) separa estilo de página, documento e texto. | Uma leitura editorial bem composta da Visão e das Decisões, com fontes próximas do trecho a que pertencem. | Adotar disciplina editorial. Rejeitar capas, cards e decoração por documento que dispersam a identidade e escondem informação técnica. |
| **Arc** | O [guia de Spaces](https://resources.arc.net/hc/en-us/articles/19228064149143-Spaces-Distinct-Browsing-Areas) define contextos separados, cada um com itens próprios, tema e ícone, navegáveis também por comando. | Projeto selecionado deve ser inequívoco; troca restaura contexto útil e não mistura resultados entre projetos. | Adotar limite de contexto. Rejeitar temas por projeto ou cores concorrentes sem mudança explícita de identidade. Não inferir roadmap atual de uma referência visual. |
| **GitButler** — verificação de framework | O [guia de desenvolvimento](https://github.com/gitbutlerapp/gitbutler/blob/master/DEVELOPMENT.md) declara Tauri e UI Svelte/TypeScript. | Pode entrar futuramente como referência de apresentação de mudanças, após inspeção própria. | **Não é exemplo GPUI.** Rust no backend não prova renderização GPUI. Não foi usado para derivar uma nova tela nesta pesquisa. |

## Viabilidade e ordem para o Xemnas

Estas conclusões são inferências de design cruzadas com a regra local; não são
resultados de benchmark dos produtos pesquisados.

1. **Alta confiança, sem fork:** posições previsíveis de ações; hierarquia de
   leitura; foco/restauração; rótulos de estado; listas virtualizadas; paleta
   contextual; overlays que não sobrepõem controles; esconder detalhes secundários
   sem retirar acesso. Implementar pelos padrões e tokens existentes.
2. **Canvas exige engenharia própria:** seleção, picking, navegação por teclado,
   zoom de câmera, rótulos por escala, resumo de agrupamentos e equivalência em
   lista. Figma e Obsidian inspiram o comportamento; não fornecem implementação
   GPUI nem garantia de 10 mil ou 100 mil entidades.
3. **Dependência externa sob avaliação:** GPUI Kit é útil para estudar foco,
   overlays e virtualização. Compatibilidade com a revisão fixada, licença de
   dependências e duplicação de padrões devem ser conferidas antes de integrar.
4. **Baixa prioridade:** docking generalizado, temas por projeto, blur de
   componente e um fork visual amplo. A pesquisa anterior já explica custos do
   fork; nenhum deles resolve o congelamento por quantidade de entidades.

### Aplicação imediata à evidência atual

A captura [Grafo compacto em Carvão](assets/vitrine/research-graph-charcoal-compact.png)
mostra os chips de camada e a dica de interação na mesma faixa, com textos
sobrepostos. É uma observação da imagem atual, não uma falha atribuída aos produtos
externos. O princípio de separar orientação, controles e canvas (Linear/Figma)
leva a uma correção concreta: reservar fluxo próprio para as camadas, recolher a
ajuda secundária quando faltar largura e manter enquadramento acessível. O plano
deve pedir verificação de nomes longos, contagens grandes e seleção nesse tamanho.

### O que faz o produto virar vitrine

Uma demonstração convincente deve revelar a cadeia real **evidência → decisão →
mapa → contexto entregue**. A hipótese é que essa continuidade comunica mais valor
que adicionar efeitos. Propor uma rota demonstrável com dados sintéticos claramente
identificados; na operação normal, só contagens, estados e ações reais. A aparência
precisa se sustentar numa captura parada, com movimento reduzido, num tema alternativo
e numa janela compacta. Velocidade e acabamento devem ser critérios da mesma entrega.

## Limitações e manutenção

- Fontes externas não foram clonadas, instaladas ou executadas. Relatos comerciais
  foram tratados como relatos, sem reproduzir números como resultado independente.
- Captura local analisada é do binário produzido pela sessão principal; este agente
  não capturou nem tomou foco. Imagens externas não foram baixadas nem republicadas.
- O objetivo do catálogo é cobrir famílias úteis, não prometer exaustão de todo app
  GPUI existente. App Stories oferece uma continuação verificável caso uma tela
  demande referência específica de tabelas, treeviews, terminais ou treemaps.
- Ao implementar, transportar só decisões duradouras para a identidade visual,
  design system ou ADR; remover a pesquisa e assets exclusivos conforme o índice
  de `docs/pesquisas/`. Esta nota não muda o escopo aprovado.
