# Acabamento visual com GPUI: o que trazer do Zeron e se vale um fork

**Data:** 01/10/2026
**Pergunta:** o que o Zeron (`zeronsh/zeron`, app GPUI com muito movimento e
vidro) tem que melhore o visual do xemnas, e vale manter um fork do GPUI para
isso?
**Status:** Em implementação. Etapa 1 (sem fork) entregue; fork mínimo e
divisão das telas abertos.

## Fontes

- Zeron em `27480d99` (MIT, © 2026 Wing):
  [`crates/ui/src/motion.rs`](https://github.com/zeronsh/zeron/blob/27480d99b9b75d31d01832248bec3f898bd64dcc/crates/ui/src/motion.rs),
  [`popover.rs`](https://github.com/zeronsh/zeron/blob/27480d99b9b75d31d01832248bec3f898bd64dcc/crates/ui/src/popover.rs),
  [`glass.rs`](https://github.com/zeronsh/zeron/blob/27480d99b9b75d31d01832248bec3f898bd64dcc/crates/ui/src/glass.rs).
- O fork de GPUI deles, [`zeronsh/zui`](https://github.com/zeronsh/zui) em
  `667d0aa`: README (procedência e licenças) e os commits de blur e fade.
- Pesquisa externa do usuário (`zeron_gpui_pesquisa.md`, fora do repositório).

## O que entrou (etapa 1, sem fork)

Tudo funciona no GPUI que já usamos (Zed `2440236`). Regras em
`docs/design/VISUAL-IDENTITY.md`, seções Movimento e Interação; atribuição em
`NOTICE`.

| Ideia do Zeron | Onde está | Commit |
| --- | --- | --- |
| Curvas cúbicas exatas e catálogo de durações | `ui::motion::{curve, spec}` | `5ef6a10` |
| Relógio compartilhado com leases (fim do redesenho contínuo do mascote e do grafo) | `ui::motion::clock` | `5ef6a10` |
| Popover com fase de saída e anotação do clique no gatilho | `ui::popup` | `f63635f` |
| Placa de material (gradiente, borda, realce, sombra) | `ui::material` | `ac3f5d9` |
| Indicador que desliza e muda de alvo no caminho | `ui::motion::glide` | `5820032` |

O achado mais importante foi de desempenho: o Zeron mediu uma janela presa na
taxa do monitor (36% de CPU) por loaders em `with_animation` repetido. O
flutuar do mascote fazia isso aqui o tempo todo.

## Fork do GPUI: decisão

**Não usar o `zui`, e não manter um fork amplo agora.**

- **Licença.** No `zui`, o crate `path` (dependência de `util`, de que o GPUI
  depende) está declarado GPL-3.0-or-later. O xemnas é MIT; na nossa revisão
  do Zed o mesmo crate é Apache-2.0.
- **Base mais velha.** O `zui` foi extraído de um Zed de 19/07/2026, sem
  histórico em comum; a nossa base é de 28/09. Desde 19/07 o GPUI recebeu
  ao menos 100 commits e o backend Windows 27 (limite da consulta à API).
- **Custo.** O blur no DirectX são ~2.300 linhas só nos commits de
  setembro (shader HLSL e renderer), sobre uma base anterior de BackdropBlur;
  cada atualização vira rebase de renderer. Cada mudança no fork recompila o
  GPUI inteiro, e aqui isso ainda passa pelo Smart App Control.
- **Valor.** O Quiet Glass foi feito para não depender de blur.

**Se o design pedir vidro de verdade (etapa 2):** fork mínimo do **Zed oficial**
(Apache), fixado na nossa revisão, com só dois patches portados do `zui` com
atribuição: fade de borda (pequeno: listas, abas, timeline) e, se aprovado um
desenho de paleta/assistente foscos, o blur do DirectX. Rebase mensal e teste
de renderização. Melhor ainda: propor o fade de borda ao próprio Zed.

## Separação de responsabilidades (aberto)

O Zeron concentra demais (`shell.rs` ~710 KB). Temos o mesmo em menor escala:
`screens/map.rs` 3,6 mil linhas, `context.rs` e `graph.rs` 2,3 mil, `app.rs`
1,8 mil. Proposta, por feature e sem refactor grande de uma vez:

- `screens/map/` com `index`, `suggestions`, `timeline`, `blocks`, `entity`;
- `screens/context/` com uma página por seção;
- o shell (`app.rs`) separando paleta, barra de projeto e roteamento.

Os mecanismos visuais já estão no lugar certo: `ui/motion`, `ui/popup`,
`ui/material`.
