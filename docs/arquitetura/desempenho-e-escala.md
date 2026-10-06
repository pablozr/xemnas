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

## Fila de jobs

Os jobs ficam em filas por assunto (`Lane` em `application::jobs`), cada uma
com seus próprios workers, para que importar uma documentação grande nunca
atrase a análise da sessão que o desenvolvedor acabou de encerrar.

| Fila | Tipos | Workers |
| --- | --- | --- |
| `now` | `analyze_capture` (capturas de sessões de agentes) | até 2 |
| `documents` | `analyze_document` (documentação importada) | até 2 |
| `suggestions` | `suggest_relations`, `derive_claims`, `derive_search_terms`, `suggest_links`, `context_routing` | 1 |

- O tipo diz a fila: `JobKind::lane` é um `match` exaustivo, então um tipo
  novo não compila sem fila. Cada worker só reivindica tipos da sua fila.
- A reivindicação é um único `UPDATE … RETURNING` (atômico com WAL e várias
  conexões; `concurrent_claims_never_hand_out_a_job_twice`).
- A recuperação de jobs interrompidos roda antes de qualquer worker; ao sair,
  `WorkerHandle::stop` + `join` param todos e quem estava no meio de um job
  termina-o antes.
- Contagens por fila (na fila, executando, falhou) vêm de uma consulta
  agrupada (`Jobs::lane_summaries`), nunca de carregar as linhas.

### Limite de chamadas ao provedor

- Um `ProviderLimiter` (`application::limiter`) é compartilhado por todas as
  filas: no máximo N chamadas ao provedor de IA ao mesmo tempo (padrão 2).
  Chamadas de um worker da fila `now` passam à frente das que esperam nas
  outras filas. Só os handlers de jobs usam o limitador
  (`ProviderFactory::with_limiter`); Visão geral, revisão e aprovação
  automática, disparadas pela pessoa, não esperam por ele.
- HTTP 429 vira `ExtractError::RateLimited` com o `Retry-After` (em segundos,
  até 15 min), sem nova tentativa dentro da chamada. O limitador pausa todo
  mundo por esse tempo (20 s sem `Retry-After`); quem chega na pausa recebe o
  tempo restante e devolve o job à fila em vez de segurar um worker.
- Tempo esgotado, falha de conexão e 5xx seguem com três tentativas dentro da
  chamada; esgotadas, viram `ExtractError::Unavailable`. Nos dois casos o job
  volta para `queued` com `run_after` (`Retry-After`, ou 15 s · 2^(tentativas-1)
  com ±25 % de variação, até 10 min) e não registra análise falha. Depois de 8
  tentativas ele fica `failed` com mensagem própria e pode ser reprocessado:
  nenhum job se perde. O limite de uso esgotado do plano ChatGPT continua sendo
  erro com a mensagem do plano, porque esperar não resolve.
- Ao fechar o app, `limiter.close()` libera quem espera vaga (o job volta à
  fila) antes de `stop`/`join` dos workers.
- **Análises em paralelo** (1 a 4, padrão 2) fica na tabela `job_settings`
  (uma linha, como `approval_settings`), lida na partida: define o N do
  limitador e os workers por fila (`Lane::workers`: até 2 em `now` e
  `documents`, 1 em `suggestions`). Mudar vale no próximo início. Não fica no
  perfil de IA porque o formulário de IA regrava o perfil inteiro e o
  consentimento é amarrado a ele; `context_settings` é por projeto.
- Ajustes → Diagnóstico mostra o cartão "Filas de análise": contagens por
  fila e a escolha de análises em paralelo (`segmented`), com o aviso de que
  vale no próximo início.

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
