# Xemnas — Quiet Glass

Este é o padrão visual do aplicativo desktop GPUI. Mudanças de identidade precisam
ser explícitas; não inventar telas, dados ou ações para preencher espaço. A
referência extensa em `docs/design-system-quiet-glass.md` pode existir localmente,
mas `docs/` é ignorado pelo Git: este arquivo é a regra versionável.

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
  elemento. Pesquisa: `docs/design-review/glass-research.md`.
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
  Estados têm badge com texto e bolinha: Pendente, Adiado, Confirmado e Rejeitado.
- Sem candidatos, orientar sobre captura e extração. Falhas mostram recuperação
  em linguagem de produto; conteúdo técnico do erro não aparece na interface.
- Confirmar, Rejeitar, Adiar/Retomar e Ajustar usam os casos de uso da Inbox.
  Ações ficam visíveis fora da rolagem de leitura, bloqueiam envios duplicados
  durante a operação e atualizam fila e contagem. Ajustes só persistem ao salvar;
  cancelar não escreve. Salvar e confirmar cria a decisão com os ajustes.
- `xemnas --demo` abre uma prévia identificada com dados fictícios em memória,
  sem iniciar integrações, workers ou alterar o banco normal.
  `--demo --long-evidence` permite verificar fontes com 1.500 linhas.

## Gate de entrega visual

Compilar e testar não bastam: capturar a janela **GPUI do binário recém-gerado**
com projeto selecionado e sem projetos, em tamanho padrão e reduzido. Conferir
truncamento de nome e caminho, seleção sob hover, foco, troca de projeto,
confirmação/cancelamento e contraste. Se o executável estiver bloqueado pela
política do Windows, registrar o limite em vez de validar com uma janela antiga
ou enfraquecer as proteções do sistema.

Para a Inbox, conferir também lista vazia, filtro sem resultados, paginação,
troca de fonte e leitura de evidências extensas. Capturas e decisões só ganham
superfícies quando ligadas a dados e ações reais no produto.
