# ADR-0012 — Modo automático e autoridade das normas

Status: aceito em 05/10/2026.

## Contexto

O ADR-0010 e o ADR-0011 mantêm a confirmação de normas (decisões e regras) com uma
pessoa. Depois disso, o usuário pediu um modo automático: um interruptor, como o modo de
permissão de um agente, em que a IA cuida do ciclo de aprovação. As duas coisas precisam
conviver sem que o sistema aceite normas com base em sinais fracos.

## Decisão

1. **Manual é o padrão.** Nada vira norma sem uma pessoa enquanto o interruptor estiver
   em Manual. O modo automático é escolha explícita do usuário, por instalação.
2. **No automático, a pessoa delega a confirmação a um juiz de IA, não à confiança do
   extrator.** O juiz recebe cada item em lote, com a evidência, e pode aceitar,
   descartar ou devolver para a pessoa. Toda ação fica no registro "Feito sozinho", com
   quem decidiu e o motivo. Descartes podem ser desfeitos.
3. **As regras locais, que não chamam IA, só descartam repetições.** Elas aceitam uma
   norma sozinhas apenas quando a calibração (`application::calibration`) mostrar que a
   confiança do extrator prevê o que esta pessoa mantém: 30 decisões ou mais e separação
   de 0,70 ou mais. Antes disso, a confiança é opinião do modelo.
4. **Natureza continua valendo.** Inferências nunca viram normas e descrições conferidas
   por observação local não entram na fila (revisão por exceção). O modo automático só
   vê o que exige revisão.
5. **Vínculo citado só no texto vai ao juiz.** Ligações tiradas de arquivos que a decisão
   tocou podem ser aceitas pelas regras; as tiradas de menções no texto, não.

## Consequências

- Sem provedor de IA ativo, o modo automático só descarta repetições; o resto espera a
  pessoa. Isso é intencional.
- O custo de IA sobe no começo (mais itens vão ao juiz). Em 05/10/2026 o usuário tirou os
  limites de ritmo: lotes de até 30 itens, sem espera para juntar itens, sem intervalo
  entre chamadas que funcionam e sem teto diário; só uma falha do provedor pausa a próxima
  chamada por 20 minutos. Quem liga o automático escolhe que a IA faça a revisão.
- A confiabilidade do automático passa a ser medida: a calibração aparece no Diagnóstico
  e a taxa de desfazer no registro indica quando o juiz erra.
