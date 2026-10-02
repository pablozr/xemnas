# Acabamento visual: revisão e referências

Data: 2026-10-02.

**Pergunta:** o que falta para o app ter acabamento impecável sem perder a
calma, e o que as referências de produto sugerem para o nosso contexto?

**Status:** Aberta. Revisão feita sobre capturas da release (Visão, Revisão,
Decisões, Contexto, Mapa em Blocos e Grafo, página de componente, janela
compacta); melhorias de lista e escala entregues, o restante é fila.

## Referências consultadas

- Resumos de [design-engineering de Emil Kowalski](https://www.ui-skills.com/playbook/use-concentric-border-radius)
  e de guias de "design taste" (confirmar nas fontes originais; a página da
  skill devolveu 403): raio concêntrico (raio externo = raio interno +
  espaçamento), algarismos tabulares em datas e contagens, resposta ao toque
  com `scale(0.96)`, transições de ~150 ms com ease-out, animações
  interrompíveis, e **não animar ações iniciadas pelo teclado** (repetidas
  centenas de vezes por dia, a animação as faz parecer lentas).
- [Raycast, análise do design system](https://aiskill.market/blog/inside-raycasts-design-system):
  fundos escuros frios, camadas sutis em vez de bordas brilhantes, acento
  reservado a seleção e estado, monoespaçada como marca de ferramenta de
  desenvolvedor, linhas de altura uniforme e raios contidos. "Premium por
  disciplina, não por enfeite."
- [Princípios de design do Windows](https://learn.microsoft.com/en-us/windows/apps/design/design-principles):
  o Windows 11 se apaga em segundo plano para manter o foco; movimento é
  reativo, direto e contextual.
- Zeron (`zeronsh/zeron`): o README não traz documento de design nem capturas
  além de uma; a referência dele é de desempenho, não de acabamento (ver
  `gpui-zeron-e-fork.md`). Para comparar o visual de verdade falta material
  (capturas de tela dele).

## O que a revisão encontrou

Já alinhado com as referências:

- Algarismos tabulares em contagens (`tabular`, `count_chip`, `count_up`).
- Acento lavanda só em seleção, foco e ação primária; mono em atalhos, caminhos
  e rótulos técnicos; hairlines em vez de bordas fortes; camadas de material.
- Estados de passar o mouse e pressionar nos controles; foco visível.
- Movimento com redução respeitada e sem redesenho em repouso.

Ajustado nesta rodada:

- Linhas do índice de Decisões não ocupavam a largura toda na lista virtual (a
  seleção parava antes do rótulo de versão).
- Blocos do Mapa montavam todos os componentes (259 em escala 1000) e
  varriam todas as entidades por bloco a cada passo de rolagem; agora vêm em
  páginas de 24 com rodapé de revelação e um índice de partes feito uma vez.
- A linha de progresso do rodapé de revelação sumia em listas muito longas;
  ganhou largura mínima.

## Fila (por impacto)

1. Resposta ao toque (mais escuro ao pressionar já existe; falta uma escala
   sutil onde o GPUI permitir) e revisão de raios concêntricos em cartões com
   chips dentro (raios fixos de 4 px dentro de cartões de 8 px).
2. Não animar entrada de linhas quando a navegação veio do teclado (a cascata
   da Revisão e do índice deve valer só para carga, não para filtrar ou
   navegar).
3. Página de componente: abre ligeiramente rolada em capturas (o título perde
   ~25 px de respiro); verificar se a rolagem da página anterior é herdada.
4. Legenda do grafo se sobrepõe a nós em mapas grandes; tirar do fluxo do
   desenho ou dar-lhe um fundo.
5. Capturas comparáveis com o Zeron e com Linear/Raycast para uma lista de
   diferenças tela a tela (espaçamento, hierarquia, densidade).
