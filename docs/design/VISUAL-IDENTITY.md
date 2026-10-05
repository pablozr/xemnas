# Xemnas — Quiet Glass

Este é o padrão visual do aplicativo desktop GPUI. Mudanças de identidade precisam
ser explícitas; não inventar telas, dados ou ações para preencher espaço. A
referência extensa original está em `docs/design/design-system-quiet-glass.md`;
onde as duas divergirem, vale este arquivo, junto com `ui/tokens.rs`.

## Hierarquia e material

- Base carvão escura, texto de alto contraste e lavanda mineral discreta. Cores,
  tipografia e espaçamento vêm de `apps/desktop-gpui/src/ui/tokens.rs` e `theme.rs`;
  não espalhar literais de cor pelas telas.
- Superfícies grandes são quietas. Reservar lavanda para seleção, foco e uma
  ação primária real; campos de busca são neutros em repouso. Não usar card,
  glow, blur ou animação para mascarar a falta de informação. Movimento segue
  a seção **Movimento** abaixo.
- Densidade de ferramenta: interface em 13 px, leitura em 14 px, títulos de
  painel em 12 px esmaecidos. Inter em toda a interface; Bricolage Grotesque só
  no wordmark e no título de leitura (a pergunta). Nomes de família vêm de
  `resolve_font_families` (o Windows registra "Inter Variable Text" e
  "Bricolage Grotesque 14pt") e pesos usam instâncias nomeadas (400/500/600).
  Títulos são compostos por palavra para a pontuação não quebrar sozinha. Controles têm 28 ou 32 px (`ControlSize`), raio de
  6 px, e vêm de `action_button`/`icon_action`; telas não desenham botões próprios.
- Uma receita de seleção para toda lista: fundo `selection` e barra lavanda de
  2 px (`mark_selected`). Foco visível é um anel inset que não desloca o layout.
- Cada tema é uma tabela `Palette` em `tokens.rs`: Quiet Glass (grafite e
  lavanda), Carvão, Organização (preto e prata fria), Musgo (verde-tinta e
  sálvia) e Meia-noite (marinho e ciano frio). O tema define também o
  destaque (seleção, foco, primário) e a tonalidade dos vidros, véus e
  bordas; nenhuma cor de tema fica fora da tabela, e o teste de contraste
  WCAG AA roda em todos. Toda cor nova precisa existir em todos os temas.
- O tema escolhido fica salvo (`settings/appearance.json`, `ui::appearance`)
  e vale antes da primeira janela. Escolha pelo botão de contraste da barra
  de título (menu com os cinco e "Fundo e mais opções…") ou em
  Configurações › Aparência, onde cada tema é uma linha de rádio com uma
  miniatura do app naquelas cores. Rotas de captura: `settings:appearance`,
  `theme-menu[:rota]`; `-Theme <id>` no script de captura.
- A janela é opaca por padrão. Glass (material do Windows) é experimental e só
  liga com `XEMNAS_BACKDROP=mica-alt`, `mica` ou `acrylic`, com o sistema em
  modo escuro. Camadas, conforme o
  Fluent: barra de título e lateral formam uma moldura translúcida sem emendas
  (`color.chrome`); o conteúdo é um card flutuante (`color.content`, ~88%) com
  8 px de margem, raio de 10 px, borda e sombra de card; painéis internos não
  têm fundo próprio (`color.pane`); evidência e código ficam sólidos;
  superfícies transitórias usam `color.floating`. O GPUI não tem blur por
  elemento. Pesquisa: `docs/pesquisas/glass-na-janela-inteira.md`.
- Feedback: confirmações são toasts que saem sozinhos; erros recuperáveis são
  uma faixa no topo com a ação real de repetir; listas carregando mostram
  esqueleto; superfícies vazias usam `empty_panel` (marca, rótulo, título, o que
  as faz encher).
- Teclado: Ctrl K abre a paleta de comandos (só o que está carregado e ações
  reais); Ctrl 1/2 trocam de destino; J/K percorrem listas; C, R, S e A agem
  sobre o candidato, com a tecla visível no botão; Ctrl Enter salva editores.
  Atalhos de uma tecla não disparam com um campo de texto em foco. Controles
  só de ícone têm tooltip.
- Mica pode compor o fundo da janela no Windows; isso **não** garante backdrop
  blur por componente. Superfícies internas precisam funcionar sem blur. No
  padrão opaco, ou numa máquina sem material, nada quebra: nenhuma superfície
  depende do blur.
- Cada informação tem um lugar: contagem na lista lateral, título da seção na
  própria seção; não repetir a mesma informação no título e no rodapé.
- **Um glifo por conceito** (`ui/icons.rs`): Revisão = `Inbox`; decisão =
  `Decision` (placa); componente = `Component` (caixa); tecnologia = `Cpu`;
  regra = `Shield`; documento do projeto = `Book`; arquivo de código e
  evidência = `File`; Contexto = `Layers`; Mapa = `Graph`; Visão = `Compass`;
  sugestão = `Lightbulb`; ligação do mapa = `Link`; visão geral de um destino =
  `Gauge`; testar uma tarefa = `Flask`; objetivo = `Flag`; escopo = `Target`;
  ferramenta de linha de comando desta máquina (Claude Code) = `Terminal`.
  Um ícone novo entra nessa lista; nunca reaproveitar um glifo para outro
  conceito.
- **Títulos de seção têm uma forma só**: `section_header` (rótulo em
  versalete META esmaecido) com, à direita, uma contagem (`count_chip`) ou uma
  ação discreta. Não há título em negrito com ícone dentro de uma página de
  leitura; grupos de uma lista (tipos de regra, tipos de documento) usam o
  mesmo cabeçalho.
- **Pílula com bolinha é estado** (`status_pill`: Pendente, Confirmada,
  Enviado…). Tipo e verbo ("Regra", "Restrição", "depende de") são `tag`, a
  mesma forma sem bolinha; "conflita com" leva o texto em âmbar. Âmbar é
  aviso real (conflito, defasagem, revisar); Pendente é neutro.

## Fundo com imagem

- Configurações › Aparência › Fundo: "Sem fundo", quatro fundos do app e
  "Sua imagem" (seletor do sistema; PNG, JPEG ou WebP), em blocos de
  164 × 92 com anel de destaque no escolhido (a borda existe em todos, então
  escolher não move a grade). Com um fundo, três controles segmentados por
  palavra (`segment_label`): Desfoque (Leve, Médio, Forte), Escurecer (Pouco,
  Médio, Bastante) e Superfícies (Mais vidro, Equilibradas, Mais sólidas).
- O GPUI não desfoca o que está atrás de um painel, então a imagem é
  desfocada uma vez (`ui::wallpaper`): reduzida (1600, 900 ou 480 px de
  largura), desfocada, escurecida e guardada, fora da thread da interface; a
  anterior fica até a nova estar pronta. Barra, lateral e cartão de conteúdo
  viram vidro sobre ela (`ColorTokens::with_veil`), com o véu do tema pintado
  uma vez sobre a imagem inteira.
- Legibilidade: além do nível escolhido, a luminância média da imagem
  escurecida nunca passa de 0,22; uma imagem clara é puxada para baixo.
  Arquivo que sumiu vira aviso na seção e o app volta ao tema sólido.
- Fundos do app, inspirados na Organização (imagens originais, sem arte,
  logo ou emblema oficial), gerados por `tools/wallpapers/render.py`: "A
  cidade que nunca existiu" (torres, lua enorme, chuva), "O castelo" (torres
  brancas no céu violeta), "Onde nada se reúne" (treze tronos em círculo) e
  "Corrente" (elos prateados e brasas). Captura: `-Wallpaper <id>`.

## Marca

- Símbolo: cruzamento de decisão. Dois traços em X; o escolhido (lavanda)
  atravessa inteiro, a alternativa (clara, mais apagada) é interrompida no
  cruzamento. Bloco carvão com leve viés lavanda, borda interna de 1 px, sem
  brilho. Fonte única: `apps/desktop-gpui/assets/brand/xemnas-mark.svg`.
- PNGs do app e o `xemnas.ico` são renderizados do SVG em cada tamanho (16 a
  512 px), nunca reduzidos de um só bitmap. `tools/brand-icon.ps1` compila
  `xemnas.rc` em `xemnas.res`, ligado ao executável pelo `main.rs` (ícone ID 1
  e versão), sem build script.
- Wordmark: "xemnas" em Bricolage Grotesque, minúsculo, ao lado do símbolo.

## Xemnas — o mascote e o assistente

- O mascote é o próprio Xemnas, uma homenagem de chibi à Organização (casaco
  preto de capuz, cordões e zíper prateados, franja prateada, olhos âmbar
  acesos no rosto em sombra). É um desenho original: não copia o modelo
  oficial do personagem nem o emblema dos Nobodies.
- É 3D pré-renderizado: `tools/mascot/model.py` (modelo de distâncias com
  sombreamento toon, contorno e brilho nos olhos) e `tools/mascot/render.py`
  geram `assets/mascot/sheet.png` (32 quadros de 160 px, 8 × 4, mesmo
  recorte) e `idle`, `blink`, `glow` avulsos para retrato e estados vazios.
  O app só exibe as imagens; não há 3D em tempo real.
- Fica no pé da lateral, acima da linha de status: figura de 72 px, "Nº I"
  em mono prata e uma frase real ("5 para revisar", ou o nome), com ponto
  âmbar quando há fila. Respira a cada ~5 s (sobe 3 px e assenta em 2,2 s)
  e fica parado entre uma respiração e outra; pisca e olha para os lados.
- Fluidez vem do relógio, não do layout (`screens/mascot.rs`). A folha tem
  27 giros da cabeça (a cada 0,03 rad), 4 passos de piscada e 1 quadro de
  olhos acesos; cada gesto escolhe o quadro pelo tempo decorrido, com
  entrada e saída suaves (`smooth`, cosseno elevado na respiração). Cada
  quadro sobe com margem e é pintado numa janela fixa, alinhada ao pixel,
  enquanto a imagem desliza por frações de pixel (o GPUI recorta a origem
  em texels, ~0,45 px a 72 px): a flutuação deixa de andar de 1 em 1 px. O
  olho aceso é o quadro `glow` sobre o base, com opacidade de 160 ms. Só se
  pede o próximo quadro enquanto um gesto dura; parado, não redesenha. É uma
  view própria e as telas são views em cache (`cached`), então um quadro do
  mascote não refaz o app. Com movimento reduzido, só pisca. Hover e painel
  aberto acendem os olhos.
- Clique, Enter ou Ctrl K ("Falar com o Xemnas") abrem o painel ao lado da
  lateral (400 × 540, `floating`, raio `dialog`, sombra de ênfase): retrato,
  "Xemnas" em Bricolage, "Nº I · assistente do projeto", a corrente prata,
  uma fala com os números reais do projeto e "Posso levar você a" com
  destinos reais. O rodapé diz que perguntas livres chegam quando o
  assistente for ligado ao provedor; nada de conversa simulada. Esc e o X
  fecham. Rota de captura: `assistant[:rota]`.
- Referências à Organização, sempre discretas: numerais romanos para o que
  tem ordem (etapas do Contexto, fluxos e passos da Visão; `format::roman`),
  a corrente prata (`chain_rule`) só no assistente, e os tokens
  `organization.silver` e `mascot.ember`, restritos ao assistente.

## Projetos — superfície atual

- Lista persistente de projetos de 248 px (linhas de 48 px com nome e caminho) e
  workspace selecionado ocupando toda a área restante. O nome do projeto na
  barra abre um painel com as propriedades reais (localização e data) e a
  remoção secundária; Esc ou clique fora fecha, trocar de projeto também.
  Não impor um card de largura fixa
  nem criar métricas ou atividade inexistentes.
- A linha selecionada conserva fundo e indicador lateral sob hover; foco visível
  e estado acessível precisam concordar. Trocar de projeto fecha a confirmação
  anterior. Após remover, focar um controle que permaneça visível.
- Busca é edição de texto real: caret, seleção, colagem, IME e filtro coerentes.
  Resultado vazio não exibe detalhe fora da busca.
- Abrir pasta usa o seletor do sistema para registrar o caminho; não copia nem
  apaga o conteúdo. Cancelamento não altera a lista. Erros de validação e de
  armazenamento têm linguagem de produto e recuperação honesta.
- Em janela menor, permitir rolagem das propriedades e confirmação sem perder
  acesso às ações. `WindowControlArea` limita arraste e controles da janela;
  busca permanece fora da região arrastável.

## Revisão — candidatos por projeto

- Qualificadores de atribuição, alcance e validação acompanham a leitura e os
  editores; distinguir fonte citada de declaração do revisor. Não ocultar dados
  malformados como ausência. Evidência na edição reutiliza `screens::evidence`.
- Progresso recente de capturas pertence ao vazio da Revisão e ao Diagnóstico,
  não interrompe o documento do candidato. Estado/motivo e retry vêm do backend.
  Polling somente visível preserva drafts, seleção e evidência; ausência na janela
  paginada não comprova que o candidato foi resolvido. Toast nomeia regra/decisão
  e vínculos realmente concluídos.

- **No mapa** (entre Motivo e Evidências): o que confirmar põe no mapa. Uma
  linha por vínculo que a evidência aponta, com caixa marcada por padrão
  (lavanda quando marcada), o verbo ("muda", "usa", "vale para"), ícone do
  tipo, nome do item e, em mono, o arquivo ou a dependência que justifica;
  contagem "2 de 3" no título. Desmarcar recusa o vínculo (não volta como
  sugestão). Arquivos sem componente aparecem numa linha META. Sem nada a
  ligar, a seção não aparece. O toast diz "Decisão criada e ligada ao mapa."

- A barra da janela (40 px) contém marca, selo de demonstração quando houver,
  tema e controles nativos. Abaixo dela, uma única barra de projeto de 44 px leva
  o nome do projeto como trilha (abre o painel do projeto) e as abas Revisão e
  Decisões; o caminho fica na lista lateral e no painel. Não há barra de status:
  o rodapé da lateral mostra só estados reais (captura ativa, indisponível ou
  demonstração; jobs extraindo; falhas).
- Em Decisões o índice fica à esquerda, como em todo destino. Não há segunda fileira de abas: estado, versão, o acesso ao
  histórico ("Versões") e as ações Exportar/Revisar ficam na linha de metadados
  do documento. Linhas de lista e abas têm hover com mola criticamente amortecida.
- **Decisões por parte.** Sob "Confirmadas", um segundo filtro do índice
  (botão fantasma com o glifo de componente e seta) abre um menu: "Todas as
  partes" e cada parte do projeto (os componentes de topo do Mapa, os mesmos
  "containers" da arquitetura) com a contagem de decisões em vigor nela ou num
  componente dentro dela (`KnowledgeGraph::decision_parts`). Escolher uma
  estreita a lista (paginada por `Decisions::list_ids`) e a busca; o botão
  passa a "storage-sqlite · 2" com fundo de seleção, "Confirmadas" vira "Em
  vigor · parte" e o rodapé diz "2 de 2 em vigor · storage-sqlite". Limpar: o
  X ao lado (tooltip com Esc) ou Esc fora da busca. Parte sem decisões:
  `empty_panel` "Nada decidido sobre X ainda", dizendo que a decisão aparece
  quando ligada a X no Mapa, e "Mostrar todas as partes". Menu: setas, Enter e
  Esc; o foco vai à opção escolhida ao abrir e volta ao botão ao fechar.
  Paleta: "Decisões por parte" e "Decisões sobre <parte>". Rotas de captura:
  `decisions:parts` (menu aberto) e `decisions:part:<nome>`.
- **Menu suspenso** (`ui::patterns::menu_panel` + `menu_item`): a superfície
  do menu de tema (`floating`, borda de cartão, elevação flutuante, até 320 px
  e depois rola), sob o gatilho, com `menu_in`/`menu_out` e `Popup`. Opção:
  glifo, rótulo truncado, contagem opcional (`count_chip`) e o check lavanda
  na escolhida; hover tinge, sem segundo fundo de seleção.
- Trocar de projeto limpa lista, evidência e filtro; respostas antigas não podem
  aparecer no novo workspace. O filtro de projeto é aplicado no caso de uso.
- Lista de candidatos de 320 px, com data, estado, pergunta e escolha proposta.
  A badge da aba conta toda a fila do projeto; a lista informa carregados e
  visíveis separadamente. A busca filtra esses itens;
  fica dentro da lista; paginação explícita permite carregar mais sem sugerir
  uma busca global. Ctrl F foca a busca do destino atual.
- Leitura ocupa o restante da janela numa coluna de 760 px com a mesma gramática
  do documento de Decisões: escolha destacada, rótulos de seção em versalete,
  motivação, evidências, confiança da extração e origem. Confiança é uma estimativa do extrator, não uma avaliação humana.
- Evidências vêm dos artefatos reais já redigidos pelo backend. Mostrar uma fonte
  por vez em tipografia monoespaçada, com seleção acessível por teclado.
  Revisão e Decisões usam o mesmo componente (`screens/evidence.rs`): abas com
  ícone de arquivo e sublinhado, legenda com caminho monoespaçado, numeração à
  direita e diff com linhas `+`/`-`/`@@` tingidas. Caminho e linhas
  usam metadados registrados; o código tem rolagem horizontal e vertical,
  altura limitada e linhas virtualizadas. Abas extensas rolam horizontalmente.
  Estados têm badge com texto e bolinha: Pendente (neutro), Adiado, Confirmado e Rejeitado.
- Sem candidatos, orientar sobre captura e extração. Falhas mostram recuperação
  em linguagem de produto; conteúdo técnico do erro não aparece na interface.
- Confirmar, Rejeitar, Adiar/Retomar e Ajustar usam os casos de uso da Inbox.
  Ações ficam visíveis fora da rolagem de leitura, bloqueiam envios duplicados
  durante a operação e atualizam fila e contagem. Ajustes só persistem ao salvar;
  cancelar não escreve. Salvar e confirmar cria a decisão com os ajustes.
- `xemnas --demo` abre uma prévia identificada com dados fictícios em memória,
  sem iniciar integrações, workers ou alterar o banco normal.
  `--demo --long-evidence` permite verificar fontes com 1.500 linhas.
- Decisões tem a seção **Relações** depois das evidências: cada linha diz como a
  relação se lê a partir da decisão aberta (Substitui, Substituída por, Depende
  de, É base de, Conflita com), abre a outra decisão e marca a substituída.
  **Relacionar** abre um seletor inline (tipo + decisões em vigor carregadas);
  substituir tem borda de aviso e explica que a escolhida sai do contexto do
  agente e fica no histórico.

- **Relevância (ADR-0006):** a fila mostra só candidatos com relevância a
  partir de 0,5; quando há escondidos, "Mostrar N de baixa relevância" (botão
  fantasma sob a busca) os inclui. Candidato a regra leva a `tag` "Regra" ao
  lado do estado; "Por que importa" lista os critérios
  marcados em texto meta abaixo do cabeçalho.

## Contexto — o que o agente recebe

- Terceiro destino do projeto (Ctrl 3). Mesma gramática do Mapa: índice à
  esquerda (`index_rail` + `index_row`, 296 px) e uma `reading_page` por
  página. O índice agrupa **Agente** (Visão geral, Entregas, Testar uma
  tarefa), **Fontes** (Decisões em vigor, Regras, Documentação, com contagem)
  e **Ajustes** (Modo de entrega); o pé mostra o modo salvo ("Ativo · até
  300 tokens") com ponto de estado.
- **Visão geral** sem cartões: linha de estado (ponto de 8 px na cor do
  modo: verde Ativo, azul Medindo, cinza Desligado; frase do que acontece;
  "Mudar modo" ou "Ativar" secundário); "Como chega ao agente" em três
  colunas numeradas em romanos (I–III) divididas por filetes (Fontes com
  contagens, Seleção pelo pedido e pela edição, Entrega com orçamento e sem
  repetição na sessão); "Últimos 7 dias" numa faixa plana entre filetes,
  quatro números em `HEADING_1` (entregas enviadas e medidas,
  sessões, tokens médios contra o limite, itens fora do orçamento), tirados
  da auditoria real; três entregas recentes e "Ver todas".
- **Entregas**: auditoria de cada bloco, do mais recente ao mais antigo, por
  dia; linha com hora em mono, `status_pill` Enviado/Medido, primeiro item
  e "e mais N", contagem e itens fora do orçamento, barra fina de tokens
  contra o limite e sessão curta em mono; a linha abre os itens que o
  agente leu (decisão com versão, regra). Vazio explica o modo e leva a ele.
- **Testar uma tarefa**: o Context Pack para uma tarefa digitada, com barra de
  orçamento, itens e exportação Markdown/JSON; a prévia não é salva.
- **Decisões em vigor**, **Regras** (por tipo, compositor e encerrar
  confirmado na linha; encerrar não apaga) e **Documentação** (ADR-0008:
  grupos por tipo, caminho em mono, "Ler de novo"; alimenta a Visão e não é
  enviada ao agente). Na Visão, fonte de documento usa o ícone de lista e o
  caminho no tooltip.
- **Modo de entrega**: Desligado, Medir e Ativo em `radio_list` +
  `radio_row` (nunca cartões lado a lado) e tokens por bloco; salvar só aparece quando algo muda.
- Rota de captura: `context:<deliveries|test|decisions|rules|documents|mode>`.
- **Revisar conhecimento** fica em Ajustes do Contexto e na paleta, sem nova
  aba. Usa coluna de leitura e rodapé fixo: Verificar localmente, Revisar com
  IA (confirmação inline) e Cancelar. Achados locais, hipóteses consultivas e
  cobertura ficam separados. Fontes mostram campo, versão e trecho; decisões
  citadas abrem em Decisões. Não há ação de aplicar achados. Rota:
  `context:knowledge-review`.
  A rota demo `context:knowledge-review-results` pré-carrega resultados
  sintéticos sem executar IA. Sair da seção ou destino cancela a execução;
  outra só é liberada quando a chamada em curso retorna.

## Visão — o projeto resumido

- Primeira aba do projeto (Ctrl 0, paleta), ADR-0007, numa `reading_page`:
  título "Visão do projeto", linha de meta (data, quantas decisões e regras,
  e em âmbar "N decisões novas desde então") e "Atualizar visão" secundário;
  seção Resumo com parágrafos e, abaixo de cada um, chips mono das fontes
  (decisão abre em Decisões; regra é só rótulo); depois a seção "Arquitetura e
  fluxos": uma frase com o que a página guarda ("8 partes e 3 fluxos") e o botão
  primário "Ver arquitetura e fluxos".
- **Arquitetura e fluxos não se desenham no app.** Um diagrama de arquitetura e a
  leitura de um fluxo passo a passo pedem espaço, zoom e câmera, e a janela do app é
  uma coluna de leitura de 760 px. Eles vivem numa página HTML (próxima seção); o app
  só carrega o resumo e o caminho até ela. O modelo `application::architecture`
  continua sendo derivado do Mapa e dos fluxos, sem IA nova, e alimenta a página.
- Fontes são chips com ícone (documento = decisão, escudo = regra) e o
  título encurtado em palavra inteira; o texto completo fica no tooltip.
  Defasagem é uma linha própria com ponto âmbar, nunca a meta inteira em cor.
- Gerar a Visão também manda documentos novos ou alterados para análise; o
  toast diz quantos ("3 documentos foram para análise; os candidatos aparecem
  na Revisão"). Na Revisão, a evidência aparece como "Documento do projeto"
  com o caminho.
- Vazio: `empty_panel` com o que será enviado ao provedor e "Gerar visão"
  primário; gerando: "Gerando…" desabilitado; erro em `error_banner`; nota de
  procedência no rodapé. Nunca texto sem fonte.

## Texto da IA e página da Visão

**Estilo dos textos.** Todo texto que a IA escreve para uma pessoa (pergunta, escolha e
motivo dos candidatos, Visão, motivos do juiz, regras sugeridas, ligações, revisão
consultiva) segue 80% do ASD-STE100, o inglês técnico controlado: uma ideia por frase,
até 20 palavras, voz ativa, uma palavra para um sentido, resultado antes do motivo. As
travas fazem parte do prompt (`plain_rules!` em `application::plain_style`): manter
todo fato e toda ressalva, não acrescentar fato, não mexer em código, nomes, ids nem
citações, que seguem literais. Texto mais claro convence mais; por isso a evidência
continua vindo antes do motivo. Todo prompt novo que gera texto para pessoas termina
com `crate::plain_rules!()`.

**Página de arquitetura e fluxos.** Na Visão, o botão "Ver arquitetura e fluxos" grava
`arquitetura-<projeto>.html` em `%TEMP%\xemnas` e abre no navegador. É um arquivo único,
sem rede, que abre offline e pode ser enviado a alguém. Só tem arquitetura e fluxos: o
resumo fica no app. O caso de uso `OverviewApi::page` lê a Visão gravada e as decisões
em vigor e regras válidas (`page::assemble`: pergunta, escolha, motivo e sua primeira
frase, premissas, consequências, quando reconsiderar, escopo, critérios de relevância do
candidato, entidades por vínculo confirmado, a parte do mapa de cada uma e as relações
entre decisões); sugestões de vínculo pendentes ficam de fora. `application::page::render`
coloca esse JSON (sem o resumo) num modelo fixo (`page/template.html`); a IA não escreve
HTML, o modelo põe todo texto com `textContent`, e o JSON é escapado para não fechar o
`<script>` nem o `<title>`.

- **Marca e fontes do app, embutidas.** O símbolo é o `xemnas-mark.svg` exato (ids com
  prefixo `xm-`, também como ícone da aba); o nome em Bricolage Grotesque, o texto em
  Inter e os rótulos mono em JetBrains Mono, subconjuntos (`tools/page-fonts.py` gera
  `page/fonts/*.b64`; licenças OFL ao lado das fontes do app).
- **Calma premium.** Fundo de tinta com duas auroras muito suaves e grade de pontos;
  vidro (`--glass`) só nas camadas que flutuam; lavanda só para seleção, foco e a ação
  primária; Bricolage nos títulos, números tabulares, movimento curto com
  `prefers-reduced-motion` respeitado; tema escuro e claro com a mesma paleta do app.
- **Arquitetura.** Mapa em colunas com curvas, um glifo por tipo de parte (banco, IA,
  fila, janela, agente...) deduzido do nome e das tecnologias, só decoração. Clicar
  numa parte abre o inspetor flutuante (o que chama, quem a chama, onde aparece) com
  **Por que é assim**: as decisões em vigor da parte e das partes dentro dela,
  agrupadas pelo primeiro critério de relevância gravado (sem critério, lista simples;
  nenhum tema inventado), pergunta como título e escolha embaixo; clicar abre o motivo,
  premissas, consequências, quando reconsiderar, escopo, o que toca e as relações
  ("substitui", "depende de", conflito na cor de perigo); abaixo, as regras que valem
  ali. Clicar numa seta mostra os passos que a formam. Pan, zoom, "Copiar Mermaid" e "Baixar
  SVG". **Passo a passo** (a referência é o fluxo guiado do IcePanel): escolher um
  fluxo nas pílulas acima liga uma barra no pé com "Passo n de m", título e texto; a
  câmera voa até as duas partes do passo, o resto apaga, a seta ativa corre tracejada
  com uma luz que a percorre; cada decisão citada no passo aparece com a escolha e a
  primeira frase do motivo ("— porque …"). Setas do teclado, "Reproduzir" e Esc.
- **Fluxos.** Diagrama de sequência por fluxo (participantes com glifo, setas
  numeradas, meio em mono) e a linha do tempo dos passos, cada um com o componente, o
  texto e as fontes (decisão em lavanda com a escolha e "porque" e a primeira frase do
  motivo, regra em azul). Passo ativo sincronizado entre
  o diagrama, a linha do tempo e a barra de progresso; "Ver no mapa" abre o mesmo passo
  no passo a passo da arquitetura.
- **Decisões** (`#decisoes`, `#decisoes/<id>`, tecla 3). Vista calma "Por que é assim":
  uma coluna por parte do mapa, com o glifo do mapa, e cada decisão em vigor num cartão
  na parte principal que toca (código, pergunta, escolha, "porque …", "Também em …");
  as que não tocam parte desenhada ficam em "Fora do mapa". Curvas entre cartões, por
  trás deles: depende de (linha), substitui (tracejada), conflito (cor de perigo); ao
  passar ou selecionar, o cartão e suas ligações acendem e o resto apaga. Filtros em
  pílulas: Todas, Só conflitos, uma por parte. O cartão abre a mesma leitura do
  inspetor numa gaveta de vidro fixa à direita; "Ver em Decisões" e as fontes dos
  passos levam a ela.

Para acrescentar uma tela: uma função de montagem em `BUILD` e uma entrada em `TABS` no
modelo; os dados novos entram no JSON sem mudar a Rust além do tipo.

## Revisão — uma fila, evidência primeiro

- **Uma fila para tudo o que espera uma pessoa.** A Revisão mostra os candidatos; quando
  o Mapa tem sugestões (relações entre decisões, contexto sugerido, vínculos), uma barra
  fina com "Decisões propostas | Ligações sugeridas" aparece no topo, com o total. O
  número na aba é a soma. O Mapa deixou de listar Sugestões no índice (três visões:
  Visão geral, Lente de arquivo, Linha do tempo).
- **Desde a última visita:** uma linha discreta no topo da lista (relógio, "há 3 d: 10
  candidatos novos · 5 decisões · 3 entregas ao agente"), que se dispensa. Some quando
  nada mudou. O momento da última visita é guardado por projeto em
  `settings/visits.json` e atualizado quando a janela perde o foco.
- **Marcador por tipo** em cada candidato (decisão em cinza, regra em azul).
- **Evidência antes do motivo:** no detalhe a ordem é pergunta, escolha sugerida,
  evidências, "Motivo escrito pela IA" (recolhido por padrão), No mapa, confiança. Uma
  explicação convincente aumenta a aceitação certa ou errada, então a fonte verificável
  vem primeiro.
- **Desfazer em vez de confirmar:** Rejeitar e Adiar mostram "Desfazer" no aviso.
- **Alcance e ressalvas** (os qualificadores do ADR-0010): logo após a escolha, só quando
  existem. Cada ressalva é o texto em corpo e, abaixo em meta, o tipo e a origem
  ("citado da evidência" ou "escrito na revisão, sem fonte"). Nada de "Não informados".
- **Também apareceu em** (revisão por exceção): só quando a mesma decisão veio de mais de
  uma conversa. Rótulo com a contagem, uma linha dizendo que confirmar ou rejeitar vale
  para todas e chips Ghost "Conversa 1", "Conversa 2" (o selecionado com
  `mark_selected`; o id da captura fica no tooltip). Ler outra conversa troca a evidência
  e mostra uma linha explicando isso. Nunca ids internos nem métricas de oportunidade no
  texto.

## Aprovação automática

É um interruptor como o modo de permissão de um agente: **Manual | Automático**, uma
chave segmentada no topo da lista da Revisão, com uma linha dizendo o que o modo faz
("A IA revisa em lote; o que ela decide fica em Feito sozinho" ou, sem provedor,
"Sem provedor de IA ativo: só as regras locais agem"). Manual é o padrão e não faz nada
sozinho. Ligado, a IA cuida de todo o ciclo: decisões propostas, ligações sugeridas,
regras e vínculos.

Cada item passa primeiro por uma triagem local e gratuita (repete uma decisão já
registrada: descarta; alta confiança, com fonte e sem parecido: aceita **só depois que a
calibração mostrar que a confiança prevê o que você mantém** (ADR-0012), senão pergunta; vínculo ao
Mapa por arquivo ou dependência: aceita; vínculo por menção no texto: pergunta, com o
trecho; o resto pergunta). Só o que sobra vai ao juiz de IA, **em lote** e com limites para não gerar
chamadas: no mínimo 3 itens (ou o mais antigo esperando 2 h), no máximo 12 por chamada,
20 minutos entre chamadas e 6 chamadas por dia. A falha do provedor não perde nada: o
item espera a próxima rodada. O que a IA não resolve fica na fila com o marcador
**"a IA deixou para você"** na linha e a frase do motivo no alto do detalhe.

Aceitar passa pelo mesmo caminho da confirmação manual (prévia e adoção), sem pausa: o
resultado vale na hora. O registro é o ledger **"Feito sozinho · N"**, recolhido sob a
lista; cada linha mostra o veredito, quem decidiu (regras ou IA) e o motivo, abre a
decisão quando foi aceita e oferece **Desfazer** quando foi descartada (volta à fila).
Uma sequência de oito confirmações com poucos segundos entre elas mostra "Ritmo alto:
abra a evidência de um dos próximos antes de confirmar", uma vez, sem bloquear.

## Contexto — quatro entradas

O índice do Contexto tem quatro entradas (Visão geral, Fontes, Entregas, Ajustes), sem
cabeçalhos de grupo; só Entregas leva contagem. As páginas de um grupo se alternam numa
chave segmentada fina no topo da página, e cada entrada abre na página que a pessoa
usou por último naquele grupo (Fontes: Decisões, Regras, Documentação, Revisar com IA;
Entregas: Histórico, Testar uma tarefa). As rotas de captura (`context:rules`,
`context:test`, `context:knowledge-review`) seguem iguais.

## Formas para números

Um número que vale mostrar vale desenhar (`ui::patterns`, canvas de poucos pixels,
sem custo por linha). Sempre com o número ou uma frase ao lado, ou `aria_label` no
pai: a forma não é a única fonte.

- `meter(fração, cor)`: barra de 3 px, uma quantidade contra um limite (tokens do
  bloco contra o orçamento). `meter_stack(partes)`: barra de 6 px dividida por peso,
  do que algo é feito (o que será apagado de um projeto).
- `sparkline(valores, w, h, cor)`: tendência, mais antigo à esquerda, maior no topo,
  área tingida e ponto no último; sem eixos (entregas e sessões dos últimos 7 dias).
- `ring(fração, lado, cor)`: anel que enche no sentido horário a partir do topo
  (média de tokens contra o limite; itens fora do orçamento, âmbar quando há).
- `share(valor, total)` mantém a fração entre 0 e 1 (total vazio dá 0).

## Mapa — o grafo do projeto

- Quarto destino do projeto (Ctrl 4, paleta), ADR-0005. Índice à esquerda
  (296 px, `pane`, filete à direita) com três visões fixas (Sugestões com
  contagem, Lente de arquivo, Linha do tempo) e as listas de Componentes e
  Tecnologias; cada linha mostra o nome e o peso ("3 decisões · 1 regra") e um
  ponto âmbar quando há conflito. Seleção com `mark_selected`; o `+` do
  cabeçalho abre o formulário de item novo.
- Coluna de leitura (`reading_page`) por visão: sugestões com Confirmar e
  Rejeitar por linha e itens propostos com Criar; detalhe da entidade (tipo,
  nome, descrição, padrões em mono, ações Vincular decisão, Vincular regra,
  Faz parte de…, Editar e Aposentar com confirmação na própria página) e as
  seções Decisões em vigor (abrem em Decisões), Conflitos, Regras, Estrutura,
  Impacto e Linha do tempo; lente de arquivo com campo e resultado; antes da
  primeira consulta, "Arquivos das últimas decisões" (até 6, do caso de uso
  `recent_files`) preenche e consulta com um clique.
- **Linha do tempo** (anatomia do Primer Timeline): do mais recente ao mais
  antigo, agrupada por dia ("Hoje", "Ontem", "29 set 2026" + contagem), num
  trilho vertical de 1 px. Decisões e regras são itens cheios: marcador de
  22 px com ícone na cor do estado (verde confirmada, âmbar substituída, azul
  regra), rótulo do tipo em META, título, detalhe e hora à direita. Manutenção
  do mapa (itens criados, ligações) vira uma linha condensada por sequência,
  com ponto vazado, contagem e até 4 nomes + "e mais N". Validade de regra é
  data de calendário: nunca muda com o fuso.
- **Visão geral** (primeira visão do Mapa): blocos no estilo C4, dois por
  linha e de mesma altura na linha, um por componente de topo, com as partes
  dentro como chips; o rodapé do bloco diz o peso ("2 decisões · 1 regra") e
  "mudou em 29 set" quando houve atividade nos últimos 14 dias; ponto âmbar =
  decisões em conflito, com legenda só quando existe. Tecnologias em chips abaixo.
- **Vizinhança** (topo do detalhe): diagrama em camadas, decisões à esquerda
  (abrem em Decisões), o item no centro (única superfície com `selection`) e
  regras à direita, ligados por curvas de 1,25 px desenhadas com `canvas` +
  `PathBuilder` (tracejadas para regras); no máximo 6 por lado e "Mais N nas
  listas abaixo". Layout determinístico, nunca force-directed.
- **Grafo** (alternativa à visão em blocos, seletor `segmented` "Blocos |
  Grafo" no cabeçalho; rota de captura `map:graph`): o mapa inteiro numa tela
  de fósforo. Única superfície com cor própria (`graph.signal` menta,
  `graph.decision` menta clara, `graph.technology` prata), restrita ao
  grafo; âmbar para conflito e sugestão, azul `status.info` para regra.
  - Formas: componente = lente (corpo escuro, aro de 1,25 px, órbita fina e
    núcleo), tamanho pelo número de ligações; tecnologia = hexágono vazado;
    decisão = ponto; regra = quadrado vazado. Rótulos em mono: componentes e
    tecnologias sempre (componente com "2 decisões · 1 regra"), pontos só
    sob o ponteiro ou com zoom alto; os vizinhos ficam no cartão lateral.
  - Linhas: traço fino sobre um traço largo e quase transparente, curva
    suave sempre para o mesmo lado; tracejado para sugestão e regra. Na
    entrada, as linhas crescem do componente para fora e os nós acendem em
    ordem, do centro para fora; um sinal curto percorre cada linha
    devagar e, no item em foco, rápido e brilhante. Sem movimento com
    `reduce_motion`.
  - Decisão ou regra sem nenhuma ligação não é desenhada (fica nas listas);
    componente sem decisões fica esmaecido e com a linha de peso neutra.
  - Layout de forças assentado antes do primeiro quadro, com sementes por id:
    o mesmo mapa abre sempre igual e não "dança"; atualizar mantém os nós
    no lugar. Ilhas (componente de topo, partes, decisões e regras ligadas)
    se repelem para não se sobrepor.
  - Foco: ponteiro ou clique apaga tudo menos a vizinhança; seleção ganha
    retícula de quatro colchetes e um cartão lateral (300 px) com ligações,
    sugestões (Confirmar/Rejeitar) e a ação primária (abrir no Mapa ou em
    Decisões). Camadas em chips no topo, legenda sem caixa embaixo à esquerda, zoom e
    "enquadrar" embaixo à direita; Esc limpa, + − 0 aproximam e enquadram.
- **Sugestões** abrem com **Relações entre decisões**: pergunta da decisão,
  `tag` com o verbo ("depende de", "substitui"; "conflita com" com texto
  âmbar), a outra pergunta, a citação entre aspas com filete à
  esquerda e o motivo em META; Rejeitar (ghost) e Confirmar (secundário).
  Em seguida, **Contexto sugerido**: `tag` com o tipo (Restrição,
  Premissa…), a regra, "Da decisão: … · Vale em …" em META e a citação com
  filete; Rejeitar e Confirmar. Em Contexto › Regras, uma regra cuja decisão
  de origem foi substituída leva a pílula âmbar "Revisar" e a frase do porquê.
  Itens sugeridos mostram a origem ("declarado no workspace Cargo") e a
  descrição do pacote, com "Criar os N do workspace" quando há mais de um;
  Vínculos sugeridos têm "Confirmar os N" no cabeçalho quando há mais de um.
- A **Vizinhança** do detalhe continua determinística em camadas. Nada entra
  no mapa sem confirmação; aposentar e rejeitar não apagam.

## Sugestões — cada cartão se explica

Sugestões (Mapa) tinha linhas curtas que só faziam sentido para quem já sabia o
que era um "vínculo" ou uma "relação". Agora cada seção abre com uma frase que
diz o que ela é e o que confirmar faz, e cada sugestão é um cartão
(`ui::patterns::suggestion_card`, `suggestion_section`, `rich_sentence`):

- **O que é**, numa frase com os nomes em negrito: "A decisão “X” depende da
  decisão “Y” só faz sentido porque a segunda foi tomada"; "A decisão “X”
  mexeu em `arquivo`, que pertence ao componente “Z”"; "Da decisão “X” o xemnas
  tirou uma restrição: …"; "A dependência `serde` foi adicionada em 3
  decisões, mas ainda não é uma tecnologia do Mapa".
- **De onde veio**: o trecho citado ("Trecho que originou a sugestão") e, nas
  relações, o motivo.
- **O que muda ao confirmar**, numa linha com marcador lavanda ("Ao confirmar,
  a decisão passa a valer para Z: aparece na página do componente e é entregue
  ao agente quando ele edita arquivos dele"). Só se descreve o que o
  produto de fato faz; nada de promessa.
- Rejeitar e Confirmar no rodapé do cartão (uma ação por sugestão, nenhuma
  primária concorrendo); "Confirmar os N" fica no cabeçalho da seção de
  vínculos.

## Configurações

- Página do app (engrenagem na barra de título, paleta Ctrl K ou a linha de
  estado da lateral) com navegação à esquerda em linhas simples (glifo de
  16 px e título; sem bloco de ícone nem dica), como Linear e Zed.
- Cada seção abre com título (`HEADING_1`) e uma frase, sem ícone. O estado
  vem logo abaixo como **linha de status**: ponto na cor do estado, título e
  uma frase, sem caixa tingida (`status_hero`). Depois, **painéis** por
  assunto: filete de 1 px, título e propósito no topo, corpo e rodapé com
  dica e a ação principal. As peças ficam em `screens/settings/parts.rs`
  (`status_hero`, `card`, `card_body`, `card_footer`, `stat_tile`, `kv_row`,
  `step`) e só valem para Configurações. Nada de bloco de ícone
  (`icon_tile` saiu): glifo só de 16 px, inline, em cor atenuada.
- Opções exclusivas são **linhas de rádio** numa lista com filetes (glifo,
  título, descrição, marca de rádio à direita; selecionada com
  `mark_selected`), nunca cartões lado a lado. Números em destaque ficam
  planos (`stat_tile`) dentro de um painel único. Requisitos antes do
  consentimento são uma lista de verificação (círculo vazio → check verde).
- Extrator em lista de rádio: Local, Modelo local ou API, Conta ChatGPT,
  OpenCode Zen ou Go e Claude Code (experimental) (ADR-0004); no OpenCode o plano (Zen ou Go) é um par de
  opções com `mark_selected` e a chave tem o atalho **Criar uma chave**. Cada
  provedor mostra só os campos que usa; trocar de provedor não leva endereço
  nem modelo de um para outro.
- **Seletor de modelo:** campo de texto sempre editável, com **Listar modelos**
  ao lado. A lista aparece abaixo como grupo de opções (`mark_selected`,
  rolagem própria) e clicar preenche o campo; carregando é esqueleto, falha e
  lista vazia dizem que o modelo ainda pode ser digitado.
- **Conta ChatGPT** segue as diretrizes do Sign in with ChatGPT: ação primária
  **Continuar com o ChatGPT**; enquanto o navegador está aberto, o cartão diz
  isso e oferece Cancelar e Abrir de novo; conectado, mostra o e-mail, o selo
  **Usando o plano do ChatGPT**, **Gerenciar uso** (ícone de seta para fora,
  abre o navegador) e Sair da conta. No primeiro login aparece uma vez o aviso
  "Você está usando o seu plano do ChatGPT" com **Entendi**.
- **Claude Code** é um painel de estado, sem login próprio, porque o login é
  do Claude Code: verificando é esqueleto; sem o CLI, selo **Não encontrado** e
  como instalar; sem login, selo **Sem login** e o comando `claude auth login`
  em monoespaçada com **Copiar comando**; conectado, e-mail, plano, método e
  versão com o selo **Conectado** e a dica de que é experimental e conta no uso
  do plano. **Verificar de novo** fica no rodapé, e o painel não tem ação
  primária: a de ativar é a do consentimento. Escolher o Claude Code preenche
  `sonnet`, o mais rápido e barato por extração (ADR-0004). Rota de captura: `settings:claude-code` (o
  demo simula os outros estados com `XEMNAS_DEMO_CLAUDE=logged-out|missing`).
- Credencial do provedor (chave de API ou chave do OpenCode) é um painel com
  selo de estado ("Guardada no cofre", "Opcional neste endereço", "Nenhuma chave")
  e o campo secreto; o valor guardado nunca volta para a tela.
- OpenCode mostra o teste de conexão como lista de verificações com resultado
  (OK, Atenção, Falha, Não se aplica) e as mensagens do backend; Diagnóstico
  mostra medianas, perdas, tarefas com reprocessar/cancelar e exporta o JSON
  sanitizado.
- Painel do projeto: **Apagar dados…** mede o impacto real e só libera Apagar
  tudo depois de digitar o nome do projeto; a pasta no disco não é tocada.

## Desempenho

O pilar do projeto é desempenho e baixo custo; as regras e orçamentos estão
em `docs/arquitetura/desempenho-e-escala.md`. No visual, isso vira:

- **Listas longas são virtuais.** O índice do Mapa e o índice de Decisões
  usam a lista do GPUI (`list` + `ListState`): só as linhas em vista viram
  elementos, então rolar custa o mesmo com 10 ou 10 mil itens. As linhas são
  chaves baratas (`IndexRow`, `IndexItem`) e cada elemento é montado quando
  entra na tela. Quando o formato muda, `splice` refaz a medição.
- **Barra fina que some** (`ui::list::{ScrollMemory, scroll_thumb}`): 3 px,
  `text_muted` a 55 %, aparece ao rolar, fica 650 ms e some em 350 ms; não é
  controle, só mostra posição e tamanho. Só pede quadro enquanto visível.
- **Rodapé de revelação** (`ui::list::reveal_footer`) para listas que crescem
  sob demanda e não são virtuais (linha do tempo, Sugestões, blocos de um
  item, lente de arquivo, regras): fio, "N de M" em algarismos tabulares, linha
  de progresso de 2 px em lavanda, "Mostrar mais N" (duas páginas) e, se o que
  resta é no máximo 200, "Mostrar todas (N)". Páginas: `LIST_PAGE` = 12,
  `TIMELINE_PAGE` = 30, `RULES_PAGE` = 10 por tipo.
- **Grafo em escala.** Um componente com mais decisões ou regras do que o
  desenho comporta (orçamento de 260 folhas, entre 2 e 10 por componente)
  mantém uma parte e dobra o resto num nó de grupo ("+590 decisões", anel
  com núcleo, maior quanto mais guarda) que abre a página do componente; as
  contagens seguem exatas. A pintura visita só o que está na viewport (com
  margem de 48 px) e, com mais de 140 links à vista, usa o modo barato:
  links retos de um traço, sem sinal correndo, nós sem halos e rótulos de
  componente só a partir de 0,9 de zoom.
- **Panorama.** Em mapas com mais de 150 nós, abaixo de zoom 0,45 só ficam
  componentes, tecnologias e grupos; decisões e regras individuais voltam ao
  aproximar (histerese de 0,1), exceto as com conflito ou sugestão, que ficam
  sempre. O cabeçalho avisa: "Panorama · N decisões e regras ocultas ·
  aproxime para vê-las". A dica de interação e os filtros dividem uma linha
  que quebra: em janela estreita a dica desce, nunca cobre os filtros.
- **Blocos virtuais.** O layout em Blocos é uma lista virtual de linhas (dois
  blocos por linha, quatro tecnologias por linha) na mesma coluna de 760 px;
  um bloco lista até 6 partes e conta o resto ("+N partes"), e a página do
  componente lista todas.
- O que pesa vai para fora da thread da interface: o layout de forças do
  grafo roda em segundo plano, só é feito quando o Grafo é aberto (na visão
  em Blocos espera), é aplicado se ainda for o mais recente e, em mapas com
  mais de 200 nós, usa menos passos. A simulação limita força e velocidade
  para um componente com centenas de decisões não explodir em posições
  infinitas.
- Nada de copiar listas dentro do `render`: o Mapa guarda sugestões, detalhe,
  linha do tempo e lente em `Arc` e o render pega um ponteiro.
- Telas pesadas ficam em views com cache (`cached`) e o mascote é view
  própria: um quadro dele não refaz o app.
- Medir antes de mexer: `XEMNAS_PERF=1` grava em `xemnas-perf.log` (pasta
  temporária) o tempo de montagem de cada tela (`ui::perf::Probe`), e
  `XEMNAS_DEMO_SCALE=N` semeia N decisões, N/4 componentes e N/3 regras, a
  maior parte num componente "docs". A janela fora da tela tem ritmo próprio
  da plataforma (~36 ms entre quadros): o número útil é o tempo de montagem.

## Movimento

- Tudo vem de `ui::motion`: curvas cúbicas exatas (`curve`: entrada
  `cubic-bezier(0.16, 1, 0.3, 1)`, saída `(0.4, 0, 1, 1)`) e o catálogo
  (`spec`). Tela nenhuma escreve duração ou curva.
- Conteúdo substituído (destino, projeto, item): `content_in`, 220 ms, fade
  com subida de 4 px. Menus e popovers: entram em 140 ms vindos do lado do
  gatilho a partir de 30% de opacidade e saem em 100 ms (`menu_in`,
  `menu_out`). Painel flutuante maior: `panel_in`, 200 ms, subida de 8 px.
  Sem escala: `div` não tem transform; o deslocamento usa `relative().top()`
  para não mover os vizinhos.
- Movimento contínuo (o sinal do grafo) nunca usa
  `with_animation` em repetição: pede a fase a `motion::clock` (30 ou 15 Hz,
  com lease de 300 ms), que estaciona quando nada visível pede. Movimento
  reduzido devolve fase parada. Movimento grande e lento a 15 Hz anda aos
  degraus: para o mascote, respirações curtas na taxa cheia, com repouso
  entre elas, em vez de um flutuar contínuo.
- Popovers (paleta, painel do projeto, painel do assistente) guardam o
  estado em `ui::popup::Popup`: aberto, saindo, fechado. Ao fechar continuam
  desenhados, sem aceitar clique, enquanto saem; `reap` os descarta depois, e
  reabrir no meio da saída vale. O gatilho anota no mouse-down se o popover
  estava montado, então o clique que o dispensou não o reabre (sem guard de
  tempo).
- A seleção das abas do projeto é uma placa só atrás da fileira que desliza
  até a aba escolhida (`motion::glide::SlideIndicator`, 180 ms): parte de
  onde está, mesmo no meio de outro deslize; a primeira colocação, um resize
  e o movimento reduzido chegam sem viagem.
- Listas que carregam entram em cascata (`motion::cascade`): cada linha
  35 ms depois da anterior, só as 6 primeiras esperam, fade com subida de
  6 px, uma vez por linha (chave pelo id). Na Revisão e na linha do tempo.
- Números de destaque contam até o valor (`count_up`) e todo contador usa
  algarismos de largura fixa (`tabular`, `tnum`), para nada pular quando o
  número muda.
- Modo foco (Ctrl \, paleta): a lateral se recolhe em 220 ms; a largura de
  fora desliza e a lateral de dentro mantém a largura, sem refluxo.
- Abertura: o mascote e o wordmark sobre o fundo do tema por 650 ms, o
  final num fade que sobe 8 px; sem isso com movimento reduzido.
- Hover de linhas e abas segue a mola de `hover_tint`.

## Interação

- **Elevação** em três níveis (`material::elevation`): dica (tooltip,
  toast), flutuante (menus, painel do projeto) e diálogo (paleta,
  assistente); cada um com sombra larga, sombra de contato e um realce
  claro na borda de cima. Nenhuma superfície flutuante define sombra própria.
- Estados vazios sem nada a fazer mostram o mascote de olhos fechados
  (Revisão); os que esperam o primeiro passo, de olhos acesos (Decisões)
  (`empty_panel_mascot`).
- Com fundo de imagem, um grão finíssimo (tile de 256 px) fica sobre a
  imagem e sob as superfícies: aspecto fosco, nunca sobre o texto.
- Botões são **placas** (`ui::material`): gradiente vertical iluminado de
  cima, borda fina, realce de 1 px na borda de cima e sombra curta embaixo.
  O secundário é a placa neutra (branco translúcido); o primário é a placa
  lavanda com halo que se abre no hover. Sem blur: o vidro é só luz e borda.
- Botões respondem em três tempos: repouso, hover (preenchimento e borda sobem;
  o primário ganha halo lavanda) e clique (um passo mais fundo). Desabilitado
  não reage. Linhas de lista e abas usam a mola de `hover_tint`.

## Gate de entrega visual

Compilar e testar não bastam: capturar a janela **GPUI do binário recém-gerado**
com projeto selecionado e sem projetos, em tamanho padrão e reduzido. Conferir
truncamento de nome e caminho, seleção sob hover, foco, troca de projeto,
confirmação/cancelamento e contraste. Se o executável estiver bloqueado pela
política do Windows, registrar o limite em vez de validar com uma janela antiga
ou enfraquecer as proteções do sistema.

Sem tomar o foco da máquina: `tools/capture-background.ps1 -Route <rota>`
abre o `--demo` fora da tela, sem foco, chega à tela por `--open` (ex.:
`overview:flow0`, `map:timeline`, `map:entity:storage-sqlite`) e copia a
janela com PrintWindow; `-Theme charcoal` troca a paleta. Nenhum clique ou
tecla é enviado.

Para a Inbox, conferir também lista vazia, filtro sem resultados, paginação,
troca de fonte e leitura de evidências extensas. Capturas e decisões só ganham
superfícies quando ligadas a dados e ações reais no produto.
