# Procurador: o Xemnas responde ao agente pela pessoa

**Data:** 06/10/2026.
**Pergunta:** quando o agente para e pergunta algo que uma decisão confirmada já responde, o
Xemnas pode responder no lugar da pessoa, com segurança, sem IA no caminho e sem tirar a
autoridade dela?
**Status:** Aberta. Desenho e protocolo; nada implementado. Escolhida pelo usuário para
aprofundar junto com o [mapa de cegueira](mapa-de-cegueira.md) (06/10/2026). Nasceu em
[ideias de virada](ideias-de-virada.md).

## A dor

Com três ou quatro agentes em paralelo, a pessoa vira o gargalo: cada agente para e espera
("X ou Y?", "posso mudar o esquema?"). Parte dessas perguntas já foi respondida antes, numa
decisão que o Xemnas guarda. Hoje a memória chega ao agente como contexto e ele pode ignorá-la
ou não achar a resposta; ninguém intercepta a pergunta.

## Por onde entrar (verificado na documentação em 06/10/2026)

| Agente | Momento | Como responder |
| --- | --- | --- |
| Claude Code | `PreToolUse` com `AskUserQuestion` | `permissionDecision: "deny"` com `permissionDecisionReason` levando a resposta e a decisão; o agente lê e segue. O gancho não preenche a resposta do formulário, então a resposta vai como motivo da recusa. |
| Claude Code | `Stop`, quando a última mensagem termina numa pergunta | `{"decision": "block", "reason": "…"}` faz o agente continuar com o motivo como retorno. O `stop.rs` já roda nesse evento e já lê o transcript. |
| OpenCode | `session.idle` (o adaptador já trata) | Mandar a resposta para a sessão pelo SDK (`client.session.prompt`, confirmar); a página de plugins não documenta um evento de pergunta. |

Fontes: [hooks do Claude Code](https://code.claude.com/docs/en/hooks),
[plugins do OpenCode](https://opencode.ai/docs/plugins/). O campo `last_assistant_message` no
`Stop` veio de resumo da documentação (confirmar); sem ele, o transcript já lido basta.

**Fora do escopo, sempre:** pedidos de permissão (`PermissionRequest`, `permission.asked`).
O Procurador responde perguntas de projeto; nunca aprova a execução de uma ferramenta.

## Como decide se responde

Precisão antes de cobertura: uma resposta errada custa mais do que uma pergunta.

1. **Extrair a pergunta.** No `AskUserQuestion`, o texto e as opções. No `Stop`, o último
   parágrafo da mensagem, só se terminar em `?` e não houver ferramenta pendente.
2. **Buscar** com a mesma seleção de contexto da injeção (ponte PT/EN, sementes pela menção,
   grafo), restrita a decisões e regras **confirmadas, em vigor e no escopo** dos arquivos da
   sessão.
3. **Responder só se** uma única decisão cobre a pergunta acima de um limiar calibrado, sem
   relação `conflita com` aberta e sem validade vencida. Com opções, a escolha da decisão
   precisa casar com uma das opções; se não casar, não responde.
4. **A resposta é citável e reversível.** Texto fixo: `Xemnas, pela pessoa, a partir de
   D:<ref> vN: <escolha> (<motivo>). Se isso não se aplica aqui, diga e pergunte de novo.`

Sem chamada de IA no caminho. Orçamento proposto: menos de 50 ms no gancho (medir com
`XEMNAS_PERF=1`).

## Escada de confiança

O mesmo caminho do modo automático da revisão ([ADR-0012](../arquitetura/adr/0012-modo-automatico-e-autoridade.md)):

1. **Sombra (padrão).** Não responde; grava o que teria respondido. Quando a pessoa responde
   de verdade, o Stop seguinte compara as duas e registra acerto ou erro.
2. **Ativo por limiar.** Liga quando a sombra acumula N casos com precisão ≥ 0,95 (números a
   fixar no portão); abaixo disso volta sozinho para sombra.
3. **Desfazer.** "Estava errado" no Xemnas registra a correção e a injeta no próximo prompt da
   sessão (`UserPromptSubmit`, `additionalContext`).

## O efeito colateral mais valioso

A pergunta que o Procurador **não** responde sai do agente bem formada, e a resposta da pessoa
fica no transcript logo depois. Pergunta do agente → resposta da pessoa é exatamente o formato
de uma decisão (pergunta → escolha → motivo). Capturar esses pares como candidatos de alta
confiança é provavelmente a melhor fonte de decisão que o Xemnas pode ter, e alimenta o próprio
Procurador: na próxima vez, ele responde.

## O que mostrar

Uma linha na Revisão, como o "Feito sozinho": `Respondi 14 perguntas por você hoje · 0 min de
espera · 1 desfeita`. No modo sombra: `Eu teria respondido 9 de 12; acertei 8`. Cada resposta
abre a pergunta, a decisão usada e a sessão.

## Como medir

- **Corpus às cegas** de perguntas de agente: positivas (cobertas por uma decisão do corpus),
  negativas de mesmo vocabulário (parecem cobertas e não são) e perguntas com opções. Holdout
  selado, como na seleção de contexto. Portão: precisão das respostas, falsas respostas ≈ 0.
- **Desempenho:** latência do gancho por pergunta, com teto.
- **No uso real:** perguntas por sessão, fração respondida, acerto na sombra e tempo entre a
  pergunta e a resposta humana (os transcripts têm horário), que é o tempo de espera evitável.

## Riscos e decisões a tomar

- **Autoridade.** O `CONTEXT.md` diz que o Engineering Assistant não tem autoridade para
  confirmar decisões. O Procurador não cria nem confirma: só aplica uma decisão que a pessoa
  já confirmou. Ainda assim, responder em nome dela muda a fronteira de autoridade e pede um
  ADR antes de sair da sombra.
- **Pergunta retórica ou de cortesia** ("quer que eu continue?") não é decisão: lista de
  exclusão e o limiar cuidam disso; medir no corpus.
- **Agente insistente.** Se o agente perguntar de novo depois de uma resposta do Procurador, a
  segunda vai sempre para a pessoa.

## Plano

O estudo de implementação ([implementação](implementacao-procurador-e-mapa.md)) substitui este
plano: acrescenta a política de autonomia e a regra de exceção.

1. Medir a dor antes de construir: nos transcripts reais, quantas perguntas por sessão, quanto
   tempo de espera e que fração parece coberta por decisões (com autorização do usuário).
2. Captura dos pares pergunta → resposta como candidatos (útil mesmo sem o Procurador).
3. Corpus e portão do casamento pergunta → decisão.
4. Modo sombra no Claude Code (`Stop` e `AskUserQuestion`).
5. ADR de autoridade; modo ativo; depois o OpenCode.
