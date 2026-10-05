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
| Seleção de contexto | `storage-sqlite/tests/context_corpus.rs` (`context_quality_gate`) | precisão, cobertura e casos negativos contaminados do bloco entregue; p95 de `build_pack` | precisão ≥ 0,65; cobertura ≥ 0,94; contaminados ≤ 2; p95 ≤ 20 ms | 0,65 (17/26); 0,94 (17/18); 2 de 15; 1,0 a 1,7 ms |
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

A precisão do contexto entregue é 0,65: cerca de um terço do bloco é ruído. Esse é o
ganho de assertividade mais valioso hoje. Melhorias candidatas, medidas por este portão:
abster-se quando a relevância é fraca, priorizar o que o grafo liga aos arquivos da
tarefa e penalizar correspondências lexicais soltas.
