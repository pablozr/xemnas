# Desempenho e escala

O pilar do xemnas é **desempenho e baixo custo**: CPU em repouso, memória,
chamadas e tokens de IA, dependências. Entre duas soluções equivalentes, vale a
mais barata. Este documento são as regras que decorrem disso; o que está
pendente fica em `docs/pesquisas/backlog-de-ideias.md` (seção Desempenho e escala).

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

### Prioridade dentro da fila

`JobKind::priority` ordena os tipos de uma fila (menor primeiro) e `claim_next`
recebe os tipos já nessa ordem (`ORDER BY` pela posição do tipo, depois o mais
antigo): o que o caminho quente do agente precisa vem antes do que só refina.

| Prioridade | Tipos |
| --- | --- |
| 0 | `analyze_capture`, `analyze_document`, `context_routing` |
| 1 | `suggest_links` (a decisão chega ao componente que ela rege), `refresh_observations` |
| 2 | `suggest_relations` |
| 3 | `derive_claims` |
| 4 | `derive_search_terms` |

A análise das decisões roda em filas próprias (`now`, `documents`), então nunca
espera por sugestões. Dentro de `documents`, a ordem de enfileirar é a de
`Documents::propose`: ADRs e especificações, depois README e guias/tarefas
(`DocumentKind`, `documents_are_queued_for_analysis_adrs_and_specs_before_guides`).
A fila `suggestions` tem um worker: uma sugestão em curso termina antes de o
próximo vínculo começar, mas nenhum lote de relações, regras ou termos passa à
frente de um vínculo à espera.

### Lotes de IA

Cada decisão adotada enfileira um job por tipo (relações, regras, vínculos) e um
por projeto (termos). Um job por decisão custava uma chamada ao provedor cada:
37 decisões, cerca de 250 chamadas e 52 minutos num projeto real. Agora os jobs
do mesmo tipo se juntam na hora de rodar:

- `Jobs::register_batch(kind, max, handler)`: quando um job do tipo é
  reivindicado, `JobRepository::claim_more` reivindica, atomicamente, até
  `max - 1` jobs mais antigos do mesmo tipo, e o handler recebe todos de uma vez
  (`&[JobRecord]` → um resultado por job). Cada job termina como `completed`,
  `failed` ou volta à fila (`Deferred`/429, todos do lote com o mesmo prazo)
  sozinho. Um lote é sempre do mesmo tipo e o handler agrupa por projeto.
- `application::batching::BATCH_SIZE = 10` assuntos por chamada. O prompt vira uma
  lista numerada (`## Decision 1`, `## New decision 1 D:xxxx`) e a resposta traz uma
  entrada por número (`{"decisions":[{"id":"1", ...}]}`, `batching::answers`). As regras
  por assunto do prompt são as mesmas; cada entrada é validada sozinha, com a mesma
  checagem de antes (citação literal contra o texto *daquela* decisão, ids, ciclos,
  duplicatas). Uma entrada ilegível ou ausente perde só o seu assunto.
- Os quatro tipos continuam separados (cada um com seu portão de qualidade):
  `suggest_links` (`LinkFinder::run_many`, componentes listados uma vez para todos),
  `suggest_relations` (`RelationFinder::run_many`, candidatos de cada decisão sob ela),
  `derive_claims` (`ClaimFinder::run_many`) e `derive_search_terms`
  (`SearchTermFinder::run_projects`, que já pedia até `BATCH` decisões por chamada; os
  jobs enfileirados do mesmo projeto, um por adoção, agora rodam como um só).
- Chamadas para N decisões: ⌈N / 10⌉ por tipo, em vez de N (4N → 4·⌈N / 10⌉, no máximo: só
  decisões sem vínculo pedem `suggest_links`; para as 37 do projeto medido, até 16 em vez de
  até 148 nos quatro tipos, fora as análises, que continuam uma por documento). Portão:
  `storage-sqlite/tests/batched_jobs.rs`, em `python tools/core-quality.py`.
- O runner (`apps/desktop-gpui/tests/e2e_pipeline.rs`) conta as chamadas pelo
  `ProviderLimiter::calls` (cada licença é uma chamada) e as imprime por passo e no total.

### Limite de chamadas ao provedor

- Um `ProviderLimiter` (`application::limiter`) é compartilhado por todas as
  filas: no máximo N chamadas ao provedor de IA ao mesmo tempo (padrão 2).
  Chamadas de um worker da fila `now` passam à frente das que esperam nas
  outras filas; `ProviderLimiter::calls` conta as chamadas feitas, para o runner. Só os handlers de jobs usam o limitador
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
- Medição: `layout_benchmark` (layout, ignorado por padrão) e `XEMNAS_GRAPH_RENDER=full|cull|cheap|lod` (pintura); números na seção Medições abaixo.
- Grafo: `fold_crowds` agrupa multidões, `Scene::on_screen` e
  `links_on_screen` cortam o que está fora, `crowded` liga o modo barato.
- Layout do grafo em segundo plano com geração para descartar resultado velho
  (`GraphCanvas::set_graph`).

## Medições (release, 02/10/2026, uma máquina)

- **Solver do layout.** Barnes–Hut vence o laço exato a partir de ~1.000 nós, de 3x
  a 6x, sem perder qualidade; em 250 nós o exato é mais rápido. A grade espacial nunca
  foi a mais rápida e saiu do app. 10.000 nós levam ~2,9 s em fundo: não congelam a
  interface, mas o resultado demora. O layout para quando surge uma geração mais nova
  (contador atômico), em vez de terminar um resultado que seria descartado.
- **Pintura (`Scene::paint`, CPU, mediana por quadro, zoom que ajusta o mapa).**
  1.000 entidades: 9,4 ms (tudo) para 1,5 ms (modo barato mais panorama); 2.500
  componentes: 10,4 ms para 2,5 ms. O modo barato vale 1,8x a 3x e o panorama mais
  1,7x a 2x; o corte por viewport quase não ajuda no zoom de ajuste, onde tudo está
  na tela.
- **Montagem por tela (`XEMNAS_PERF=1`).** Escala 1000: Mapa 4,3 ms, Decisões 0,5 ms,
  Revisão 0,6 ms (medianas). Escala 2500: Mapa 0,5 ms; layout do grafo 566 ms antes
  do Barnes–Hut, 321 ms depois, fora da thread da interface.
- **Não medido:** GPU, rolagem e zoom contínuos em uso real, zoom aproximado, 5.000
  nós ou mais (semear 5.000 decisões passa de 100 s) e hit-test. Uma varredura linear
  de 2.500 nós por movimento do mouse é pequena diante do que foi poupado. Duas
  execuções por célula, uma máquina: não é p95.

Telas novas seguem estas regras antes de seguir o padrão visual
(`docs/design/VISUAL-IDENTITY.md`).
