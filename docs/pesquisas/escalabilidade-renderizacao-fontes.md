# Escalar grafos e blocos: fontes e opções de renderização

**Data:** 2026-10-02. Fontes consultadas nesta data.
**Pergunta:** como tornar o mapa em grafo e em blocos utilizável acima dos cerca de
2.500 itens relatados pelo usuário, preservando GPUI, dados reais e interação?
**Status:** Em implementação. Fontes, diagnóstico e plano técnico concluídos;
benchmark de layout em release executado e solver escolhido (Barnes–Hut acima de
300 nós); agrupamento, corte por viewport, modo barato e cancelamento entregues.
Benchmark de pintura no aplicativo, índice espacial e read models paginados abertos.

## Resultado e grau de certeza

A primeira direção recomendada é manter GPUI e substituir trabalho proporcional
ao universo por trabalho proporcional à região explorada: layout fora da thread
da interface, algoritmo de repulsão subquadrático, virtualização dos blocos,
descarte espacial antes da pintura e nível de detalhe no grafo. GPU própria é uma
opção posterior, condicionada a perfil de desempenho; GPUI já usa GPU.

**Confirmado no código:** existem custos suficientes para explicar uma trava.
**Ainda hipótese:** qual deles domina o travamento na máquina do usuário. Não
houve reprodução, perfil de CPU/GPU ou confirmação de um limite rígido de 2.500.
A busca por `2500`/`2_500` não encontrou um teto de nós no código do app; encontrou
apenas tempo de espera da ferramenta de captura. O número do relato deve ser
tratado como ponto de falha observado, não como limite intrínseco do GPUI.

O projeto fixa GPUI no commit
[`244023605536a412ab6b8d5b658466b89fb15401`](https://github.com/zed-industries/zed/tree/244023605536a412ab6b8d5b658466b89fb15401),
em `apps/desktop-gpui/Cargo.toml:39`. As APIs recomendadas abaixo foram conferidas
nessa revisão, também presente no cache Cargo local, em vez de assumir a API de
`main` ou de outro fork.

## Onde o trabalho cresce hoje

| Evidência local | Consequência | Confiança |
| --- | --- | --- |
| `graph.rs:240`, `set_graph`, chama `build`, `place` e `settle` dentro da atualização da entidade | Preparação/layout bloqueiam a thread que atualiza a interface | Confirmada por inspeção |
| `map.rs:394/431`, aplicação de dados chama `fill_graph` sem verificar `Layout::Graph` | A abertura em Blocos também paga o layout completo do grafo oculto | Confirmada |
| `graph.rs:47`, `SETTLE_STEPS = 420`; `graph.rs:1992`, `tick`, visita cada par de nós | Repulsão e colisão têm custo quadrático por iteração | Confirmada |
| `graph.rs:561`, `step`, roda `tick` e clona as arestas quando há calor | Arrastar/reorganizar também pode recolocar layout caro no frame | Confirmada |
| `graph.rs:1140`, montagem de `Scene`, clona os nós e arestas a cada renderização | Alocação e cópia acompanham o universo, incluindo textos e IDs | Confirmada |
| `graph.rs:1283`, `Scene::paint`, ordena os nós filtrados por camada e pinta todos; a máscara recorta a saída | Recorte visual não evita seleção, cópia, ordenação e preparação anterior | Confirmada |
| `graph.rs:1335`, `paint_links`, gera 18 segmentos por traço, 2–3 traços e cometas por aresta | Muitas arestas multiplicam primitivas e geração de caminhos | Confirmada |
| `graph.rs:331`, `hit`, percorre os nós para achar o alvo | Mouse pode pagar uma varredura por evento | Confirmada |
| `map.rs:2530–2585`, `render_overview`, monta todos os pares de cards e chips; para cada topo filtra todas as entidades | Árvore de elementos cresce com todos os cards e o agrupamento pode chegar a O(T × N) | Confirmada |
| `graph/query.rs:467`, `snapshot`, e `project_map`/`project_graph` em 513/575 | Os read models atuais materializam snapshots; paginar só na tela não elimina esse custo | Confirmada |
| `graph/query.rs:524–535`, para cada entidade calcula vínculos e percorre arestas para atividade | Agregados podem crescer com entidades × arestas; pré-indexar vínculos e atividade no read model | Confirmada |

Só o laço de pares executa `N × (N − 1) / 2` visitas por tick. Multiplicando pelas
420 iterações da primeira carga, sem contar outras etapas:

| Nós | Pares por tick | Visitas de pares em 420 ticks |
| --- | ---: | ---: |
| 2.500 | 3.123.750 | 1.311.975.000 |
| 10.000 | 49.995.000 | 20.997.900.000 |
| 50.000 | 1.249.975.000 | 524.989.500.000 |

**Primeira correção proposta:** preparar o grafo somente quando a vista Grafo
for solicitada; deixar marca de dados alterados e sincronizar em fundo nessa
transição. Essa mudança evita pagar a simulação ao abrir Blocos, mas não resolve
o grafo grande por si só. Carregar também seu read model sob demanda evita o
custo backend correspondente. O estado de progresso precisa ser real ao alternar.

São contagens calculadas do algoritmo, **não tempos medidos**. Reduzir 420 para
um número menor ajuda a constante, mas preserva o crescimento quadrático.

## Recursos disponíveis no GPUI fixado

[`canvas.rs`](https://github.com/zed-industries/zed/blob/244023605536a412ab6b8d5b658466b89fb15401/crates/gpui/src/elements/canvas.rs)
expõe callbacks de prepaint e paint com `Window` e `App`. É uma porta para as
primitivas de pintura; não fornece virtualização espacial nem um render pass
wgpu arbitrário. O grafo já usa essa porta. A otimização imediata é escolher a
cena visível antes de chamar suas operações de pintura.

[`uniform_list.rs`](https://github.com/zed-industries/zed/blob/244023605536a412ab6b8d5b658466b89fb15401/crates/gpui/src/elements/uniform_list.rs)
documenta renderização preguiçosa de itens de altura uniforme. Precisa de altura
limitada e overflow apropriado; se o pai exigir a altura integral do conteúdo,
o benefício pretendido pode desaparecer. Para blocos realmente uniformes, um
item da lista pode ser uma linha contendo dois cards, sem criar todos os cards.

[`list.rs`](https://github.com/zed-industries/zed/blob/244023605536a412ab6b8d5b658466b89fb15401/crates/gpui/src/elements/list.rs)
é a alternativa para alturas variáveis, mantém estado com `ListState` e
`SumTree`, e possui medição apenas dos itens visíveis. Alturas alteradas fora da
região precisam invalidar o estado por `splice`/`reset`. Isso se aplica melhor
aos blocos atuais, cujas partes e textos variam. Evitar a opção de medir tudo.

[`BackgroundExecutor::spawn`](https://github.com/zed-industries/zed/blob/244023605536a412ab6b8d5b658466b89fb15401/crates/gpui/src/executor.rs#L99)
executa uma future `Send + 'static` numa thread de fundo. O cálculo deve receber
dados próprios e devolver posições/índices, mantendo `Window`, `Context`,
`Rc<Cell<_>>` e estado de foco na thread da interface.

**Proposta para o app:** separar topologia imutável (`Arc`) de posições e estilo
por frame; preparar índices uma vez por mudança dos dados, publicar snapshots
compactos de posições em frequência limitada e aplicar na UI somente o último
resultado da geração atual. A frequência exata é parâmetro de benchmark, não
regra pronta. Um `async` que apenas executa CPU na foreground continua bloqueando.

## Layout: eliminar o custo quadrático

### Barnes–Hut com colisão local

A documentação primária de
[D3 many-body](https://d3js.org/d3-force/many-body) descreve uma quadtree e a
aproximação Barnes–Hut, com custo O(N log N) típico e controle de precisão por
`theta`. Essa fonte demonstra a técnica, não sua velocidade neste app Rust.
Distribuições degeneradas exigem cuidado e a aproximação muda o resultado.

**Adaptação proposta:** uma quadtree construída por tick agrega regiões
distantes; forças exatas ficam nas regiões próximas. A fórmula atual usa a
média das cargas de dois nós, portanto a agregação deve representar quantidade
e carga acumulada adequadamente; trocar por uma fórmula padrão sem revisão
pode mudar distâncias, agrupamentos e estabilidade. Testar contra o solver
exato em grafos pequenos usando erro e qualidade visual, não igualdade de floats.

O laço atual também resolve colisões. **Substituir apenas a repulsão deixa uma
possível etapa quadrática de colisões.** Separar broad phase espacial (grade ou
quadtree com raios/envelopes) e narrow phase exata dos candidatos próximos. A
[força collide do D3](https://d3js.org/d3-force/collide) é referência primária
para raios, força e iterações; as caixas dos rótulos do xemnas precisam de
tratamento próprio.

### Layout fora da UI, incremental e cancelável

A documentação de
[simulações D3](https://d3js.org/d3-force/simulation) recomenda calcular layouts
estáticos grandes num worker para evitar congelar a interface. No app nativo,
o equivalente viável é `BackgroundExecutor` com CPU limitada, não adicionar JS.

**Proposta:** desenhar imediatamente posições anteriores ou uma disposição
determinística barata; calcular em lotes, verificar cancelamento entre lotes,
publicar progresso real, rejeitar resultados atrasados por `generation_id` e
reter a última cena usável. Trocar projeto, data, filtro ou sair da tela cancela
a geração anterior. Uma tarefa CPU monolítica sem pontos de cancelamento não
fica responsiva só porque seu handle foi descartado.

Preservar posições por ID, colocar novos nós perto de vizinhos, relaxar só a
região alterada e estabilizar o restante protege o mapa mental. Não reaquecê-lo
inteiro ao selecionar um nó ou mudar somente a câmera. Uma mudança global da
topologia pode exigir recomputação completa em fundo. O código já preserva
posições conhecidas; a proposta amplia esse mecanismo, não começa do zero.

### Multinível e algoritmos alternativos

[Graphviz sfdp](https://graphviz.org/docs/layouts/sfdp/) é um algoritmo de forças
multinível para grafos grandes. Inspirar-se no esquema: contrair grupos,
resolver macroposições, expandir e refinar. O agrupamento inicial pode usar
componentes/`part_of` reais do domínio, evitando inventar comunidades sem
significado para quem usa o produto.

O [artigo original de ForceAtlas2](https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0098679)
combina Barnes–Hut, repulsão por grau e temperaturas adaptativas. É alternativa
para grafos de relações, não prova de que haverá 60 fps com cards e rótulos.
Sua simulação contínua deve parar quando estável neste produto.

[ELK Layered](https://eclipse.dev/elk/reference/algorithms/org-eclipse-elk-layered.html)
oferece layout em camadas, hierarquia, portas e diferentes roteamentos. Serve
como alternativa para fluxos direcionados/arquitetura, com maior custo de
integração no app Rust: runtime externo ou algoritmo equivalente nativo.
Não é necessário para eliminar a trava inicial.

Antes de escolher Graphviz como dependência de distribuição, validar o Windows:
a [galeria oficial de sfdp](https://graphviz.org/Gallery/undirected/root.html)
registra uma restrição de GTS na remoção de sobreposição `prism` no Windows.
Isso **não significa que sfdp inteiro seja indisponível**, mas impede assumir
que todo exemplo da galeria funciona no binário distribuído.

## Pintura: culling, nível de detalhe e arestas

### Descartar antes de pintar

O [tldraw documenta](https://tldraw.dev/sdk-features/performance) índice espacial,
descarte fora do viewport, cache de geometria, zoom estabilizado e simplificação
de detalhes. São técnicas transferíveis; seus números ilustrativos não são
benchmark de GPUI.

**Proposta:** inverter a transformação da câmera para obter o retângulo visível
em coordenadas do mundo. Consultar envelopes de nós e arestas com margem para
labels, halos e overscan. O crate Rust
[`rstar::RTree`](https://docs.rs/rstar/latest/rstar/struct.RTree.html) oferece
consultas por envelope e nearest-neighbor; grade uniforme é uma alternativa
simples para posições dinâmicas. Medir custo de atualizar o índice durante
layout: reconstruir uma R-tree integral por mouse move pode anular o ganho.

Não descartar uma aresta apenas porque seus dois extremos estão fora da tela:
a curva pode cruzar o viewport. Usar envelope conservador da curva e recorte
mais preciso se necessário. Na troca de cena, preservar o índice global ou
remapear endpoints; filtrar nós sem remapear `link.a`/`link.b` quebraria o desenho.
O hit-test deve consultar candidatos espaciais e respeitar ordem visual,
visibilidade, tamanho mínimo de alvo e área ocupada pelo painel lateral.

### Quatro escalas de leitura propostas

| Escala | Conteúdo | Relações | Rótulos/efeitos |
| --- | --- | --- | --- |
| Panorama | Componentes reais e contagens dos grupos | Conexões agregadas por tipo entre grupos | Só nomes dos componentes; sem cometas |
| Estrutura | Componentes e nós simples no viewport | Orçamento de arestas estruturais | Selecionado, foco e nomes prioritários |
| Exploração | Nós visíveis e vizinhança de interesse | Relações locais, com filtros e total omitido | Labels com teste de colisão e limite por área |
| Inspeção | Foco e painel com dados completos | Arestas do foco com direção/tipo | Detalhe atual, halo e movimento localizado |

São propostas de produto a validar com o padrão visual vigente. Os limiares
devem usar tamanho projetado/densidade, com histerese para não piscar ao fazer
zoom. Zoom-out põe quase tudo no viewport; por isso culling sozinho não basta.
Nenhuma escala pode apagar conflitos/sugestões sem sinalizar que existem.

**Orçamento de arestas proposto:** manter conflitos, seleção e caminho destacado;
em panorama agregar conexões, em exploração desenhar somente relações úteis.
Detalhes completos ficam consultáveis em inspeção. Distinguir dado ausente,
filtro e dado carregado mas omitido no desenho. Se o conjunto prioritário for
grande demais, também precisa de agregação explícita e acesso ao total.

[Sigma.js](https://www.sigmajs.org/docs/advanced/renderers/) separa programas GPU
para nós/arestas, usa primitivas simples e oferece picking por cores. A fonte
documenta que linhas simples são a opção mais eficiente de aresta; é evidência
de um trade-off de geometria, não motivo para substituir GPUI por webview.

**Adaptação proposta:** linha/curva simples no estado comum; os 18 segmentos,
traços múltiplos e cometas atuais somente para foco e cenas pequenas. Congelar
o grafo ocioso; animar câmera e foco com limite de entidades. Cachear ordem,
adjacência, geometria estática e medidas de texto; invalidar por dados, zoom,
tema, fonte/DPI e estado relevante. Um cache sem chave de DPI/tema produz erro
visual difícil de rastrear.

## Blocos: virtualização real e preservação de foco

**Proposta:** construir `children_by_parent` numa passagem quando os dados mudam,
sem refazer a filtragem de todas as entidades para cada card a cada frame.
Gerar IDs estáveis por card e lista ordenada de linhas; usar `list` para linhas
de altura variável ou `uniform_list` apenas se o desenho realmente equalizar
a altura. Cabeçalho e rodapé fixos ficam fora da lista com viewport limitado.

Não enfiar a lista dentro de um scroll pai que mede todo o conteúdo. Redimensionar
a janela ou mudar a quantidade de colunas invalida medidas e exige restaurar
âncora por ID; quantidade de colunas é uma decisão do padrão visual, não deve
ser escolhida só para facilitar o renderer.

Partes dentro de um único card podem formar milhares de chips e continuar
caríssimas mesmo com linhas virtualizadas. Mostrar resumo e contagem reais,
abrindo uma lista virtualizada de partes no detalhe. Virtualizar tecnologias
também; `flex_wrap` com todas as tecnologias não é virtualização.

O foco não pode desaparecer por uma reciclagem acidental. Manter a seleção
lógica por ID, revelar o item ao navegar por teclado e controlar crescimento
do cache de `FocusHandle`. Acessibilidade deve oferecer navegação/lista
semântica e detalhes equivalentes; um canvas rápido com objetos pintados não
substitui controles e descrições alcançáveis.

## GPU própria: viabilidade e custo no Windows

O [renderer Windows do GPUI fixado](https://github.com/zed-industries/zed/blob/244023605536a412ab6b8d5b658466b89fb15401/crates/gpui_windows/src/directx_renderer.rs)
usa Direct3D 11 e `DrawInstanced`; possui pipelines de quads, paths e sprites.
Portanto, “usar GPU” já descreve a base existente. O provável desperdício
anterior à GPU precisa ser medido primeiro.

No [Window dessa revisão](https://github.com/zed-industries/zed/blob/244023605536a412ab6b8d5b658466b89fb15401/crates/gpui/src/window.rs#L4971),
`paint_surface` está sob `cfg(target_os = "macos")`. Não há nessa API uma
superfície de textura externa portátil pronta para plugar um renderer wgpu
no canvas Windows. A presença de `gpui_wgpu` no lockfile não prova que o
backend Windows do produto use wgpu.

[`wgpu::RenderPass`](https://docs.rs/wgpu/latest/wgpu/struct.RenderPass.html)
expõe desenhos com intervalos de instâncias. Um renderer próprio poderia
manter buffers de posições/estilos, aplicar a câmera por uniform e desenhar
quads instanciados com distância assinada para círculos; edges simples em
buffers separados. Essa API é referência atual, não versão escolhida para o app.

| Caminho | Viabilidade | Custos e condição de uso |
| --- | --- | --- |
| Primitivas GPUI + culling/LOD/cache | Alta, sem fork | Primeiro experimento; mantém texto, foco e composição existentes |
| Elemento customizado usando o paint GPUI | Alta, sem GPU própria | Dá organização e cache; por si só não remove layout quadrático nem expõe shaders |
| Nova primitiva/shader no renderer GPUI | Média, requer fork/patch | Ganho potencial para edge/node batches; manutenção do fork e diferenças entre plataformas |
| wgpu offscreen + upload de imagem CPU para GPUI | Possível protótipo, não comprovado | Readback, cópia/upload e sincronização podem destruir ganho por frame; útil apenas como comparação |
| Compartilhar textura GPU nativamente | Incerta nesta versão | Exige ponte Windows/D3D, contrato de lifetime, sincronização, DPI e recuperação de device loss |
| Janela/superfície separada wgpu | Possível com grande integração | Complica composição, foco, overlays e interação; custo de produto alto |
| GPU compute para layout | Pesquisa posterior | Transferência, cancelamento, consistência e picking; não corrige árvore de cards ou SQL |
| Webview com Sigma/Cytoscape | Alternativa arquitetural | Outro runtime e integração de acessibilidade/tema; avaliar só se a rota nativa falhar nos critérios |

**Gate proposto:** só financiar a ponte GPU/fork se layout em background,
virtualização e cena limitada passarem, mas o perfil ainda apontar preparação
ou rasterização como gargalo dominante. Círculos GPU rápidos não garantem
labels, conectividade e seleção rápidos.

## Consultas e volume: não esconder o custo no backend

O backend precisa devolver projeções apropriadas: resumo por componente,
páginas de blocos, vizinhança com cursor, detalhes sob demanda e conectividade
agregada. Filtros temporais `as_of`, status, tipo de relação e escopo do projeto
devem permanecer consistentes entre todas as páginas. São novos casos de uso
do `application`; a tela não deve calcular regra de negócio.

A [documentação SQLite de row values](https://www.sqlite.org/rowvalue.html)
explica que `LIMIT/OFFSET` paga trabalho proporcional ao offset e propõe
comparação por tupla para janelas de rolagem. Preferir cursor estável com
ordenação total, desempate por ID e índice correspondente, verificando o plano
da consulta; o cursor do app deve também identificar revisão/data/filtros.

Paginar nós arbitrariamente pode devolver arestas cujos extremos ainda não
existem no cliente. Definir contrato: grupos e suas contagens, fronteira
expandível ou endpoints marcados como não carregados. Exibir `carregados / total`
quando aplicável. Um limite de trabalho por consulta é útil, mas não deve
virar truncamento silencioso que altera o significado do grafo.

[`SQLite R*Tree`](https://www.sqlite.org/rtree.html) oferece índice para consultas
por envelopes. É opção para uma cena persistida/segmentada muito grande, não
substituto automático da R-tree em memória durante animação. Persistir layout
exige chave por projeto, revisão dos dados, versão do algoritmo e parâmetros;
coordenadas antigas não podem explicar um snapshot temporal diferente.

## Matriz de decisão e sequência recomendada

Esforço relativo é estimativa qualitativa de engenharia, não prazo prometido.

| Opção | Resolve | Esforço | Limitação | Ordem |
| --- | --- | --- | --- | --- |
| Instrumentar carregamento, layout e frames | Identifica causa dominante | Baixo | Não corrige sozinho | 1 |
| Preparar/carregar grafo sob demanda | Layout do grafo oculto ao entrar em Blocos | Baixo/médio | Grafo grande ainda precisa das demais mudanças | 2 |
| Layout background + cancelamento + primeiro desenho barato | Congelamento inicial/atualizações | Médio | Algoritmo caro continua consumindo CPU | 2 |
| Listas GPUI + agrupamento pré-calculado | Construção/layout de milhares de cards | Médio | Cards gigantes também precisam de detalhe progressivo | 2 |
| Barnes–Hut + colisão espacial | Repulsão/colisão quadrática | Médio/alto | Aproximação/qualidade precisam de validação | 3 |
| Culling + hit-test espacial + snapshots compartilhados | Cópias/pintura/eventos fora da região | Médio | Panorama continua denso | 3 |
| LOD + orçamento de labels/arestas + ociosidade | Panorama e animação persistente | Médio | Exige desenho explícito da informação agregada | 4 |
| Read models paginados + resumo/revisão consistente | Materialização/SQL/memória | Alto | Contrato backend/front coordenado | 4 |
| Multinível + layout incremental persistido | Universo muito grande e estabilidade | Alto | Invalidação e qualidade são difíceis | 5 |
| Shader/fork/ponte wgpu | Gargalo GPU remanescente confirmado | Muito alto | Custo por plataforma e recuperação de dispositivo | Condicional |

Aumentar o teto para 10 mil antes de corrigir layout e cards tende a ampliar o
problema. O objetivo inicial é tornar **10 mil itens consultáveis e exploráveis**,
não prometer 10 mil cards detalhados ou 50 mil labels simultâneos. Depois,
experimentar 50/100 mil com agregação e carregamento progressivo.

## Protocolo concreto de benchmark

Nenhuma meta abaixo é resultado medido. Usar build release fixado e comparar
baseline com cada intervenção isolada; registrar commit, versão GPUI,
hardware/GPU/driver, resolução, DPI, paleta e preferência de movimento.

**Dados:** 250, 1.000, 2.500, 5.000, 10.000, 50.000 e 100.000 nós; árvores,
cadeias, muitas ilhas, hubs de alto grau, componentes profundos e recorte real
anonimizado. Para grafos esparsos testar E≈N, 4N e 10N; um subgrafo denso menor
estressa E separadamente. Evitar fabricar clique completo de 100 mil nós.
Nos blocos variar número de raízes, filhos por card, tecnologias e altura de texto.

**Separar medições:** consulta SQL/snapshot, agrupamento, layout inicial, tempo
até primeiro frame útil, seleção/cópia da cena, preparação de paths/texto,
CPU de frame, GPU e memória residente/pico. Registrar nós/arestas/labels
efetivamente desenhados e candidatos testados. Frame-time de GPU sozinho não
mostra 5 segundos de CPU antes do primeiro frame.

**Interações:** abrir mapa frio/quente, alternar Blocos/Grafo, rolar até o fim,
pan/zoom contínuo, zoom-out total, selecionar hub, arrastar, alterar filtro/data,
trocar projeto durante layout, abrir painel, redimensionar para janela compacta,
voltar repetidamente. Cancelamento, ciclos de carga e cache de foco precisam de
checagem de crescimento de memória.

**Metas iniciais propostas:** interação p95 abaixo de 16,7 ms como alvo de 60 Hz,
e p99 abaixo de 33,3 ms para evitar quedas persistentes; resposta visível de
ações comuns em até 100 ms; primeiro estado útil em até 500 ms quando dados
já estão disponíveis; zero longos bloqueios do loop da UI durante layout.
Medir também piores episódios, não só média. Em hardware fraco acordar uma
meta alternativa explícita, em vez de afirmar aprovação com base na máquina forte.

**Qualidade funcional:** contagens e relações corretas no snapshot, nenhum
endpoint inválido, nenhuma curva cruzando viewport perdida pelo culling,
seleção/foco preservados, estabilidade espacial de nós existentes e
recuperação de erro/cancelamento. LOD deve mostrar o que foi agregado/omitido.

**Validação futura no repo:** executar fmt, clippy e testes pertinentes,
architecture e `cargo check -p desktop-gpui` no Windows; inspecionar linhas
longas; validar visualmente binário recém-gerado nas paletas e janela compacta.
Capturas que tomam foco ou clicam precisam da autorização prevista em AGENTS.md.
Capturas demo em segundo plano foram feitas para a pesquisa visual, sem foco
ou cliques. Esta pesquisa não apresenta FPS, consumo ou limites de escala
como se fossem medidos.

## Limitações e decisões pendentes

- A causa dominante da trava e o total de arestas do cenário real permanecem
  sem perfil. O limite de 2.500 veio do relato, sem reprodução instrumental.
- Não foi integrado nenhum crate de layout/spatial index nem testada uma ponte
  GPU. Licença, manutenção e compatibilidade de dependências futuras precisam
  de conferência na versão escolhida.
- Referências D3, tldraw, Sigma, Graphviz e ELK fundamentam técnicas; seus
  limites e tempos não são transferíveis ao xemnas por comparação documental.
- O primeiro experimento deve provar fluidez no hardware alvo e preservar
  significado/foco. Só então escolher o tamanho operacional e alterar eventual
  limite de dados, com progresso real e degradação explícita.

## Microbenchmark local reproduzível (02/10/2026)

O diagnóstico acima foi acompanhado de um experimento isolado, sem modificar o
app. Código e dados: [tick-benchmark.rs](assets/escalabilidade/tick-benchmark.rs)
e [tick-resultados.csv](assets/escalabilidade/tick-resultados.csv).

- Base inspecionada: `7cb779a9754ae2ff51d6c629b6c14f3e38c7440c`.
- CPU: AMD Ryzen 5 5600G, 6 núcleos / 12 threads; Windows.
- Compilador: `rustc 1.98.1 (48a229cea 2026-09-01)`; `opt-level=1`, igual ao
  workspace em desenvolvimento. Não representa uma medição release.
- `tick` e `Node::{radius,charge}` extraídos sem mudar os cálculos; `Node`
  reduzido aos campos usados, com layout de memória diferente do objeto real.
- Ilhas de 25 nós, um componente por ilha, demais nós decisões; uma aresta
  `Affects` por decisão, comprimento de repouso 58. Posições iniciais numa grade.
- 420 passos e resfriamento iguais aos de `settle`; três repetições por tamanho,
  checksum consumido por `black_box`. Nenhuma compilação ou captura concorrente
  durante a rodada registrada. Não se controlaram energia, temperatura e demais
  processos do Windows; três repetições são uma exploração, não um p95.

| Nós | Arestas | Mediana dos 420 passos | Faixa das 3 execuções |
| --- | --- | --- | --- |
| 100 | 96 | 10,458 ms | 10,012–10,495 ms |
| 500 | 480 | 253,924 ms | 240,016–262,364 ms |
| 1.000 | 960 | 970,915 ms | 957,654–1.007,133 ms |
| 2.500 | 2.400 | 6.070,531 ms | 6.001,959–6.147,006 ms |

A ordem de grandeza é compatível com um bloqueio perceptível se esse trabalho
acontece na thread de UI. **Não é o tempo de abertura do aplicativo, FPS ou uma
prova de toda a causa do travamento relatado.** Não mede `build`, `place`, strings,
consulta SQLite, pintura, GPU, drag, máscaras, acessibilidade nem topologias densas.
Não mede Barnes–Hut ou qualquer ganho da solução proposta. Rodadas preliminares
tiveram concorrência com build/capturas e foram descartadas da tabela, inclusive
uma de 5.000 nós. Os dados versionados são da execução final sem esses trabalhos.

Reprodução no PowerShell, da raiz, com saída fora do repositório:

```powershell
$benchExe = Join-Path $env:TEMP 'xemnas-tick-benchmark.exe'
rustc -C opt-level=1 --edition=2021 `
  docs/pesquisas/assets/escalabilidade/tick-benchmark.rs -o $benchExe
& $benchExe
```

O arquivo foi formatado, `rustfmt --check` passou e sua versão final recompilou.
`cargo build --locked -p desktop-gpui --bin xemnas` passou no Windows; foram
capturadas telas demo do binário recém-gerado sem foco ou input. Elas usam poucos
nós e servem à pesquisa visual, **não validam desempenho em 2.500 nós**. Como esta
entrega altera somente pesquisa e experimento isolado, não foram rodados
clippy/test de crates de produção. A próxima implementação precisa dos checks
proporcionais exigidos no AGENTS.md e do benchmark dentro do aplicativo.

Para localizar o gargalo na aplicação real, complementar os spans propostos com
[WPR/WPA e CPU por thread e stack](https://learn.microsoft.com/en-us/troubleshoot/windows-server/support-tools/support-tools-xperf-wpa-wpr).
Uma trace ETW completa não foi coletada nesta pesquisa. Não assumir que o
renderer DirectX do Windows reproduz resultados publicados para Metal no macOS.

## Benchmark de layout dentro do repositório (release, 02/10/2026)

`layout_benchmark` (teste ignorado em `screens/graph.rs`) compara três solvers
sobre o mesmo mapa sintético, com as 420 iterações planejadas de `settle`
(reduzidas acima de 200 nós, como no app). Reproduzir:

```powershell
cargo test --release -p desktop-gpui --lib layout_benchmark -- --ignored --nocapture
```

- **Solvers:** `AllPairs` (o laço de pares original, referência exata),
  `Grid` (grade de 420 px; o que o app usava) e `BarnesHut` (quadtree com
  `theta` 0,9, alcance de 420 px como a grade e colisão por grade fina de 80 px).
- **Mapas:** `ilhas` (ilhas de 25 nós: um componente e 24 decisões, E ≈ N) e
  `hub` (um componente com todos os outros nós ligados: pior caso para a grade).
- **Máquina:** AMD Ryzen 5 5600G, Windows, `rustc 1.98.1`, perfil release; duas
  execuções por célula, menor tempo registrado; nenhuma compilação ou captura
  concorrente durante a rodada. Dados em
  [layout-resultados.csv](assets/escalabilidade/layout-resultados.csv).
- **Qualidade:** `overlaps` = pares de formas que se sobrepõem; `link_ratio` =
  comprimento médio das ligações sobre o repouso (1 seria o ideal); `extent` =
  raio que contém o desenho.

### Mapa em ilhas (ms)

| Nós | AllPairs | Grid | Barnes–Hut | BH contra Grid |
| ---: | ---: | ---: | ---: | ---: |
| 250 | 74 | 156 | 96 | 1,6× mais rápido |
| 1.000 | 360 | 576 | 172 | 3,3× |
| 2.500 | 2.473 | 2.265 | 578 | 3,9× |
| 5.000 | 9.589 | 5.870 | 1.269 | 4,6× |
| 10.000 | (não rodado) | 16.820 | 2.850 | 5,9× |

Qualidade equivalente: 0 sobreposições até 5.000 nós em todos; em 10.000, 4 na
grade e 2 em Barnes–Hut; `link_ratio` 1,44 a 1,92 nos três, com Barnes–Hut
ligeiramente menor.

### Mapa em hub (ms)

| Nós | AllPairs | Grid | Barnes–Hut |
| ---: | ---: | ---: | ---: |
| 250 | 67 | 167 | 141 |
| 1.000 | 321 | 778 | 370 |
| 2.500 | 1.979 | 5.018 | 1.418 |
| 5.000 | (não rodado) | 20.327 | 4.601 |
| 10.000 | (não rodado) | (não rodado) | 13.527 |

Aqui as sobreposições existem nos três (183 a 221 em 2.500 nós): milhares de
nós ao redor de um só ponto não cabem sem se tocar; é o caso que o agrupamento
em nó de grupo evita no produto.

### O que isto decide

- **A grade nunca foi a mais rápida.** Pagava o custo de tabela de espalhamento
  e ordenação sem poupar pares quando as ilhas são densas. Foi retirada do app
  (fica no benchmark como referência).
- **Barnes–Hut vence a partir de ~1.000 nós, de 3× a 6×, sem perder qualidade.**
  Em 250 nós o laço exato é o mais rápido e é exato; o app usa `AllPairs` até
  300 nós e `BarnesHut` acima (`solver_for`).
- **10.000 nós ainda levam ~2,9 s** (ilhas) em fundo: não congelam a interface,
  mas o resultado só aparece depois. É o orçamento a atacar com layout
  incremental e multinível (ver abaixo), não com mais constante.
- Tempos são do layout isolado; não incluem `build`, SQLite, pintura nem GPU.
- Limite: duas execuções por célula, uma máquina; não é p95. O experimento
  anterior (`tick-resultados.csv`, 6,07 s a 2.500 nós em `opt-level=1`) e este
  (2,27 s para a mesma grade em release) não são comparáveis: perfis diferentes.

### Entregue a partir destes dados

- Solver escolhido por tamanho e **cancelamento entre passos**: a geração mais
  nova avisa por um contador atômico e o layout em fundo desiste, em vez de
  terminar um resultado que será descartado.
- Agrupamento de multidões (nó de grupo), corte por viewport e modo barato de
  pintura (ver `docs/arquitetura/desempenho-e-escala.md`).

## Benchmark de pintura dentro do aplicativo (release, 02/10/2026)

Mede o tempo que `Scene::paint` leva para montar as operações de um quadro
(`Probe` "graph-paint", `XEMNAS_PERF=1`), com o mapa semeado por
`XEMNAS_DEMO_SCALE` e a janela fora da tela (`--demo --background --open
map:graph`), na mesma máquina. `XEMNAS_GRAPH_RENDER` escolhe quanto trabalho se
poupa: `full` (todos os nós e ligações, todos os efeitos), `cull` (corte pela
viewport), `cheap` (mais o modo barato em vista cheia) e `lod` (o do app, mais
o panorama). Ambos medem o quadro no zoom que ajusta o mapa inteiro à janela,
~30 quadros por célula; é tempo de CPU para montar a pintura, **não** de GPU.

| Escala | full | cull | cheap | lod (app) |
| --- | ---: | ---: | ---: | ---: |
| 1.000 (259 entidades) | 9,44 ms | 9,31 ms | 3,12 ms | **1,52 ms** |
| 2.500 (626 componentes) | 10,35 ms | 7,16 ms | 5,11 ms | **2,46 ms** |

Medianas; máximos de `lod`: 2,5 ms e 4,7 ms (contra 16,6 e 38,3 ms em `full`).
Layout em fundo, mesma execução: 108 ms (1.000) e 321 ms (2.500), contra 566 ms
em 2.500 antes do Barnes–Hut (grade, mesma semente).

Leitura:

- **Corte pela viewport não ajuda no zoom de ajuste**: tudo está na tela. Seu
  valor aparece com o zoom aproximado, que este protocolo ainda não mede.
- **O modo barato** (ligações retas de um traço, sem sinal, nós sem halos) vale
  de 1,8× a 3×. **O panorama** (esconder decisões e regras individuais abaixo
  de zoom 0,45, mantendo conflitos e sugestões) vale mais 1,7× a 2×.
- Os dois juntos levam o quadro de ~10 ms a ~2 ms em ambas as escalas, dentro
  do orçamento de 16,7 ms com folga mesmo contando GPU e composição.
- Não medido: GPU, rolagem e zoom contínuos em uso real, 5.000 nós e mais
  (semear 5.000 decisões leva mais de 100 s), hit-test. Uma `RTree` ou grade
  para o hit-test continua só uma hipótese; a varredura linear de 2.500 nós por
  movimento do mouse é pequena diante do que foi poupado.

## Consequências para o app

- **Solver:** laço exato até 300 nós; Barnes–Hut acima.
- **Pintura:** modo barato com mais de 140 ligações à vista; panorama abaixo de
  zoom 0,45 em mapas com mais de 150 nós, com histerese de 0,1 e o aviso
  "Panorama · N decisões e regras ocultas" (nada some sem dizer).
- **Em aberto:** layout incremental e multinível para 10.000+ nós, índice espacial
  para o hit-test, read models paginados no backend (sessão de backend).
