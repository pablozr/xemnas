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
| Seleção de contexto | `storage-sqlite/tests/context_corpus.rs` (`context_quality_gate`), corpus v3: 33 famílias, 99 consultas, 12 em holdout; inclui regra permanente com escopo de componente | precisão, cobertura e casos negativos contaminados do bloco entregue, no geral e por divisão; p95 de `build_pack` | precisão ≥ 0,94; cobertura ≥ 0,96; contaminados = 0; p95 ≤ 20 ms | linha de base 0,23 / 0,76 / 9 de 24; com cobertura mínima de termos: 0,63 / 0,58 / 1; com foco do grafo: 0,70 / 0,58 / 1; com ponte PT/EN: 0,75 / 0,91 / 1; com termos de busca como último recurso: 0,76 / 0,95 / 1; com sinônimos como um conceito e cobertura relativa: 0,83 / 0,95 / 1; com a oração principal: 0,94 (63/67); 0,95 (63/66); 0 de 24; holdout 0,96 / 0,89 / 0 de 9; 2 ms; com regras com escopo (06/10/2026; 4 famílias novas): 0,95 / 0,96 / 0 de 27; 2 ms |
| Seleção de contexto, holdout selado | `storage-sqlite/tests/context_corpus.rs` (`sealed_v4_quality_gate`), corpus v4: 24 famílias, 72 tarefas escritas às cegas (sem ver o código de seleção), todas holdout | as mesmas | precisão ≥ 0,58; cobertura ≥ 0,61; contaminados ≤ 4 de 21; p95 ≤ 20 ms; **nenhuma regra é ajustada olhando estas famílias** | 0,58 (39/67); 0,62 (39/63); 4 de 21; 2 ms |
| Seleção de contexto, calibração | `storage-sqlite/tests/context_corpus.rs` (`calibration_v5_quality_gate`), corpus v5: 30 famílias escritas às cegas, 8 negativas de mesmo vocabulário | as mesmas | precisão ≥ 0,41; cobertura ≥ 0,43; contaminados ≤ 15 de 24; p95 ≤ 20 ms; aqui se ajusta, o v4 continua selado | 0,41 / 0,43 / 15 de 24 |
| Ligações por menção | `application/src/graph/mention.rs` (`mention_quality_gate`, `mention_matching_scales_to_a_large_project`) | precisão e cobertura em textos rotulados, incluindo o corpus de pacotes com apelidos derivados (com negativos de mesmo vocabulário); tempo para 2.000 decisões × 60 partes | precisão ≥ 0,90; cobertura = 1,0; ≤ 3 s | 0,90 (27/30); 1,0; 1,5 s |
| Vínculos propostos pela IA | `storage-sqlite/tests/link_corpus.rs` (`link_quality_gate`), corpus de 20 decisões sobre 6 componentes, 8 negativas de mesmo vocabulário; respostas de modelo em `fixtures/link_corpus_answers.json` | precisão e cobertura do pipeline (resposta do modelo → validação de id e citação literal); negativas com vínculo; tempo de montar o pedido e validar | precisão ≥ 0,93; cobertura ≥ 0,87 | primeira rodada real (06/10/2026, gpt-5.6-luna): 0,71 / 0,75, 4 de 8 negativas com vínculo, todas decisões sobre o projeto e não sobre o código (escopo de publicação, dono, nome, glossário); com o prompt dizendo isso: 0,93 / 0,88, 1 de 8 (ganho otimista: a regra veio dessas falhas); validação + pedido de 60 componentes < 5 ms |
| Revisão automática | `storage-sqlite/tests/auto_approval.rs` | regras só aceitam com confiança calibrada; lotes de até 30; falha pausa a próxima chamada; fila esvazia sem teto; desfazer | todos passam | 8/8 |
| Triagem automática | `application/src/auto_approval.rs` (testes de unidade) | repetição descartada, sem calibração nada aceito pelas regras | todos passam | 9/9 |
| Observações | `storage-sqlite/tests/observations_evaluation.rs`, `observations_router_corpus.rs` (relatórios) | latência de refresh, montagem e consulta; zero chamadas ao provedor | relatório | refresh p95 16 ms; consulta p95 2 ms |

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
sugestão, `claims_out_of_scope` já filtra a regra por tarefa. O prompt não mudou, então a
fixture de respostas continua valendo.

A fixture é gerada uma vez, por um teste ignorado que chama o provedor e exige a autorização
do usuário:

```
cargo test -j4 --locked -p ai-provider --test link_suggestions_live -- --ignored --nocapture
```

A fixture foi gerada em 06/10/2026 e o portão roda sempre; regenerar exige a autorização do usuário, porque chama o provedor.
