# Ideias de produto em discussão

**Status:** Aberta. A aba Contexto já foi entregue; o resto segue em discussão.

Anotações de 30/09/2026, depois do merge das fases 3 a 5. Nada aqui é escopo
aprovado; é material para decidir depois do dogfood (`docs/operacao/dogfood-log.md`). A
visão de longo prazo continua em `docs/produto/visao-consolidada-do-produto.md` e a
pesquisa de provedores de IA em `docs/pesquisas/provedores-e-modelos-de-ia.md`.

## 1. Capturar e revisar: o modelo está certo?

**O que funciona.** A autoridade humana (nada vira decisão sem confirmação), a
evidência anexada e o filtro de relevância que corta o trivial antes de gastar
atenção. Isso é o que diferencia o xemnas de um "resumo automático de sessão".

**Os riscos.**

- **Fadiga de fila.** Uma Inbox tende a virar tarefa, como notificação. Decisões
  boas são raras e caras; se a fila encher de candidatos medianos, a pessoa para
  de abrir. `noise.dismissed_ratio` e `review_time_ms` já medem isso.
- **Distância do momento.** A revisão acontece longe de quando a escolha foi
  feita; o contexto mental já se perdeu e confirmar vira adivinhação.
- **Memória que ninguém lê.** O valor aparece quando o contexto volta para o
  agente ou para a pessoa. Captura sem uso é só arquivo.
- **Começo vazio.** Um projeto novo no xemnas começa sem nada, mesmo que já tenha
  ADRs, `AGENTS.md` e anos de decisões.

**Ajustes que eu faria, sem trocar o modelo.**

1. **Revisar por sessão, não por item.** Agrupar candidatos pela sessão do agente
   ("Sessão de ontem, 14h: 2 decisões, 1 premissa") com triagem rápida e "aceitar
   com ajuste". Deixa a revisão curta e dá o contexto de onde saiu.
2. **Mostrar conflito na revisão.** Quando um candidato contradiz uma decisão em
   vigor, dizer isso no próprio candidato ("contradiz D:ab12cd34") e oferecer
   **Substituir**. É onde a memória mais protege o projeto.
3. **Confirmar no momento, opcionalmente.** Mais tarde, o agente poderia propor a
   decisão na própria conversa (ferramenta MCP de escrita que só cria candidato) e
   a pessoa confirma ali. Continua exigindo confirmação humana; pede ADR porque o
   MCP hoje é só leitura.
4. **Importar o que já existe.** Semear a memória a partir de ADRs, `AGENTS.md`,
   `CLAUDE.md` e docs do repositório, como candidatos com evidência apontando para
   o arquivo. Resolve o começo vazio.

## 2. Aba de contexto do projeto

**Implementada em 30/09/2026** (`40ead8a`): modo do agente, decisões em vigor,
regras do projeto e prévia do Context Pack. "Mudou recentemente" e "Para
reconsiderar" ficaram de fora e seguem como ideia. O raciocínio original:

Sim, acho útil, e talvez seja a tela que mais mostra o valor do produto. Hoje o
contexto está espalhado entre Decisões, claims (sem tela) e o Context Pack (sem
tela). Uma aba **Contexto** responderia "o que vale neste projeto agora?".

Conteúdo proposto, todo com dados que o backend já tem:

- **Em vigor**: decisões confirmadas não substituídas, as mais recentes primeiro,
  com escolha em uma linha e referência `D:xxxx`.
- **Regras do projeto**: premissas, restrições, objetivos e convenções válidas
  hoje (`Claims::list` na data atual), agrupadas por tipo, com criar e encerrar.
- **Mudou recentemente**: confirmadas, substituídas e claims encerradas nas
  últimas semanas (composto das listas; uma consulta de linha do tempo dedicada
  fica para depois).
- **Para reconsiderar**: decisões com "reconsiderar quando" preenchido.
- **O que o agente recebe**: modo de contexto do projeto (desligado, medir,
  ativo), quanto foi injetado e uma prévia do bloco para uma tarefa digitada
  (o Context Pack). Transparência sobre o que a IA vê.

A aba vira o lugar natural do modo de contexto por projeto e das claims, que
hoje não têm interface.

## 3. Outras ideias úteis

- **Exportar um resumo do projeto para os agentes.** Gerar, por ação explícita,
  uma seção de `AGENTS.md`/`CLAUDE.md` com decisões em vigor e convenções. Serve
  aos agentes que não usam MCP e respeita a regra de nunca escrever no repositório
  sozinho: a pessoa escolhe o destino e revisa o preview.
- **"Por que isto é assim?" a partir de um arquivo.** Escolher um caminho e ver as
  decisões que o afetam. Depende do grafo de entidades da Fase 4
  (`docs/roadmap/fase-4/00-plano-grafo-de-entidades.md`).
- **Medir se o contexto foi usado.** Contar quando o agente cita uma referência
  `D:xxxx` ou abre uma decisão pelo MCP. Mostra se a memória está pagando o custo.
- **Resumo semanal.** O que foi decidido, o que mudou e o que está pendente, para
  quem volta ao projeto depois de dias.
- **Aviso de reconsideração.** Quando uma captura toca algo citado no "reconsiderar
  quando" de uma decisão (dependência, arquivo, tecnologia), avisar na revisão.

## 4. Ordem sugerida

1. ~~Aba **Contexto**~~ (feita).
2. Revisão agrupada por sessão e aviso de conflito na revisão.
3. Importar ADRs/`AGENTS.md` existentes.
4. Exportar resumo para `AGENTS.md`.
5. Proposta de decisão pela conversa (MCP de escrita), depois de ADR.

## 5. Direções anotadas em 02/10/2026 (para o futuro)

Duas ideias da conversa de 02/10, guardadas aqui para decidir depois. A opinião e
a proposta de medição estão em
[medir-eficacia-do-contexto.md](medir-eficacia-do-contexto.md).

1. **Controlar agentes autônomos e um assistente de decisão entre projetos.** No
   futuro o xemnas se integra aos agentes para controlá-los como agentes autônomos
   (como orquestradores do tipo Symphony/Codex da OpenAI). O assistente serviria à
   tomada de decisão, apoiado nas decisões e contextos de **todos os projetos já
   feitos**, para aproveitar o que eles têm em comum quando um projeto novo
   começa. Cuidados já apontados: níveis de autonomia por risco (a Revisão vira o
   portão do que passa do limite), condições de aplicabilidade em cada decisão
   (para não transferir conselho fora de contexto), fronteiras de
   confidencialidade entre projetos e custo (busca local primeiro).
2. **Medir se o contexto injetado ajuda na implementação.** Mecanismo para saber
   se a injeção tem efeito na implementação do agente e qual. Proposta: escada de
   evidência (exposição, absorção, adesão, retrabalho, resultado), canários,
   holdout em `shadow`, repetição offline e painel de Eficácia em Contexto. É
   pré-requisito da ideia 1.

Também anotado: **filtrar a documentação** que entra na Revisão (feito em
`documents::digest`: só partes centrais; ver ADR-0008). Falta aprender com o que a
pessoa dispensa e mostrar, na aba Contexto, quais documentos entram e por quê.

