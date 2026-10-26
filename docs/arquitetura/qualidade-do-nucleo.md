# Qualidade do núcleo: portões de assertividade e desempenho

O núcleo do Xemnas é o caminho captura → memória (grafo, observações, revisão) →
contexto que chega ao agente. Toda mudança nesse caminho vem com um teste de
**assertividade** (o que entra está certo e o que falta é pouco) e um de **desempenho**
(latência e custo). Os testes medem corpora rotulados e falham quando um piso ou teto é
cruzado.

Rodar todos de uma vez:

```powershell
python tools/core-quality.py
```

O script recompila cada binário de teste com outro `codegen-units` quando o Smart App
Control bloqueia o hash (o comportamento é o mesmo; só o hash muda) e imprime um resumo.

## Portões

| Portão | Onde | Mede | Piso ou teto | Linha de base (05/10/2026) |
| --- | --- | --- | --- | --- |
| Seleção de contexto | `storage-sqlite/tests/context_corpus.rs` (`context_quality_gate`), corpus v3: 42 famílias, 126 consultas, 12 em holdout; inclui regra com escopo de componente, uma convenção global, 15 restrições sem escopo nem vínculo que nunca podem entrar (ruído) e um componente grande (`quasar`) com 20 regras de temas variados, do qual entram só as do tema da tarefa | precisão, cobertura e casos negativos contaminados do bloco entregue, no geral e por divisão; p95 de `build_pack` | precisão ≥ 0,94; cobertura ≥ 0,96; contaminados = 0; p95 ≤ 20 ms | linha de base 0,23 / 0,76 / 9 de 24; com cobertura mínima de termos: 0,63 / 0,58 / 1; com foco do grafo: 0,70 / 0,58 / 1; com ponte PT/EN: 0,75 / 0,91 / 1; com termos de busca como último recurso: 0,76 / 0,95 / 1; com sinônimos como um conceito e cobertura relativa: 0,83 / 0,95 / 1; com a oração principal: 0,94 (63/67); 0,95 (63/66); 0 de 24; holdout 0,96 / 0,89 / 0 de 9; 2 ms; com regras com escopo (06/10/2026; 4 famílias novas): 0,95 / 0,96 / 0 de 27; 2 ms; com a política de injeção de regras (06/10/2026; 15 regras sem escopo semeadas e 3 famílias novas): com a política antiga 0,05 / 0,96 / 30 de 30 (as 15 entravam sempre), com a nova 0,94 (0,9419) / 0,96 (0,9643) / 0 de 30; 3 ms; só a marca `*` torna uma regra global (07/10/2026): uma convenção sem escopo deixa de contar como global, e as semeaduras com convenção global (inclusive as de `observations_evaluation.rs` e `observations_router_corpus.rs`, que já a marcavam) levam a marca; números inalterados: 0,94 (0,9419) / 0,96 (0,9643) / 0 de 30; 4 ms; regras ligadas ranqueadas (07/10/2026; componente grande com 20 regras e 6 famílias novas, 2 negativas; pares `body`/`corpo` e `sign`/`assinar` na ponte PT/EN): 0,95 (0,9490) / 0,97 (0,9688) / 0 de 36; p95 3 ms no `core-quality.py` (6 a 14 ms com a máquina carregada, e antes da mudança 7 a 17 ms nas mesmas condições) |
| Seleção de contexto, holdout selado | `storage-sqlite/tests/context_corpus.rs` (`sealed_v4_quality_gate`), corpus v4: 24 famílias, 72 tarefas escritas às cegas (sem ver o código de seleção), todas holdout | as mesmas | precisão ≥ 0,58; cobertura ≥ 0,61; contaminados ≤ 4 de 21; p95 ≤ 20 ms; **nenhuma regra é ajustada olhando estas famílias** | 0,58 (39/67); 0,62 (39/63); 4 de 21; 2 ms; com a política de injeção de regras e as 15 regras sem escopo semeadas: 0,58 (0,5821) / 0,62 / 4 de 21 (relatado, não ajustado; a política antiga daria 0,03 / 0,62 / 21 de 21 com elas); sem a convenção sem escopo como global (07/10/2026): 0,58 (0,5821) / 0,62 (0,6190) / 4 de 21, igual, sem ajuste |
| Seleção de contexto, calibração | `storage-sqlite/tests/context_corpus.rs` (`calibration_v5_quality_gate`), corpus v5: 30 famílias escritas às cegas, 8 negativas de mesmo vocabulário | as mesmas | precisão ≥ 0,41; cobertura ≥ 0,43; contaminados ≤ 15 de 24; p95 ≤ 20 ms; aqui se ajusta, o v4 continua selado | 0,41 / 0,43 / 15 de 24; com a política de injeção de regras: 0,41 (0,4133) / 0,43 / 15 de 24 (a política antiga daria 0,02 / 0,43 / 24 de 24 com as 15 regras sem escopo); sem a convenção sem escopo como global (07/10/2026): 0,41 (0,4133) / 0,43 (0,4306) / 15 de 24, igual |
| Seleção de contexto em escala | `storage-sqlite/tests/context_scale.rs` (`rule_ranking_scales_to_a_large_project`, ignorado no `cargo test`): 2.120 regras sobre 60 componentes, 20 deles com 100 regras ligadas; 300 tarefas que tocam um componente grande pelo arquivo | p95 de `build_pack` | p95 ≤ 100 ms | antes (07/10/2026, sem o ranqueamento; o grafo e as claims carregados mais de uma vez por pacote): p50 83 ms, p95 138 ms; com uma carga do grafo por pacote e no máximo `MAX_TIED_RULES` (3) regras ligadas: p50 28 ms, p95 32 ms no `core-quality.py` (39 a 46 / 61 a 83 ms com a máquina carregada). Não medido: onde vai o resto do tempo (busca FTS e carga do snapshot) |
| Ligações por menção | `application/src/graph/mention.rs` (`mention_quality_gate`, `mention_matching_scales_to_a_large_project`) | precisão e cobertura em textos rotulados, incluindo o corpus de pacotes com apelidos derivados (com negativos de mesmo vocabulário) e casos de negação (en e pt), listas coordenadas, pseudo-gatilhos e nome dentro de outro caminho; tempo para 2.000 decisões × 60 partes | precisão ≥ 0,95; cobertura afirmada ≥ 0,95; cobertura com o juiz = 1,0; duvidosas ≤ 6; ≤ 3 s | 0,956 (43/45); 1,0; 0,9 s (07/10/2026, com negação, lista e segmento de caminho; antes 0,90 (27/30); 1,0; 1,1 s); 09/10/2026, com palavras de polaridade de es/fr/de/it e o "no" do português: o léxico só decide o inequívoco em en/pt e qualquer outro gatilho levanta dúvida (não afirmada, vai ao juiz). Antes, com o léxico decidindo: 0,889 (48/54); 1,0; 0,5 s. Depois: 0,958 (46/48); 0,958 afirmada e 1,0 com o juiz; 6 duvidosas (2 certas); 0,5 s |
| Relevância (estrutura e triagem do turno) | `application/tests/relevance_corpus.rs` (`relevance_quality_gate`, `relevance_filter_latency`), 64 capturas rotuladas: casos inline de `fixtures/relevance_cases.rs` + fixtures de `tests/fixtures/extraction` | cobertura, passagens falsas por estrutura, harness triado, chamadas de triagem e caracteres por chamada, tempo de `assess_relevance` para 2.000 capturas | cobertura ≥ 1,0; falsas = 0; harness = 0; triagens ≤ 50; ≤ 2.800 caracteres por chamada; ≤ 800 ms | 14/10/2026, antes de a mudança de rumo do usuário contar como sinal: 0,235 (4/17); 0 de 23; 191 ms. Depois: 1,000 (18/18); 0 de 23; 219 ms. Verbos operacionais soltos ("prioriza", "abandon") viram pista, com 2 negativos novos: 0 de 25; 226 ms. Pistas só somam dentro de uma mensagem, com 1 negativo novo (duas mensagens com uma pista cada): 0 de 26; 214 ms. Mensagem do harness (`<task-notification>`, `<system-reminder>`, `/compact`) deixa de contar como fala do usuário, com 1 negativo inline, 7 fixtures reais (JevGuard e cloudrs, caminhos e e-mails mascarados) e 1 positiva sintética (harness em `user_text` não veta o diff de migração). Antes: 1 passagem falsa em 27 (a notificação com frase de rumo). Depois: 1,000 (19/19); 0 de 34; 327 ms. Escolha do usuário entre opções (`user_picks_option`) e edição de documento normativo, que furam os vetos: antes, com os 4 positivos e 1 negativo novos: 0,826 (19/23); 0 de 35; 227 ms. Depois: 1,000 (23/23); 0 de 35; 326 ms. Replay sobre a cópia do banco (81 capturas `Empty`, `relevance_replay`): passam 1 → 7 (a 1 já chegava ao modelo), 8.144 → 31.350 caracteres de artefatos (≈ 7,8 mil tokens, +5,8 mil) em 7 chamadas (+6). A regra de ordinal em palavras livres saiu (a escolha falada fica para a triagem do turno; `real-option-pick.json` deixa o portão): 1,000 (22/22); 0 de 35; 262 ms; replay 6 de 81, 26.993 caracteres (≈ 6,7 mil tokens). 18/10/2026, estrutura decide e a fala vai à triagem do turno (d3efa58): 1,000 (29/29); 0 de 35; harness 0; 50 de 64 triadas; máx. 1.318 caracteres por chamada (33.233 no total); 198 ms. Replay (26/10, 116 capturas de 07 a 18/10): pass 10, triage 68 (≈ 21,3 mil tokens), empty 38, 3 em risco 26/10/2026, "durável" definido no prompt da triagem: mesmas 29/29, 0 de 35 e 50 de 64; máx. 1.473 caracteres por chamada (40.983 no total); 225 ms. |
| Triagem do turno (encanamento) | `application/tests/extraction.rs` (`triage_*`; entrada `turn triage` do `core-quality.py`) e `ai-provider/tests/extractor.rs` (`triage_sends_the_prompt_and_only_the_turn_text`) | estrutura vai direto; harness nunca chega; escolha em palavras livres chega e grava a linha; turno rotineiro termina vazio com uma linha; offline termina vazio; falhas adiam ou fecham com código | todos passam | 8/8 em `cargo test -p application --test extraction triage_` |
| Triagem do turno, acerto ao vivo | `ai-provider/tests/triage_live.rs` (ignorado; `XEMNAS_LIVE_TRIAGE=1`) | decisões reconhecidas e não-decisões barradas, chamadas, caracteres | reconhecidas = 1,0; barradas ≥ 0,96 | 26/10/2026, `sonnet` (Claude Code): 21/21 reconhecidas; 25/29 barradas (0,862); 50 chamadas, 33.233 caracteres; 227 s (≈ 4,5 s por chamada). Mesmo dia, "durável" definido no prompt: 21/21; 28/29 (0,966); 40.983 caracteres; 203 s |
| Entrega de contexto (injeção) | `storage-sqlite/tests/injection_corpus.rs` (`injection_quality_gate`, `injection_latency_gate`): reproduz o dogfood do JevGuard e roda o `prepare` real com SQLite: decisão, regra global e um `package.json` raiz com 4 observações; sessão de 7 turnos (pedido de cache, duas `<task-notification>`, pedidos sem relação com o manifest, um que cita o `vitest`, um que cita o `package.json`); 50 sessões para o tempo | `filler` (bloco sem nada do que o turno pedia), `harness` (bloco ou linha de `context_injections` para prompt sintético), cobertura do que cada turno pedia, média de tokens por bloco e p95 de `prepare` | filler = 0; harness = 0; cobertura = 4/4; média ≤ 75 tokens; p95 ≤ 20 ms | antes (14/10/2026): filler 3, harness 1, cobertura 2/4, média 217 tokens (4 blocos, 870 tokens); p95 3,1 ms. Depois: 0, 0, 4/4, média 79 tokens (3 blocos, 237 tokens); p95 2,2 a 2,6 ms. Qualificadores como texto simples na linha compacta (14/10/2026): média 74 tokens (3 blocos, 223 tokens). Portão de densidade `compact_density_gate` (pack no formato do dogfood, 16 decisões e 53 regras, qualificadores com id de artefato): antes 2 itens de 478 caracteres nos 300 tokens, depois 3 itens de 350 caracteres; piso ≥ 3 itens e ≤ 360 caracteres por item; nenhuma linha leva `artifact_id`, `qualifiers:` nem id de artefato |
| Reconciliação de observações | `storage-sqlite/tests/observation_reconcile.rs` (`reconcile_idle_gate`, `reconcile_tick_latency_gate`): projeto parado com `package.json` real em disco, 60 voltas do reconciliador (uma por minuto), depois manifest reescrito e `Cargo.toml` novo; 64 projetos com 3 manifests e 50 voltas para o custo | jobs `refresh_observations` e geração criados em voltas sem mudança, refresh por manifest alterado, p95 da volta sobre 64 projetos | voltas paradas depois da primeira = 0 jobs e 0 geração; manifest alterado = exatamente 1 refresh; p95 ≤ 100 ms | antes (14/10/2026, banco real): 120 jobs por hora com 2 projetos parados (60 de 60 voltas agendavam, inclusive em horas sem uso). Depois: `gate reconcile idle_ticks=60 jobs=1 generation_delta=1` (só a primeira volta depois de abrir o app agenda; as outras 59 agendam 0); `latency reconcile_tick p50=13,6 ms p95=20,0 ms` para 64 projetos |
| Vigia de documentos | `storage-sqlite/tests/document_watch.rs` (`document_watch_gate`, `document_watch_tick_latency_gate`): projeto com ADR central já indexado, 60 voltas paradas, depois ADR editado, 3 edições em 3 voltas, ADR novo, edição em pasta `archive-*` e projeto nunca indexado; 64 projetos com 40 documentos e 50 voltas para o custo | jobs `analyze_document` e recibos criados em voltas sem mudança, jobs por edição, rajada, arquivo novo, arquivado e projeto sem índice, p95 da volta sobre 64 projetos | voltas paradas = 0 jobs e 0 recibos; ADR editado = exatamente 1 job (depois de 1 volta de espera); 3 edições seguidas = 1; arquivado e não indexado = 0; p95 ≤ 500 ms | antes (17/10/2026): documento editado, inclusive por subagente, só chegava à Revisão se alguém atualizasse o overview. Depois: `gate document_watch idle_ticks=60 jobs=0 edited_jobs=1 burst_jobs=1 new_jobs=1 archived_jobs=0 unindexed_jobs=0`; `latency document_watch_tick p50=60,5 ms p95=72,2 ms` para 64 projetos de 40 documentos (2.560 arquivos listados, nenhum aberto; em 4 execuções, p95 de 72 a 221 ms: a listagem depende do sistema de arquivos e oscila, por isso o teto de 500 ms); a volta de assentamento (reler a pasta e calcular o hash dos 2.560 arquivos) leva ≈ 485 ms nos 64, e só roda depois de uma mudança |
| Vínculos por estrutura | `storage-sqlite/tests/link_structure_corpus.rs` (`link_structure_quality_gate`, `link_structure_scales`, este ignorado no `cargo test`): 5 projetos sintéticos em disco (o quinto, `cloud`, com o dono duplo de `gpui` e decisões em en/pt/es), sem `.git` (workspace Rust com o app homônimo do projeto, monorepo pnpm, npm workspaces e pacote único), 53 decisões rotuladas (negação, lista coordenada, nome do projeto, arquivo citado em documento, dependência, símbolo, raiz e CI, negativos de mesmo vocabulário); escala com 60 componentes, 2.000 decisões e 5.000 arquivos | precisão e cobertura das arestas `affects` contra os rótulos; `negated_linked` (vínculo a parte só citada para ser excluída), `homonym_linked` (vínculo ao componente de mesmo nome do projeto quando o texto fala do produto), `ghost_proposals` (proposta de componente cuja pasta não existe); `rule_accepted_wrong` (vínculo errado que as regras aceitariam sem o juiz), `to_judge` (pendentes que vão ao juiz) e `doubtful` (com alarme de polaridade), com a cobertura contando os duvidosos certos (`recall_with_judge`); tempo de `refresh_suggestions` frio e quente | piso atual: precisão (afirmados) ≥ 0,95; cobertura afirmada ≥ 0,88 e com o juiz ≥ 0,95; rule_accepted_wrong = 0; to_judge ≤ 27; doubtful ≤ 10; negated = 0; homonym = 0; ghost = 0; listagem de 5.000 arquivos ≤ 300 ms; símbolos de 2.000 arquivos (16 MB) ≤ 800 ms; refresh frio ≤ 4,5 s e quente ≤ 3 s. Alvos ao fim das regras estruturais: precisão ≥ 0,95; cobertura ≥ 0,90; os três contadores = 0 | linha de base (07/10/2026, antes das regras estruturais): 0,487 (19/39); 0,613 (19/31); 11; 4; 3; 2,1 s frio e 1,3 s quente. Com menção afirmativa: 0,783 (18/23); 0,581 (18/31); 0; 2; 3. Com o nome do projeto restrito: 0,857 (18/21); 0,581; 0; 0; 3.. Com o índice de arquivos: 1,000 (20/20); 0,645 (20/31); 0; 0; 0; listagem de 5.000 arquivos 69 ms; refresh 1,5 s frio e 0,8 s quente.. Com o dono da dependência: 1,000 (24/24); 0,774 (24/31).. Com símbolos e nomes de arquivo: 1,000 (27/27); 0,871 (27/31); varredura de símbolos 0,5 s; refresh 2,8 s frio (com a varredura) e 1,5 s quente.. Com a raiz e a CI: 1,000 (31/31); 1,000 (31/31).. 09/10/2026, polaridade como alarme (casos `cloud` e a34 incluídos): antes 0,857 (36/42); 1,000; rule_accepted_wrong 2; to_judge 23; negated 4. Depois: 1,000 (32/32); 0,889 (32/36) afirmada e 1,000 com o juiz; rule_accepted_wrong 0; to_judge 26; doubtful 10 (4 certos); negated 0; refresh 2,0 s frio e 0,9 s quente. 09/10/2026, dependência de dois donos só liga ao dono que a decisão também cita (casos p09 e p10): antes dos dois casos a regra errava ambos (0,944 (34/36); to_judge 29); depois 1,000 (34/34); 0,895 (34/38) afirmada e 1,000 com o juiz; rule_accepted_wrong 0; to_judge 27; refresh 2,0 s frio e 0,95 s quente (2,04 s e 1,00 s antes) |
| Vínculos indicados pelo extrator | `storage-sqlite/tests/extracted_links.rs` (extração, adoção como as regras e o job de vínculo reais; modelo falso que conta chamadas) e `ai-provider` (`the_map_comes_first_and_is_the_same_bytes_for_any_capture`) | chamadas do proponente de vínculos depois de uma adoção em que o extrator citou o componente (citação literal conferida no texto final) contra uma sem componentes; bytes que a lista de nomes acrescenta ao prompt do extrator | com o extrator: 0 chamadas; sem: 1; 60 componentes (nome de 20 caracteres e descrição de 100) ≤ 8.192 bytes; citação parafraseada, nome fora do mapa e edição que remove a citação não viram vínculo; o caso real do updater (a decisão não cita `sc-platform`, o extrator o reconhece pela descrição) vira vínculo com 0 chamadas; variações de nome (caixa, `_`, espaço, crases, `::sufixo`, apelido, nome copiado com a descrição como `sc-platform: OS integration...`) resolvem e `sc-platform crate` não; a linha de `assessments` conta listados, propostos, mantidos, desconhecidos e sem citação | 0 chamadas com o extrator e 1 sem (09/10/2026); 7.569 bytes para 60 componentes com descrição de 100 caracteres (eram 1.440 só com nomes; 7.560 antes de 09/10/2026, quando o cabeçalho passou a pedir só o nome); o prefixo do mapa é idêntico entre capturas, então serve ao cache do provedor |
| Vínculos propostos pela IA | `storage-sqlite/tests/link_corpus.rs` (`link_quality_gate`), corpus de 20 decisões sobre 6 componentes, 8 negativas de mesmo vocabulário; respostas de modelo em `fixtures/link_corpus_answers.json` | precisão e cobertura do pipeline (resposta do modelo → validação de id e citação literal); negativas com vínculo; tempo de montar o pedido e validar | precisão ≥ 1,00; cobertura ≥ 0,75 | primeira rodada real (06/10/2026, gpt-5.6-luna): 0,71 / 0,75, 4 de 8 negativas com vínculo, todas decisões sobre o projeto e não sobre o código (escopo de publicação, dono, nome, glossário); com o prompt dizendo isso: 0,93 / 0,88, 1 de 8 (ganho otimista: a regra veio dessas falhas); prompt em lote no gpt-6-luna (07/10/2026): 1,00 / 0,75, 0 de 8 (uma decisão por chamada: 1,00 / 0,81; as perdas são o segundo componente de decisões que governam dois); validação + pedido de 60 componentes < 5 ms |
| Lotes de jobs de IA | `storage-sqlite/tests/batched_jobs.rs` (`n_decisions_cost_ceil_n_over_ten_calls_per_kind`, modelo falso que conta chamadas), `tests/jobs.rs` (prioridade e lote) | N decisões custam ⌈N / 10⌉ chamadas por tipo (vínculos, relações, regras, termos), nenhuma com mais de 10 assuntos, todos os jobs terminam, uma entrada ilegível ou ausente perde só o seu assunto, uma pausa do provedor devolve o lote inteiro à fila | 23 decisões → 3 chamadas por tipo (eram 23); portão exato | 3 / 3 / 3 / 3 |
| Revisão automática | `storage-sqlite/tests/auto_approval.rs` | regras só aceitam com confiança calibrada; lotes de até 30; falha pausa a próxima chamada; fila esvazia sem teto; desfazer; o juiz nomeia o outro lado de um conflito (candidato ou decisão em vigor) e nenhum id chega ao motivo; vínculo da IA duvidado é descartado e o de menção segue para a pessoa; os vínculos vão num lote à parte, com o prompt próprio `LINK_REVIEW_PROMPT` e a evidência completa (escolha, descrição e caminhos do componente, tipo de evidência), e os motivos estruturais (arquivo, dependência de um dono, símbolo) são aceitos pelas regras sem chamada (custo: no máximo 1 chamada a mais por passada que tenha vínculos e outros itens); as três saídas de um conflito (ficar com esta, com a outra, as duas com escopo); regra repetida (em vigor ou no lote) descartada pelas regras e regra só parecida levada ao juiz com as vizinhas; o juiz é um job do núcleo, enfileirado quando um job que cria itens termina (um só na fila, nada em modo manual, nada duplicado) e uma passada sem nada novo não chama a IA (`latency auto_review_idle_ms`, 3 projetos, abaixo de 200 ms; medido 6 a 17 ms); o que o juiz lê está em inglês, com a linha de idioma primeiro | todos passam | 28/28 (eram 25: o 22/22 anterior estava desatualizado; 3 testes do job) |
| Robustez do pipeline | `storage-sqlite/tests/pipeline_robustness.rs` | o mapa tem os componentes declarados (Cargo e npm) logo após registrar o projeto e indexar documentos, e é idempotente; análise de documento que falhou volta à fila no máximo `MAX_AUTOMATIC_ATTEMPTS` (3) vezes sem pedido e sempre ao importar o arquivo; o motivo da falha fica limpo, em uma linha, com até 200 caracteres e sem conteúdo do documento | todos passam | 8/8 |
| Triagem automática | `application/src/auto_approval.rs` (testes de unidade) | repetição descartada, sem calibração nada aceito pelas regras; `conflicts_with` lido e mapeado, ids trocados pelo título no motivo; nada em português chega ao juiz (`nothing_portuguese_reaches_the_judge`); os dois prompts nomeiam a linha de idioma | todos passam | 18/18 (eram 12 no texto; 16 antes desta tarefa) |
| Regra repetida | `storage-sqlite/tests/auto_approval.rs` (`RULE_PAIRS`, `rule_similarity`), 12 pares rotulados (6 paráfrases em PT/EN, reordenadas ou com sinônimos; 6 regras parecidas mas diferentes, ex.: "warning nunca produz FAIL" x "error sempre produz FAIL") | precisão e cobertura do descarte local (conceitos do `terms.rs` + sobreposição ≥ `DUPLICATE_AT` 0,85) | precisão ≥ 0,9; cobertura ≥ 0,3 | precisão 1,00; cobertura 0,33 (2 de 6: paráfrase entre idiomas e com flexões distintas ainda passa ao juiz) |
| Observações | `storage-sqlite/tests/observations_evaluation.rs`, `observations_router_corpus.rs` (relatórios) | latência de refresh, montagem e consulta; zero chamadas ao provedor | relatório | refresh p95 16 ms; consulta p95 2 ms. O roteador de contexto não tem portão de número; cada falha agora grava um código e o Diagnóstico conta por código (`metrics.routing.failed`), sem número medido ainda |

## Regras

- **Mudou o núcleo, mediu.** Uma mudança em captura, extração, grafo, observações,
  revisão, aprovação automática ou seleção/entrega de contexto inclui ou atualiza um
  portão deste documento.
- **Melhorou, sobe o piso.** Quando uma mudança melhora um número, o piso ou o teto do
  portão passa a ser o novo valor, no mesmo commit. Assim a próxima mudança não devolve o
  ganho sem perceber.
- **Corpus honesto.** Negativos usam as mesmas palavras em outro sentido; um falso
  positivo conhecido fica rotulado como negativo (por exemplo, "o núcleo do problema" não
  é o componente `core`). Não ajuste o rótulo para o número subir.
- **Desempenho com folga.** Tetos de latência têm folga para rodar numa máquina em uso;
  a linha de base registra o valor real.

## Próximo alvo

Com o corpus v3 (distratores do mesmo vocabulário, consultas em inglês e tarefas guiadas
por arquivo), a precisão do contexto partiu de 0,23. Cobertura mínima de termos, foco do
grafo, a ponte PT/EN e os termos de busca gerados na adoção (só como último recurso) a
levaram a 0,76; contar sinônimos da tarefa como um conceito e exigir que um quase-acerto
cubra tanto quanto o melhor resultado, a 0,83; exigir que o item se apoie na oração
principal da tarefa, a 0,94, com cobertura de 0,95 e nenhum caso negativo contaminado
(holdout 0,96 / 0,89). A meta foi atingida **neste corpus**, mas não se sustentou fora dele: o corpus v4,
escrito às cegas por um agente que só viu as decisões (gírias, erros de digitação, prompts
longos com identificadores, quase-acertos de vários formatos), mediu 0,58 / 0,62 / 4 de 21.
Nele, as regras depois da ponte PT/EN somam quatro pontos de precisão e nenhum de
cobertura; os termos de busca trocam três pontos de cobertura por dois casos contaminados.
O v4 fica selado: serve para medir, nunca para ajustar. O próximo passo se decide pelo v4
e pelos dados reais do usuário, medidos só na máquina dele com
`storage-sqlite/tests/dogfood_context.rs` (ignorado; nada do que lê é versionado). Os termos são gerados uma vez por um
modelo real e versionados como fixture (`context_corpus_terms.json`); regenerar exige a
autorização do usuário, porque chama o provedor. O plano está em [precisão do contexto](../pesquisas/precisao-do-contexto.md).
Embeddings como veto e resgate (experimento removido; resultados em
[busca semântica local](../pesquisas/busca-semantica-local.md)) levaram o v4 a
0,67 / 0,60 / 4 (`potion-multilingual-128M`) e 0,60 / 0,60 / 4 (`multilingual-e5-small`) e
não entraram; o corpus de calibração v5, escrito às cegas, mede 0,41 / 0,43 / 15 de 24 e
nenhuma variante com vetor o melhora sem derrubar o v3
([embeddings no contexto](../pesquisas/busca-semantica-local.md)).

## Vínculos propostos pela IA

Uma decisão que veio de um ADR, uma especificação ou uma conversa não toca arquivo, então
nada a liga ao componente que ela rege. Na adoção, se nenhum arquivo, dependência ou
pessoa a ligou ao mapa, o job `suggest_links` pergunta ao modelo a quais componentes vivos
(até 60) ela se aplica e guarda a resposta como sugestão `affects` pendente, que a revisão
confirma ou descarta. O modelo copia as palavras da decisão que a ligam ao componente; o
app confere o id e a citação literal (caixa e acento à parte) e descarta o resto. No máximo
3 vínculos por decisão, nunca um que já existe ou que uma pessoa invalidou, e uma decisão
já perguntada não é perguntada de novo. Sem componentes vivos o job termina sem chamar a
IA. Sugestões com citação verificada vão ao juiz da revisão automática (`Triage::Ask`), que
vê a citação e o motivo.

A mesma pergunta vale para uma regra permanente (claim `constraint` ou `convention`) sem
ligação a componente e cuja decisão de origem, se houver, também não tem vínculo: o job
recebe o id da claim, envia o enunciado (mais o texto dos qualificadores) com o mesmo prompt
e guarda uma sugestão `applies_to` pendente da claim ao componente. É enfileirado na adoção
da regra e pelo refresh do mapa (até 50 por refresh, uma vez por claim); confirmada a
sugestão, o escopo do pacote (`graph::scope`) já separa a regra por tarefa.

O prompt agora é em lote (até 10 decisões numeradas por chamada, componentes listados uma
vez; ver [Lotes de IA](desempenho-e-escala.md)). A fixture continua com o formato de uma
resposta por decisão (`{"links":[...]}` ou a entrada do modelo para aquela decisão, que
`parse_links` lê do mesmo jeito), então o portão segue medindo a validação por decisão. As
respostas atuais foram geradas com o prompt em lote no gpt-6-luna (07/10/2026, 2 chamadas):
precisão 1,00, cobertura 0,75 e 0 de 8 negativas ligadas. Com uma decisão por chamada, no mesmo
modelo e prompt, deu 1,00 / 0,81: o lote custa uma decisão (`redaction`, a última do seu lote).
As outras perdas são do modelo, que liga só o componente principal de uma decisão que governa
dois (falta o `core` em `event-mapping`, `report-format` e `redaction`). Pedir no prompt que
julgasse cada componente não mudou as respostas, então o piso de cobertura desceu para 0,75 e
o de precisão subiu para 1,00.

A fixture é gerada uma vez, por um teste ignorado que chama o provedor e exige a autorização
do usuário:

```
cargo test -j4 --locked -p ai-provider --test link_suggestions_live -- --ignored --nocapture
```

A fixture foi gerada em 07/10/2026 e o portão roda sempre; regenerar exige a autorização do usuário, porque chama o provedor.

## Validação da extração

Cada proposta do modelo é validada sozinha (`extract/validation.rs`); a captura só falha
quando nenhuma proposta passa. Um qualificador exige citação literal: o texto precisa
estar no artefato que ele cita, e o app não aceita declaração humana sem artefato vinda
da IA. Esse é o único campo que o modelo cita de memória e erra com frequência (parafraseia,
junta frases), então `reconcile_with_evidence` o trata como os demais fatos que o app
conhece melhor: descarta o qualificador que não passa (artefato desconhecido, texto que
não é trecho literal, vazio ou longo demais) e mantém a decisão com os que passam, em vez
de perder a proposta inteira. A referência de artefato que apenas contém um id real
(`artifact <id>`) vira o id. A validação estrita (`qualifiers::validate_extracted`)
continua valendo para o que é persistido. Testes:
`extract::tests::a_misquoted_qualifier_is_dropped_and_the_decision_survives` e
`qualifiers::tests::retain_supported_keeps_only_literal_excerpts_within_bounds`.

Medido na execução ponta a ponta sobre um projeto real
([operação](../operacao/operacao-e-referencia.md#execução-ponta-a-ponta-sobre-um-projeto-real)):
antes, 5 de 30 documentos falharam com `qualificador sem citação literal verificável`.

## Relevância: estrutura e triagem do turno

`assess_relevance` (`extract/relevance.rs`) decide o destino de cada captura: `Pass`,
`Triage` ou `Empty`.

- **`Pass`, só por estrutura** (`filter_relevant`): DDL ou segurança em linha adicionada de
  código de produção; dependência nova; edição com linha adicionada em documento normativo
  (ADR, spec, `AGENTS.md`, `CLAUDE.md`, fora de pasta de arquivo); `AskUserQuestion`
  respondida; ou documento. Vai direto ao extrator, sem IA no caminho.
- **`Triage`**: o usuário falou e nada estrutural aconteceu. A triagem do turno
  (`extract/triage.rs`) é uma chamada mínima: `TRIAGE_PROMPT` e um schema estrito
  `{decision, kind}` com `kind` em `choice|rejection|priority|course_change|none`. Leva no
  máximo `TRIAGE_USER_CHARS` (1.500) caracteres da fala, sem lembretes do harness nem bloco de
  contexto, e os últimos `TRIAGE_ASSISTANT_CHARS` (600) do assistente. Fala com menos de 3
  caracteres alfanuméricos (`MIN_SPEECH_ALNUM`) não chama. Um veredito positivo vira o único
  sinal do extrator (`choice` → `user_picks_option`, `rejection` → `rejects_alternative`,
  `priority` e `course_change` → `changes_course`); o negativo termina `Empty`.
- **`Empty`**: o resto, inclusive um turno com documento e sem estrutura.
- Com o extrator offline, o que iria à triagem termina `Empty`.

Falhas: 429 ou timeout adiam o job; outra falha fecha a avaliação como `Failed`, com o código
gravado.

Sem listas de frases. Os exemplos antigos ("de lado", "a partir de agora", "adia", "o 1
funciona, mas o 2 não pq") viraram casos do conjunto
`application/tests/fixtures/relevance_cases.rs`, compartilhado pelo portão, pelos testes
`triage_*` e pelo teste ao vivo.

Mensagem do harness não é fala do usuário: a triagem usa `injection::is_harness_prompt`, a mesma
fonte do hook de contexto (`<task-notification>`, `<command-name>`, `<command-message>`,
`<local-command-`, `<user-prompt-submit-hook>`, `<system-reminder>…</system-reminder>` no início), e também
ignora o `/comando` solto (uma barra e uma palavra). O portão exige `harness_triaged = 0`
(`NEVER_TRIAGED`). Diff, documentos e sinais estruturais continuam iguais.

Registro: cada chamada vira uma linha em `turn_triages` (migração 0052): `kind`, `passed`,
modelo, caracteres enviados e `error_code`, nunca o texto.

Replay (`storage-sqlite/tests/relevance_replay.rs`, ignorado, `XEMNAS_REPLAY_DB`, recusa o
banco vivo, sem IA). Sobre uma cópia do banco real, com 116 capturas de 07 a 18/10: pass 10,
triage 68 (≈ 85 mil caracteres, ≈ 21,3 mil tokens de entrada), empty 38. Ficam 3 em risco: 2
ainda vão à triagem; 1 é um `<task-notification>` de subagente, texto do harness por design
(b417bf9).

Teste ao vivo (`ai-provider/tests/triage_live.rs`). Só roda com autorização do usuário:
`#[ignore]` mais `XEMNAS_LIVE_TRIAGE=1`, com o perfil vivo.

```powershell
$env:XEMNAS_LIVE_TRIAGE = "1"
cargo test --locked -p ai-provider --test triage_live -- --ignored --nocapture
```

Custo por rodada: 50 chamadas, ≈ 41 mil caracteres (≈ 10,2 mil tokens). A saída traz
`recognized` (decisões rotuladas reconhecidas), `barred` (não-decisões barradas), chamadas,
caracteres e erros por código; só nomes de caso e números, nunca a fala. Um 429 para a rodada
e o resultado sai parcial; se as 3 primeiras chamadas falharem, a rodada para e o teste falha.

Primeira rodada (26/10/2026, `sonnet` pelo Claude Code): 21 de 21 decisões reconhecidas e 25 de
29 não-decisões barradas, em 227 s. As 4 passagens falsas são ordens do momento lidas como
prioridade ou mudança de rumo: "foca em fazer o teste passar", "prioriza os testes que
falharam", "foca no teste que falhou / troca pra cargo nextest" e "abandon the retry loop in
this function". O custo de cada uma é uma chamada do extrator, que ainda pode não achar nada.

Segunda rodada (mesmo dia e modelo), com o prompt definindo "durável" como o que vale além da
tarefa atual (focar, fazer primeiro ou tirar algo do código agora é pedido rotineiro): 21 de 21
reconhecidas e 28 de 29 barradas, em 203 s, a +155 caracteres por chamada (33.233 → 40.983 no
conjunto). Sobrou "foca no teste que falhou / troca pra cargo nextest", lido como escolha; a troca
de ferramenta é discutível como decisão. O piso é o desta rodada: reconhecidas = 1,0 e
barradas ≥ 0,96. Outro modelo exige nova rodada.

Lacunas: a decisão no fim do turno anterior do assistente; "pode ser", "ok" e outras
confirmações curtas, porque a decisão está no turno anterior (pegá-las exigiria passar todo
"ok/sim" e ler o texto do assistente); edições feitas por subagentes, que o hook Stop não vê
(só o transcript principal) e por isso só chegam pelo vigia de documentos.

Caso real de 13/10: 18 de 27 capturas de um projeto saíram `Empty`; o extrator achou
candidatos em 3 de 9 sem ruído. Uma delas era a fala "vamos deixar o v2 de lado" num turno
que só mexeu em testes. Não medido: se o extrator, com o prompt atual, propõe a mudança de
rumo como decisão (exige chamar o provedor).

### Pendências (decisão do usuário)

Não implementadas: limite diário de triagens; modelo mais barato só para a triagem; coluna
"Triagens" em `tools/dogfood-report.py`.
