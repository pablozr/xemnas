# Desempenho e escala

O pilar do xemnas é **desempenho e baixo custo**: CPU em repouso, memória,
chamadas e tokens de IA, dependências. Entre duas soluções equivalentes, vale a
mais barata. Este documento são as regras que decorrem disso; o que está
pendente fica em `docs/pesquisas/listas-e-grafos-em-escala.md`.

## Regras

1. **Montar só o que está na tela.** Lista com mais de algumas dezenas de
   linhas é virtual (`gpui::list`); o que não é virtual mostra uma página e
   revela mais por demanda. Exibir tudo de uma vez nunca é a opção.
2. **Resumir antes de listar.** Um número ("+590 decisões"), um grupo ou uma
   página de resumo vale mais que mil linhas. Quem precisa do detalhe abre o
   item e lê ali.
3. **Carregar por demanda.** Dados em páginas (keyset, não `OFFSET`) e só
   quando o recurso é pedido; consulta cara só quando a tela aparece.
4. **Trabalho pesado fora da thread da interface.** Layout, busca, preparação
   de linhas e leitura de disco rodam no executor de fundo; a interface só
   instala o resultado, se ainda for o mais recente.
5. **Nada de copiar coleções no `render`.** Cada passo de rolagem refaz a view
   dona dela. Estado compartilhado fica em `Arc`; o render pega um ponteiro.
6. **Movimento custa quadros.** Só se pede o próximo quadro enquanto algo se
   move; repouso é zero redesenho. Loops ambientes usam o relógio compartilhado
   de 30 Hz, que estaciona quando ninguém o consome.
7. **Medir.** `XEMNAS_PERF=1` (tempo de montagem de cada tela em
   `%TEMP%\xemnas-perf.log`) e `XEMNAS_DEMO_SCALE=N` (semeia N decisões, N/4
   componentes, N/3 regras). Dizer o número e dizer o que não foi medido.

## Orçamentos

| Coisa | Valor | Onde |
| --- | --- | --- |
| Página de listas no Mapa | 12 linhas; linha do tempo 30; regras 10 por tipo | `LIST_PAGE`, `TIMELINE_PAGE`, `RULES_PAGE` |
| "Mostrar todas" | até 200 itens | `MOST_AT_ONCE` |
| Folhas do grafo | 260 no total, 2 a 10 por componente | `LEAF_BUDGET`, `MIN_LEAVES`, `MAX_LEAVES` |
| Modo barato do grafo | mais de 140 links à vista | `CROWDED_LINKS` |
| Panorama do grafo | zoom abaixo de 0,45 (volta acima de 0,55) em mapas com mais de 150 nós | `PANORAMA_BELOW`, `PANORAMA_BAND`, `PANORAMA_FROM` |
| Solver do layout | laço exato até 300 nós; Barnes–Hut acima | `EXACT_UP_TO`, `solver_for` |
| Margem de corte do grafo | 48 px além da viewport | `CULL_MARGIN` |
| Layout completo do grafo | até 200 nós; acima disso, menos passos | `FULL_STEPS_UP_TO` |
| Mascote | 32 quadros de 160 px, ~4 MB de textura, decodificados na primeira vez | `screens/mascot.rs` |

## Padrões em código

- Lista virtual: `ListState` na view, `list(state, cx.processor(...))`, linhas
  como chaves baratas e `splice` quando o formato muda (`screens/map.rs`,
  `screens/decisions.rs`); barra e rodapé em `ui/list.rs`.
- Medição: `layout_benchmark` (layout, ignorado por padrão) e `XEMNAS_GRAPH_RENDER=full|cull|cheap|lod` (pintura); números em `docs/pesquisas/escalabilidade-renderizacao-fontes.md`.
- Grafo: `fold_crowds` agrupa multidões, `Scene::on_screen` e
  `links_on_screen` cortam o que está fora, `crowded` liga o modo barato.
- Layout do grafo em segundo plano com geração para descartar resultado velho
  (`GraphCanvas::set_graph`).

Telas novas seguem estas regras antes de seguir o padrão visual
(`docs/design/VISUAL-IDENTITY.md`).
