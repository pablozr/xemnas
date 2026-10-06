# ADR-0006: Tipo e relevância dos candidatos

- **Status:** Vigente. (registro original: aceito)
- **Data:** 2026-09-30
- **Contexto:** no uso real, a extração propunha como "decisão" correções de bug e detalhes de implementação ("`clear` exige identidade do alvo"), e uma resposta com dois ajustes virava dois candidatos. A confiança media se a escolha foi tomada, não se ela importa, então o ruído chegava com 99%. Algumas dessas propostas eram, na verdade, regras de um componente ("só um humano resolve `review_legacy`"), que não cabem como decisão nem devem se perder.

## Decisão

1. **Tipo e relevância na mesma chamada do extrator.** Cada proposta traz `kind` (`decision`, `rule` ou `detail`), `significance` de 0 a 1 e os critérios marcados de um teste de significância adaptado de Olaf Zimmermann: `cross_cutting`, `data_or_contract`, `security_or_privacy`, `external_dependency`, `hard_to_reverse`, `first_of_a_kind`, `past_problem`, `constrains_future_work`. Sem critério, relevância abaixo de 0,3; um, cerca de 0,5; dois ou mais, 0,7 ou mais.
2. **`detail` nunca é gravado.** `rule` vira candidato a regra: confirmar cria uma claim (`constraint`) do projeto, não uma Engineering Decision. Candidatos abaixo de 0,5 ficam fora da fila padrão da Revisão, guardados e visíveis por "Mostrar N de baixa relevância".
3. **O extrator recebe o contexto do projeto** (`ExtractionStore::background`): decisões e regras que o mapa liga aos arquivos da captura e as mais recentes, para não repetir o que já existe; e os últimos candidatos confirmados e rejeitados, como exemplos do gosto do usuário. Tudo local, limitado, nunca capturas cruas.
4. **Juiz separado (segunda chamada) fica para depois**, só se as três medidas acima não bastarem: dobraria o custo por captura.

## Consequências

- Migration 15: `decision_candidates` ganha `kind`, `significance` e `criteria`; candidatos antigos leem como decisões relevantes.
- O schema estrito da saída do modelo inclui os três campos; respostas sem eles leem como decisão no limiar.
- A Revisão mostra o selo "Regra" e "Por que importa" com os critérios.

## Alternativas rejeitadas

- **Juiz sem critério escrito:** um modelo avaliando a própria saída sem rubrica tende a aprová-la.
- **Normalizar a entrada:** o ruído nasce na classificação, não no que o OpenCode entrega.
- **Apagar o que tem baixa relevância:** perderia o que o usuário pode querer recuperar; fica guardado e escondido.

## Estado hoje

`kind`, `significance` e `criteria` seguem no esquema do extrator (`application::extract`, `CandidateKind`). O juiz de IA do modo automático ([ADR-0012](0012-modo-automatico-e-autoridade.md)) decide a aprovação; ele não substitui a classificação feita na extração.
