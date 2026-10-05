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
| Seleção de contexto | `storage-sqlite/tests/context_corpus.rs` (`context_quality_gate`), corpus v3: 29 famílias, 87 consultas, um terço em holdout | precisão, cobertura e casos negativos contaminados do bloco entregue, no geral e por divisão; p95 de `build_pack` | precisão ≥ 0,74; cobertura ≥ 0,90; contaminados ≤ 1; p95 ≤ 20 ms | linha de base 0,23 / 0,76 / 9 de 24; com cobertura mínima de termos: 0,63 / 0,58 / 1; com foco do grafo: 0,70 / 0,58 / 1; com ponte PT/EN: 0,75 (60/80); 0,91 (60/66); 1 de 24; holdout 0,70 / 0,78 / 0 de 9; 2 a 3 ms |
| Ligações por menção | `application/src/graph/mention.rs` (`mention_quality_gate`, `mention_matching_scales_to_a_large_project`) | precisão e cobertura em textos rotulados (com negativos de mesmo vocabulário); tempo para 2.000 decisões × 60 partes | precisão ≥ 0,92; cobertura = 1,0; ≤ 3 s | 0,93 (13/14); 1,0; 1,5 s |
| Revisão automática | `storage-sqlite/tests/auto_approval.rs` | regras só aceitam com confiança calibrada; uma chamada por lote; limites diários; desfazer | todos passam | 8/8 |
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
grafo e a ponte PT/EN a levaram a 0,75, com cobertura de 0,91. Falta a meta de 0,85 de
precisão: o ruído restante vem de consultas com distrator do mesmo vocabulário, e as
faltas, de paráfrases sem palavra em comum ("coluna" e "esquema", "integração contínua"
e "CI"). O plano está em [precisão do contexto](../pesquisas/precisao-do-contexto.md).
