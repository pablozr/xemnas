# Glass na janela inteira: pesquisa e direção

Pesquisa de 30/09/2026, depois do primeiro teste com Acrylic na barra de título e na lateral. Pergunta: como estender o material ao app todo sem perder legibilidade nem cair no "vidro em tudo".

## O que as referências fazem

**Windows 11 / Fluent (a plataforma do app).** O material é a **camada base da janela inteira**, e o conteúdo fica numa **camada de conteúdo de baixa opacidade** por cima (`LayerFillColorDefaultBrush`), que deixa o material aparecer de leve. Há dois padrões: o **contínuo** (uma área única de conteúdo) e o de **cards** (conteúdo segmentado). Com Mica Alt, entra um terceiro nível: a camada de comando (navegação, barra de título) fica entre a base e o conteúdo. O material deve aparecer na barra de título. Aplicar o material mais de uma vez ou em um elemento isolado não funciona. [Mica](https://learn.microsoft.com/en-us/windows/apps/design/style/mica), [Layering](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/layering).

**Acrylic** é para superfícies transitórias (menus, flyouts, popups). A documentação desaconselha acrílico de fundo em superfícies grandes, painéis de acrílico lado a lado (criam uma emenda visível) e texto na cor de destaque sobre acrílico. O acrílico também custa GPU e bateria, e vira cor sólida em Economia de bateria ou quando a transparência está desligada. [Acrylic](https://learn.microsoft.com/en-us/windows/apps/design/style/acrylic).

**Fluent 2** separa Mica (base opaca e tingida), Acrylic (vidro fosco para superfícies temporárias) e Smoke (véu escuro que destaca um modal). [Fluent 2 Material](https://fluent2.microsoft.design/material).

**Arc.** A navegação fica sobre o material, e o conteúdo vive num **card flutuante** com margens, cantos contínuos de ~10 pt, borda de 1 px e sombra suave. A margem do lado encostado na lateral colapsa. [Guia de design do Arc](https://blakecrosley.com/guides/design/arc), [implementação do layout em card](https://github.com/driceroland/Search/pull/143).

**Zen.** Mesma ideia (lateral e moldura sobre o material, conteúdo arredondado), com modo compacto em que a lateral flutua sobre o conteúdo. [Zen, modo compacto](https://docs.zen-browser.app/user-manual/compact-mode).

## Restrições do Xemnas

- O GPUI não tem blur por elemento: só o material da janela (Mica, Mica Alt, Acrylic). "Vidro" dentro do app é translucidez sobre esse material, nunca desfoque próprio.
- O material só funciona bem no modo escuro do sistema com a paleta escura. No modo claro, a moldura continua opaca.
- A leitura é longa (evidência, código, decisões). O conteúdo precisa de contraste estável, independente do papel de parede.

## Direção escolhida: moldura de vidro, conteúdo em card

1. **Base.** Mica Alt na janela inteira (Acrylic é opcional via `XEMNAS_BACKDROP`).
2. **Camada de comando.** Barra de título e lateral translúcidas (`color.chrome`), sem bordas entre elas: são uma única moldura de vidro.
3. **Camada de conteúdo.** A área do projeto vira um card flutuante com 8 px de margem à direita e embaixo, encostado na lateral, raio de 10 px, borda de 1 px e sombra de card. O fundo é `color.canvas` a ~88% de opacidade, o equivalente ao `LayerFillColorDefault`: o material aparece como tom, não como imagem.
4. **Painéis internos** (fila da Revisão, índice de Decisões) ficam translúcidos dentro do card, em vez de blocos opacos, com fios de 1 px separando.
5. **Objetos de leitura** (a moldura de evidência, o poço de código) continuam sólidos. É o padrão de cards do Fluent aplicado só onde há leitura densa.
6. **Superfícies transitórias** (paleta, painel do projeto, toast, tooltip) ficam quase opacas, com borda e sombra de elevação. A paleta usa Smoke (véu escuro) por trás.
7. **Janela inativa ou transparência desligada.** O próprio Windows troca o material por cor sólida; as camadas translúcidas continuam legíveis sobre ela.

Evitado de propósito: acrílico em cada painel (emendas e ruído), texto lavanda sobre vidro e vidro atrás de código.
