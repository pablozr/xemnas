# Direção: memória persistente e contexto de baixo atrito

**Data:** 03/10/2026.
**Pergunta:** como evoluir a memória do Project sem exigir curadoria contínua do dev?
**Status:** Aberta. Direção confirmada pelo usuário; algoritmos, metas e implementação
da automação ainda dependem de validação. Não altera silenciosamente a confirmação
humana vigente para Engineering Decisions e regras.

Primeiro slice incorporado no [ADR-0011](../arquitetura/adr/0011-observacoes-descritivas-de-manifests.md):
observações automáticas de manifests e router determinístico inicial.
[Resultados e limites](../operacao/memoria-descritiva-e-router.md). IA para
ambiguidade, automação ampliada e comparação de esforço humano seguem pendentes.

## Objetivo aprovado

Xemnas deve assumir captura, organização, manutenção e seleção de memória para
coding agents. O humano intervém quando ambiguidade e impacto justificam, não para
confirmar centenas de registros descritivos. Decisões continuam entidades de
primeira classe; o grafo é infraestrutura interna, não necessariamente a experiência
principal. Performance, baixo custo e redução de atrito são pilares conjuntos.

## Distinções para o contrato futuro

- Descrição: o que uma fonte verificável mostra, com commit/alcance/validade.
  “Usa PostgreSQL” não constitui “deve usar PostgreSQL”.
- Norma: orientação para mudanças futuras; exige autoridade/evidência adequadas.
  Confiança de LLM, frequência e repetição não conferem autoridade.
- Inferência: interpretação identificada, sem promoção silenciosa a regra.
- Proveniência: fonte, mensagens, arquivos/diffs, episódio, suporte, versão,
  confiança e processo/modelo responsável, quando disponíveis. Ausência explícita.

Essas categorias são direção de evolução; o vocabulário/contrato vigente está em
`docs/produto/CONTEXT.md`. Não renomear Context Claim ou Engineering Decision como
sinônimos indiscriminados de memória.

## Algoritmo e custo

Código primeiro: captura incremental, deduplicação, fingerprints, fatos extraíveis
de manifests/configuração/diffs, índices e caches invalidados pela fonte.
IA interpreta intenção/ambiguidades que código não resolve; não reenviar toda a
sessão nem chamar modelo em cada consulta por padrão.

Política de automação combina suporte verificável, autoridade, alcance, atualidade,
contradição e impacto. Descrições verificáveis podem ser incorporadas por política
futura sem fila obrigatória. Normas ambíguas pedem pergunta direcionada. Agrupar
equivalentes e não repetir perguntas sem mudança material.

Context router: Project → tarefa/arquivos → validade/alcance → relevância → orçamento.
Entregar restrições aplicáveis e fatos rastreáveis, explicitar informação parcial e
abster-se de contexto fraco. Detalhes sob demanda e deduplicação por versão válida.
Não garantir exatidão universal ou cortar ressalvas para economizar tokens.

## Medir e explicar cada entrega

Medir latência p50/p95, CPU/memória, chamadas/tokens por episódio, contexto
irrelevante, restrições omitidas, promoções normativas indevidas, intervenções,
perguntas repetidas e tempo humano ativo. Separar estimativas de usage/faturamento,
espera de máquina de esforço humano e resultado intermediário de estado final.
Fixar orçamento/metas antes de ajustar algoritmo; nenhum número foi aprovado aqui.

Cada entrega explica o que mudou, por que, evidência de validação e limites.
Processo de implementação: pacotes completos, frentes independentes em paralelo,
contratos resolvidos antes de dividir, sem duplicar investigação ou rodar checks
idênticos sem mudança. Medir repasses, retrabalho e tempo até entrega validada;
não atribuir lentidão a modelos sem evidência.

## Próxima sequência

Fechar [lote de confiança/clareza](../operacao/lote-confianca-clareza.md), publicar
baseline do corpus e então desenhar a política descritiva/normativa com casos
contrastivos. Só depois experimentar context router e revisão por exceção.
Não ampliar o lote bloqueado com automação ainda sem contrato.
