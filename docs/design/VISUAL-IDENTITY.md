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
  glow, blur ou animação para mascarar a falta de informação. A única animação é
  um fade de 160 ms quando o conteúdo é substituído (destino, projeto, item).
- Densidade de ferramenta: interface em 13 px, leitura em 14 px, títulos de
  painel em 12 px esmaecidos. Inter em toda a interface; Bricolage Grotesque só
  no wordmark e no título de leitura (a pergunta). Nomes de família vêm de
  `resolve_font_families` (o Windows registra "Inter Variable Text" e
  "Bricolage Grotesque 14pt") e pesos usam instâncias nomeadas (400/500/600).
  Títulos são compostos por palavra para a pontuação não quebrar sozinha. Controles têm 28 ou 32 px (`ControlSize`), raio de
  6 px, e vêm de `action_button`/`icon_action`; telas não desenham botões próprios.
- Uma receita de seleção para toda lista: fundo `selection` e barra lavanda de
  2 px (`mark_selected`). Foco visível é um anel inset que não desloca o layout.
- Cada paleta (Quiet Glass e Carvão) é uma tabela `Palette` em `tokens.rs`;
  toda cor nova precisa existir nas duas.
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
  `Gauge`; testar uma tarefa = `Flask`; objetivo = `Flag`; escopo = `Target`.
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
- Em Decisões não há segunda fileira de abas: estado, versão, o acesso ao
  histórico ("Versões") e as ações Exportar/Revisar ficam na linha de metadados
  do documento. Linhas de lista e abas têm hover com mola criticamente amortecida.
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
- **Visão geral**: cartão de estado (ponto com halo na cor do modo: verde
  Ativo, azul Medindo, cinza Desligado; frase do que acontece; "Mudar modo"
  ou "Ativar" secundário); "Como chega ao agente" em três etapas numeradas
  ligadas por chevrons (Fontes com contagens, Seleção pelo pedido e pela
  edição, Entrega com orçamento e sem repetição na sessão); "Últimos 7 dias"
  em quatro blocos com número em `DISPLAY` (entregas enviadas e medidas,
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
- **Modo de entrega**: Desligado, Medir e Ativo lado a lado e tokens por
  bloco; salvar só aparece quando algo muda.
- Rota de captura: `context:<deliveries|test|decisions|rules|documents|mode>`.

## Visão — o projeto resumido

- Primeira aba do projeto (Ctrl 0, paleta), ADR-0007, numa `reading_page`:
  título "Visão do projeto", linha de meta (data, quantas decisões e regras,
  e em âmbar "N decisões novas desde então") e "Atualizar visão" secundário;
  seção Resumo com parágrafos e, abaixo de cada um, chips mono das fontes
  (decisão abre em Decisões; regra é só rótulo); seção Principais fluxos com
  cartões de vidro dois por linha (título, descrição, passos e componentes).
- O cartão abre o fluxo na mesma coluna: "Todos os fluxos" (ghost) volta;
  passos numerados em círculos de 24 px ligados por filete vertical, com
  título, texto, chip do componente (ícone de grafo, abre no Mapa) e fontes.
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
  Impacto e Linha do tempo; lente de arquivo com campo e resultado.
- **Linha do tempo** (anatomia do Primer Timeline): do mais recente ao mais
  antigo, agrupada por dia ("Hoje", "Ontem", "29 set 2026" + contagem), num
  trilho vertical de 1 px. Decisões e regras são itens cheios: marcador de
  22 px com ícone na cor do estado (verde confirmada, âmbar substituída, azul
  regra), rótulo do tipo em META, título, detalhe e hora à direita. Manutenção
  do mapa (itens criados, ligações) vira uma linha condensada por sequência,
  com ponto vazado, contagem e até 4 nomes + "e mais N". Validade de regra é
  data de calendário: nunca muda com o fuso.
- **Visão geral** (primeira visão do Mapa): blocos no estilo C4, dois por
  linha, um por componente de topo, com as partes dentro como chips; cada
  decisão em vigor é um quadrado cheio de 8 px e cada regra um vazado, e o
  bloco cresce com as decisões; ponto verde = atividade nos últimos 14 dias,
  âmbar = decisões em conflito, com legenda. Tecnologias em chips abaixo.
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
  - Layout de forças assentado antes do primeiro quadro, com sementes por id:
    o mesmo mapa abre sempre igual e não "dança"; atualizar mantém os nós
    no lugar. Ilhas (componente de topo, partes, decisões e regras ligadas)
    se repelem para não se sobrepor.
  - Foco: ponteiro ou clique apaga tudo menos a vizinhança; seleção ganha
    retícula de quatro colchetes e um cartão lateral (300 px) com ligações,
    sugestões (Confirmar/Rejeitar) e a ação primária (abrir no Mapa ou em
    Decisões). Camadas em chips no topo, legenda embaixo à esquerda, zoom e
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
  descrição do pacote, com "Criar os N do workspace" quando há mais de um.
- A **Vizinhança** do detalhe continua determinística em camadas. Nada entra
  no mapa sem confirmação; aposentar e rejeitar não apagam.

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
- Extrator em lista de rádio: Local, Modelo local ou API, Conta ChatGPT e
  OpenCode Zen ou Go (ADR-0004); no OpenCode o plano (Zen ou Go) é um par de
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
- Credencial do provedor (chave de API ou chave do OpenCode) é um painel com
  selo de estado ("Guardada no cofre", "Opcional neste endereço", "Nenhuma chave")
  e o campo secreto; o valor guardado nunca volta para a tela.
- OpenCode mostra o teste de conexão como lista de verificações com resultado
  (OK, Atenção, Falha, Não se aplica) e as mensagens do backend; Diagnóstico
  mostra medianas, perdas, tarefas com reprocessar/cancelar e exporta o JSON
  sanitizado.
- Painel do projeto: **Apagar dados…** mede o impacto real e só libera Apagar
  tudo depois de digitar o nome do projeto; a pasta no disco não é tocada.

## Interação

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
