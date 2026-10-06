# Mapa de cegueira: onde ninguém entende o código que existe

**Data:** 06/10/2026.
**Pergunta:** o Xemnas consegue mostrar, com evidência e a custo baixo, as partes do projeto
que foram escritas por agentes sem que a pessoa as tenha entendido, e ajudar a fechar essas
lacunas antes que custem caro?
**Status:** Aberta. Desenho e protocolo; nada implementado. Escolhida pelo usuário para
aprofundar junto com o [Procurador](procurador.md) (06/10/2026). Nasceu em
[ideias de virada](ideias-de-virada.md).

## A dor e por que ninguém resolveu

- **Dívida de compreensão** é a distância entre o código que se entrega e o código que a equipe
  consegue explicar, depurar e mudar com segurança
  ([Faros](https://www.faros.ai/blog/comprehension-debt-in-software-engineering),
  [Augment](https://www.augmentcode.com/guides/comprehension-debt-ai-code-review)). Quem delegou
  a escrita à IA teve menos de 40% num teste de compreensão posterior; quem usou a IA para
  perguntar conceitos teve mais de 65% (citado por esses artigos; confirmar o estudo original).
- **As métricas atuais quebraram.** Truck factor, grau de autoria e mapas de conhecimento
  supõem que quem escreveu entende. Com código gerado, a mesma autoria é compatível com
  entendimento total, parcial ou nenhum. O artigo pede um "instrumento baseado em compreensão"
  e deixa a construção dele em aberto ([The Substrate Collapse](https://arxiv.org/abs/2606.20882)).
- **O Xemnas tem o dado que falta.** O `git blame` só sabe quem fez o commit; o Xemnas guarda a
  **conversa** que produziu cada trecho: o que a pessoa pediu (`user_text`), o que o agente
  mudou (`diff_hunk`), as ferramentas (`tool_summary`), as decisões confirmadas e o componente
  de cada arquivo (`components_for` no grafo).

## O sinal: engajamento, não autoria

Para cada trecho mudado por um agente numa sessão capturada, uma de três classes, por sinais
locais e sem IA:

| Classe | Evidência na sessão |
| --- | --- |
| **Explicado** | Uma decisão confirmada cobre o componente, ou a pessoa perguntou "por que/como" sobre ele, ou deu a instrução que o especifica (menciona arquivo, símbolo ou componente) |
| **Supervisionado** | A pessoa conduziu a tarefa com instruções concretas, mas não sobre este trecho |
| **Às cegas** | Só aprovação ("ok", "segue", "faz"), modo automático, ou nenhuma fala da pessoa entre o pedido e o fim |

Por componente: a fração do código vivo que veio de trechos às cegas. "Vivo" começa
aproximado (trechos capturados nos últimos meses, menos os substituídos por capturas
posteriores) e só usa `git blame` sob demanda, para os componentes do topo, porque blame é
caro.

É um **indicador**, não prova de entendimento: conversar sobre o código não garante
entendê-lo, e quem leu o diff em silêncio aparece como cego. O desenho assume isso e o testa
(abaixo).

## Prioridade: cegueira × risco

Uma zona cega pequena e parada não importa. A ordem combina:

- **Cegueira**: fração às cegas do componente.
- **Movimento**: quantas sessões mexeram nele recentemente.
- **Dor**: sessões de correção de erro que o tocaram (texto da pessoa com "erro", "bug",
  "quebrou", falha de teste no `tool_summary`).
- **Centralidade**: dependentes no grafo.

## O que a pessoa vê e faz

- **Uma lente no Mapa**: componentes coloridos pela cegueira; na Visão, uma linha: `3 zonas
  cegas; a maior: storage-sqlite (37% escrito sem conversa)`.
- **Tour de 3 minutos** na zona do topo, sob demanda (a única etapa com IA): três perguntas
  sobre o que aquele código faz e por quê, geradas das sessões que o escreveram. A pessoa
  responde em uma linha cada; o Xemnas mostra o trecho e a explicação da sessão. O que ela
  confirmar vira decisão ou regra, e o trecho passa a **explicado**.
- **Para o agente** (`file_context` e injeção): "a pessoa nunca discutiu este módulo; explique
  as mudanças aqui antes de seguir". Numa zona cega, o agente passa a ensinar em vez de só
  entregar, que é o uso da IA que preservou a compreensão no estudo acima.

Respeitando o pilar: classes calculadas na chegada da captura, incrementais; o mapa lê
agregados; o tour só roda quando pedido.

## Não é métrica de pessoas

É local, de cada um sobre o próprio trabalho. Nunca ranking entre pessoas nem exportação
para gestão. Se um dia houver equipe, o mapa mostra componentes, não autores.

## Como saber se é verdade

1. **Previsão falsificável** (a mesma do artigo): zonas cegas devem ter sessões de correção
   mais longas e mais tentativas do agente do que zonas explicadas de tamanho e movimento
   parecidos. Testável no histórico de dogfood (`app.db`, só leitura, já liberado).
2. **Teste com a pessoa**: cinco perguntas sobre zonas cegas e cinco sobre zonas explicadas,
   sem saber qual é qual; o acerto deve ser menor nas cegas. Se não for, o sinal não serve e a
   ideia para aqui.
3. **Portão de assertividade** da classificação: sessões rotuladas às cegas (explicado,
   supervisionado, cego), com holdout selado.
4. **Desempenho**: custo por captura e tempo de montagem da lente com `XEMNAS_DEMO_SCALE`.

## Plano

1. Classificador de engajamento sobre as capturas existentes e o portão (só backend).
2. Agregado por componente e a previsão falsificável no dogfood: se não houver correlação,
   parar.
3. Lente no Mapa e linha na Visão.
4. Sinal para o agente no `file_context`.
5. Tour de 3 minutos.

## Ligação com o Procurador

Os dois tratam da mesma fronteira, o que a pessoa decidiu e entende versus o que o agente fez
sozinho. O Procurador reduz as interrupções onde já existe decisão; o mapa mostra onde não
existe nenhuma e onde o agente deveria perguntar ou explicar mais. Uma zona cega pode deixar o
Procurador mais conservador: ali ele responde menos e encaminha mais.
