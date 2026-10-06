# Ciclo correção → verificação → enforcement

Data: 2026-10-06.

**Pergunta:** além de guardar e injetar contexto, como o xemnas pode aprender com o
que o usuário corrige nos agentes, conferir o que eles entregam e transformar as
regras que continuam sendo violadas em checks que não custam tokens?

**Status:** Aberta. Ideias anotadas numa conversa; nada pesquisado a fundo, medido
nem implementado.

## Visão geral

As três ideias formam um ciclo, e cada etapa mede a anterior:

1. **Capturar a correção.** A regra nasce do erro real que o agente cometeu.
2. **Verificar no diff.** Antes do commit, a regra é conferida no que o agente entregou.
3. **Promover a enforcement.** Se a regra continua sendo violada, ela vira teste ou lint.

Isso reforça a direção "core first": menos atrito para criar memória, injeção mais
assertiva e uma métrica de eficácia que hoje não temos (violações evitadas).

## 1. Aprender com as correções

Hoje a memória nasce do que o agente propõe e o usuário aprova. A fonte mais rica é
cada vez que o usuário **corrige** um agente: uma mensagem de correção ("não, use X"),
um revert, uma edição humana logo depois da do agente no mesmo trecho. É uma regra
implícita que já vem com a evidência.

- **Detecção:** primeiro uma heurística barata sobre a sessão capturada e o diff
  (mensagem do usuário com negação ou imperativo depois de uma edição do agente;
  revert; reescrita do mesmo hunk). A IA entra só para redigir o candidato.
- **Saída:** uma regra candidata, com a correção como evidência, que entra na
  revisão normal (ou na aprovação automática por faixas).
- **Em aberto:** o limiar entre "correção pontual" e "regra"; o escopo da regra
  (arquivo, módulo, projeto); como evitar ruído em sessões exploratórias.

## 2. Guarda no diff

Injetar contexto é torcer para o agente ler. O passo seguinte é conferir o resultado:
num hook de fim de turno (`Stop`/`PostToolUse` no Claude Code; equivalente no
OpenCode), cruzar o diff com as regras confirmadas daquele escopo e avisar ou
bloquear antes do commit, citando a decisão (`D:<ref>`).

- **Custo:** filtro por escopo e léxico primeiro; o modelo só julga quando há
  candidata. Sem modelo na maioria dos turnos.
- **Métrica que nasce junto:** violações detectadas, confirmadas e evitadas, por
  regra. É a medida de eficácia mais direta que o produto teria.
- **Em aberto:** avisar ou bloquear por padrão; falsos positivos (uma regra
  bloqueando errado destrói a confiança mais rápido que uma regra ausente); como o
  usuário contesta uma acusação e isso volta como evidência para a regra.

## 3. Promover memória a enforcement

Regra violada repetidamente é sinal de que deveria ser código. O xemnas propõe o
check certo: teste de arquitetura (o crate `architecture`), regra de lint, entrada
num hook. A memória sai do orçamento de tokens e passa a custar zero por sessão.

- **Gatilho:** N violações confirmadas pela guarda (item 2), ou regra mecanizável
  por natureza (dependência proibida, caminho, nome).
- **Saída:** um patch proposto com o teste e o vínculo à decisão; a regra fica
  marcada como "garantida por teste" e deixa de ser injetada.
- **Em aberto:** quais regras são mecanizáveis; quem mantém o teste quando a
  decisão muda (o radar de premissas precisaria enxergar testes vinculados).

## Relação com pesquisas existentes

- [medir-eficacia-do-contexto.md](medir-eficacia-do-contexto.md): a guarda dá uma
  métrica direta que complementa a proposta de medição.
- [20-ideias-dores-reais-devs.md](20-ideias-dores-reais-devs.md) (17, mapa de
  verificação): lá a verificação é obrigação → evidência da tarefa; aqui é
  regra confirmada → diff.
- [direcao-memoria-contexto-baixo-atrito.md](direcao-memoria-contexto-baixo-atrito.md):
  a captura por correção é uma fonte de memória de baixo atrito.

## Outras ideias da mesma conversa (não priorizadas)

- Sincronização ao vivo de decisões entre sessões paralelas, com alerta de conflito.
- Memória procedural: minerar fluxos que deram certo e propor skills e hooks.
- "Blame do porquê": trailer ou git notes ligando commit a decisão, e
  `file_context` ancorado na linha.
