# ADR-0017 — Polaridade como alarme, componentes na extração e idioma da saída

**Status:** Vigente. Aceito em 09/10/2026. Emenda o item 1 do
[ADR-0016](0016-vinculos-por-estrutura-e-mencao-afirmativa.md) (menção afirmativa) e o
[ADR-0007](0007-visao-do-projeto-gerada-pela-ia.md) (idioma da Visão).

## Contexto

Três erros do mesmo projeto real (`cloudrs`) e um da Visão do JevGuard mostraram o limite da
decisão anterior:

1. "… and no GPUI types" ligou a decisão aos dois componentes que declaram `gpui`. O `no` do
   inglês não era gatilho (em português é "em o"), a dependência estava em dois donos, a regra
   mandou ao juiz e o juiz aceitou: faltava alarme de polaridade na evidência que ele recebia.
2. "… with no rusqlite handles" ligou a um dono único e foi aceito pelas regras, sem juiz.
3. O mesmo para "El núcleo no usa rusqlite": o léxico só conhecia inglês e português, então a
   polaridade dependia do idioma do texto.
4. A Visão do JevGuard saiu em inglês numa interface em português: o prompt pedia "o idioma dos
   registros", e as descrições da raiz e da CI eram frases em português gravadas no banco.

Um léxico de negação por idioma não fecha: toda palavra que nega numa língua pode ser prosa em
outra, e cada língua nova pede uma lista. O que precisa mudar é quem decide.

## Decisão

1. **O léxico só decide o inequívoco em inglês e português.** `NEGATION_BEFORE`/`NEGATION_AFTER`
   continuam descartando o vínculo, sem sugerir nada, mas perdem `salvo`, `outside` e
   `apart from` (podem ser aditivos). Custo de manter: o ADR-0016 mediu 11 vínculos negados indo a
   zero com essa lista; mandar esses casos ao juiz custaria tokens em toda passada, e no modo
   manual devolveria à pessoa o ruído que ele tirou.
2. **Qualquer outro gatilho, em qualquer idioma, levanta dúvida** (`DOUBT_BEFORE`/`DOUBT_AFTER`:
   `no`, `sin`, `sans`, `ohne`, `senza`, `nicht`, `ni`...). O acerto volta com o gatilho (`Mention
   { quote, doubt }`) e a razão do vínculo ganha o sufixo `\npolaridade duvidosa: "<gatilho>" em
   "<citação>"` (`DOUBT_MARK`, `with_doubt`, `doubt_of`, `without_doubt`), em menção, dependência
   e símbolo. O escopo da dúvida é menor que o da negação (`DOUBT_SCOPE_WORDS = 3`; 3, 4 e 5
   deram o mesmo resultado no corpus). Em uma única passada, a frase mais longa ganha: `no longer`
   é inequívoco, `no` sozinho é dúvida. Mais pseudo-gatilhos (`no solo`, `non seulement`, `nicht
   nur`...) e terminadores (`pero`, `sino`, `mais`, `aber`...).
3. **Vínculo em dúvida é fraco.** `triage_link` responde `Ask`; `needs_links` não o conta como
   laço; o juiz recebe `Alerta de polaridade: ...` e o `LINK_REVIEW_PROMPT` explica que pode ser
   alarme falso ("no" do português) e que o link se descarta quando o componente é nomeado para
   ser excluído, evitado ou trocado; no modo manual fica pendente. Arquivo citado por documento
   com polaridade duvidosa não conta como arquivo (cai na menção do componente, que leva a
   marca); arquivo tocado por diff nunca tem dúvida. O roteamento de tarefa e o estreitamento de
   regra (`scope.rs`) seguem contando o acerto duvidoso, como antes.
4. **`revalidate` passa a rever dependência e símbolo** pendentes ou confirmados pelas regras
   (nunca pela IA, pessoa ou herança). Só duvidoso e a razão sem marca: invalida como `Rules`, e
   `blocked()` deixa a derivação reescrever a aresta com a marca, que vai ao juiz. Ausente ou
   negado: invalida. Menção continua sustentada por acerto limpo ou duvidoso.
5. **O extrator diz onde a decisão vale.** Recebe só os nomes dos componentes vivos (até 60, em
   ordem de chave), no início do conteúdo (prefixo estável para o cache do provedor), e responde
   `components:[{name, quote}]` com no máximo 3. O nome é conferido por `entity_key` contra o
   mapa; a citação tem de ser literal da `question`, `choice` ou `rationale` da própria
   candidata (a mesma validação do `link_suggestions`, mínimo de 8 caracteres). Fica na tabela
   lateral `candidate_components` (migração 0048, só para frente, sem FK para `entities`):
   enfiar isso em `qualifiers` mudaria a semântica do ADR-0010.
6. **Na adoção as componentes viram sugestão da IA**: aresta `Derived` pendente com razão
   `ai_link_reason(citação, EXTRACTED_LINK_WHY)`, só para entidade viva sem nenhuma linha do
   mesmo tipo (um vínculo estrutural ou uma recusa da pessoa ganha) e cuja citação ainda está no
   texto final (edições incluídas). `needs_links` conta a aresta como "já perguntado", então o
   job de `suggest_links` que `queue_untied` tenha enfileirado vira no-op: zero chamadas. O
   `revalidate` não a toca (é "o proposto pela IA"), e quem confirma é o juiz ou a pessoa.
7. **O idioma da saída vem de quem chama.** `OutputLanguage` (`application::output_language`)
   entra em `OverviewApi::generate`; o app manda a tag de `i18n::current()` e o backend a põe na
   primeira linha da mensagem ("Output language: Brazilian Portuguese"). O conteúdo das decisões
   nunca é traduzido. Os motivos do juiz, do proponente de vínculos e de relações seguem "no
   idioma do item" (frases curtas presas a um conteúdo; levar o idioma a jobs em segundo plano
   cria acoplamento antes do refactor do núcleo): pendência do ticket 21.
8. **Raiz e CI não guardam descrição.** Ficam vazias no banco; `graph::infra_kind` deduz o tipo
   pelos padrões (`*` é raiz; padrão de `CI_SYSTEMS` é CI), a interface mostra o texto pelo i18n e
   ao modelo vai um texto fixo em inglês (`graph::description_for_model`). Bancos existentes são
   limpos uma vez no refresh, só quando a descrição é exatamente o literal antigo.

## Consequências

- O custo novo é o do juiz: no corpus estrutural (5 projetos, 60 decisões), `to_judge` foi de 23
  para 26 e `doubtful` é 10 (4 certos, 6 errados). O português paga mais: todo "no X" vira
  pendente. O ajuste é `DOUBT_SCOPE_WORDS`, medido sem efeito entre 3 e 5 no corpus.
- O que as regras aceitam sozinhas deixa de depender do idioma: `rule_accepted_wrong` foi de 2
  para 0. Nada no léxico aceita ou descarta fora de inglês e português; o juiz ainda pode
  aceitar um alarme (o alerta melhora a evidência, não a garante).
- Vínculo já confirmado pelo juiz ou por pessoa não é revisto (de propósito); só os aceitos pelas
  regras são reabertos.
- O prompt do extrator cresce 1.440 bytes com 60 nomes de 20 caracteres, num prefixo idêntico
  entre capturas. Com o extrator citando o componente, o proponente de vínculos faz 0 chamadas
  contra 1 sem ele (`extracted_links.rs`). A citação prova que o componente aparece no texto da
  decisão, não na fonte; o juiz continua decidindo.
- As mensagens `polaridade duvidosa` e `indicado pelo extrator junto com a decisão` seguem em
  português até o ticket 21, como as demais razões de vínculo.

## Medições (09/10/2026)

| Gate | Antes | Depois |
| --- | --- | --- |
| Vínculos por estrutura, precisão das asseridas | 0,857 (36/42) | 1,000 (32/32) |
| Cobertura asserida / com o juiz | 1,000 / 1,000 | 0,889 (32/36) / 1,000 |
| `rule_accepted_wrong` | 2 | 0 |
| `negated_linked` | 4 | 0 |
| `to_judge` / `doubtful` | 23 / 0 | 26 / 10 |
| Menção, precisão / cobertura asserida / com o juiz | 0,889 (48/54) / 1,000 / 1,000 | 0,958 (46/48) / 0,958 / 1,000 |
| Menção, 2.000 decisões × 60 partes | 0,8 s | 0,5 s |
| Refresh frio / quente (5.000 arquivos) | 2,1 s / 1,0 s | 2,0 s / 0,9 s |
| Chamadas do proponente de vínculos, com o extrator / sem | 1 / 1 | 0 / 1 |
| Bytes do prompt do extrator com 60 componentes | 0 | 1.440 |
