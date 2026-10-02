# Listas e grafos em escala

Data: 2026-10-02.

**Pergunta:** como o app continua suave quando o projeto cresce (milhares de
decisões, centenas de componentes, mil nós no grafo) sem exibir tudo de uma vez?

**Status:** Em implementação. Listas virtuais do Mapa e de Decisões, rodapé de
revelação, agrupamento e corte no grafo entregues; carga paginada no backend e
as demais telas abertas.

## O que a pesquisa apontou

- **Zeron** (`zeronsh/zeron`, MIT; docs `whale-ui-responsiveness`,
  `mugen-pretext`, `performance-*`): lista virtual do próprio GPUI com alturas
  medidas, preparação das linhas no executor de fundo com compartilhamento por
  `Arc`, relógio de pulso compartilhado a 30/15 Hz, consultas só sob demanda e
  opções de lista persistentes. Aplicável ao nosso GPUI oficial: tudo menos o
  que depende do fork (confirmar ao reler os docs deles).
- **GPUI**: `uniform_list` (altura única) e `list` + `ListState` (altura
  variável, medida; `splice` para mudar o formato; deslocamento e tamanho
  disponíveis para uma barra própria). Rolar notifica a view dona, então a view
  não pode carregar cópias das coleções.
- **Grafos grandes** (prática comum, confirmar fontes): agregar em
  supernós expansíveis, nível de detalhe por zoom, corte por viewport e layout
  incremental; desenhar mil nós com halos e curvas é o que pesa, não a
  simulação.
- **Paginação**: keyset (cursor pela última chave) em vez de `OFFSET`, que
  relê o que pulou; o índice de Decisões já usa cursor.
- **Acessibilidade de lista virtual**: o leitor de tela só vê as linhas
  montadas; a contagem total precisa estar num rótulo (o chip de contagem do
  cabeçalho cumpre).

## Entregue

Ver `docs/arquitetura/desempenho-e-escala.md` e a seção Desempenho de
`docs/design/VISUAL-IDENTITY.md`.

## Em aberto

- Página de componente com "Ver todas" por seção (decisões, regras, impacto,
  linha do tempo) com filtro: hoje o nó de grupo abre a página, que ainda
  pagina por rodapé.
- Linha do tempo global como lista virtual com cabeçalhos por dia.
- Revisão (`inbox`) e Contexto como listas virtuais; a Revisão já pagina por
  demanda e anima a entrada.
- Camada de dados com janela de tempo e cursor também para o Mapa (o backend
  entrega o mapa inteiro de uma vez); pedir à sessão de backend.
- Grafo: expandir um grupo no lugar, layout incremental local e nível de
  detalhe por zoom nos rótulos de decisões.
- Medir em escala 1000 e 5000 com a release e registrar os números aqui.
