# ADR-0018 — Idioma nos jobs, juiz no núcleo, mapa descrito ao extrator e dono compartilhado

**Status:** Vigente; emenda os itens 5 e 7 do [ADR-0017](0017-polaridade-como-alarme-componentes-na-extracao-e-idioma-da-saida.md).
Aceito em 09/10/2026.

## Contexto

A primeira rodada real no `cloudrs` mostrou quatro defeitos que o ADR-0017 não cobria:

1. Os motivos do juiz saíam em português, espanhol ou inglês, misturados, e os do proponente de
   vínculos também em francês e espanhol.
2. O juiz automático só rodava quando a tela de Revisão era aberta.
3. O extrator propunha poucos componentes: nenhuma das 44 candidatas de documento citou um.
4. Uma dependência declarada por dois componentes ligava a decisão aos dois.

Este ADR cresce por tarefa; cada seção abaixo vale desde o commit que a introduziu.

## Idioma da saída nos jobs

**Causa.** O prompt do juiz é inglês e pede "o idioma do item", mas o conteúdo do usuário ia
todo com rótulos em português (`[vínculo …]`, `Evidência (…)`, `Escolha`, `Componente`,
`Caminhos`, `Alerta de polaridade`, `Pergunta`, `Motivo`, `Confiança do extrator`, `Fontes`,
`Parecidas em vigor`, `## Regras em vigor`, `## Decisões em vigor`, `[relação`, `[contexto`,
`Citação`, `Da decisão`) e as razões gravadas (`dependência citada:`, `; também declarada por
outro componente`) entravam cruas. O modelo seguia o idioma dominante do texto, e no banco real
respondeu até em espanhol. O proponente de vínculos e o de relações tinham o prompt e o pedido
100% em inglês e a instrução vaga "no idioma da decisão", que não ancora nada: saíram francês,
espanhol e português por lote.

**Decisão.**

- `application::output_language::LanguageSource` (`Arc<dyn Fn() -> OutputLanguage>`) é lida
  quando o job roda, não gravada em configuração: segue a troca de idioma ao vivo, não exige
  migração e o `application` não conhece a interface. `LinkFinder`, `RelationFinder` e
  `Approvals` ganham `.with_language(source)` (padrão inglês, então testes e construtores
  existentes continuam valendo). A composition root (`main.rs::interface_language`) passa
  `i18n::current()`.
- Os quatro pedidos (vínculos, relações e os dois do juiz) começam com `Output language: X` e
  os prompts mandam escrever "no idioma de saída nomeado na primeira linha da mensagem"
  (o padrão da Visão: `output_language::header`).
- O texto que o juiz lê é todo inglês (rótulos, tipos de evidência, o alerta de polaridade e a
  montagem da mensagem em `review_message`); só o conteúdo do usuário (pergunta, escolha,
  citação) segue como foi escrito. O formato das **razões gravadas** não muda: o juiz recebe
  uma descrição em inglês da evidência, não a razão crua.
- Strings de produto em português no ledger ("outro item", "decidido pela IA", "a IA não
  respondeu…", os `Triage::Accept("…")`) ficam para o ticket 21.

**Não verificável aqui:** o idioma real das respostas só aparece numa nova rodada no `cloudrs`.
