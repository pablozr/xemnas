# Xemnas — Quiet Glass

Este é o padrão visual do aplicativo desktop GPUI. Mudanças de identidade precisam
ser explícitas; não inventar telas, dados ou ações para preencher espaço. A
referência extensa em `docs/design-system-quiet-glass.md` pode existir localmente,
mas `docs/` é ignorado pelo Git: este arquivo é a regra versionável.

## Hierarquia e material

- Base carvão escura, texto de alto contraste e lavanda mineral discreta. Cores,
  tipografia e espaçamento vêm de `apps/desktop-gpui/src/ui/tokens.rs` e `theme.rs`;
  não espalhar literais de cor pelas telas.
- Superfícies grandes são quietas. Reservar lavanda e contornos para seleção,
  cabeçalho do projeto, foco e uma ação primária real. Não usar card, glow,
  blur ou animação para mascarar a falta de informação.
- Mica pode compor o fundo da janela no Windows; isso **não** garante backdrop
  blur por componente. Superfícies internas precisam funcionar sem blur.
- Cada informação tem um lugar: contagem na lista lateral, título da seção na
  própria seção; não repetir a mesma informação no título e no rodapé.

## Projetos — superfície atual

- Lista lateral de 322 px e detalhe selecionado ocupando toda a área restante.
  Cabeçalho contínuo, propriedades reais (localização e data) logo abaixo dele e
  ação de remoção secundária junto às propriedades, dentro da primeira área visível.
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

## Gate de entrega visual

Compilar e testar não bastam: capturar a janela **GPUI do binário recém-gerado**
com projeto selecionado e sem projetos, em tamanho padrão e reduzido. Conferir
truncamento de nome e caminho, seleção sob hover, foco, troca de projeto,
confirmação/cancelamento e contraste. Se o executável estiver bloqueado pela
política do Windows, registrar o limite em vez de validar com uma janela antiga
ou enfraquecer as proteções do sistema.

Inbox, capturas e decisões são superfícies futuras: só integrar ao detalhe
quando houver dados e ações reais no produto.
