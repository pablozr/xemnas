# Engineering Decision Intelligence

Este contexto descreve a memória decisional de um projeto de software: como informações vindas do trabalho de humanos e agentes se tornam candidatos revisáveis, decisões confirmadas, regras e contexto entregue ao agente. Os termos abaixo existem no código atual (`crates/domain`, `crates/application`).

## Language

**Project**:
O espaço de trabalho de software cujo contexto, decisões, evidências e evolução são acompanhados. Um worktree do git conta como o Project registrado do mesmo repositório.
_Avoid_: Workspace, repository

**Adapter**:
Uma integração substituível que traduz dados de uma ferramenta externa para Source Artifacts do produto, sem conter regras de decisão. Hoje: o plugin do OpenCode e os hooks do Claude Code.
_Avoid_: Connector core, provider logic

**Source Artifact**:
Um registro imutável ou versionado vindo de uma fonte externa, como turno de agente, diff, commit, arquivo ou documento.
_Avoid_: Context, truth

**Capture Envelope**:
Um pacote versionado e idempotente produzido por um Adapter para transportar Source Artifacts e seus metadados de origem até o produto, sem interpretá-los como decisões. Sem o app aberto, espera na outbox.
_Avoid_: Event, Decision Candidate

**Capture Episode**:
A visão de detalhe de uma captura: coordenadas imutáveis de origem e até cem fatos descritivos dos artefatos. É apresentação, não identidade canônica nem autoridade.
_Avoid_: Task, session

**Evidence**:
Uma referência citável a um Source Artifact que apoia ou contradiz um Context Claim, Decision Candidate ou Engineering Decision.
_Avoid_: Source, proof

**Decision Candidate**:
Uma possível escolha durável (decision) ou regra (rule) inferida do trabalho real, ainda sem autoridade até confirmação. Traz relevância, critérios e Qualifiers. O tipo `detail` nunca é gravado.
_Avoid_: Decision, suggestion

**Decision Inbox**:
O conjunto assíncrono de Decision Candidates aguardando revisão (tela Revisão), sem bloquear o trabalho que os originou. Candidatos de baixa relevância ficam guardados e escondidos por padrão.
_Avoid_: Approval queue, notification feed

**Engineering Decision**:
Uma escolha confirmada, com escopo, rationale, premissas, evidências e condições de reconsideração. Revisar cria nova versão e mantém a anterior.
_Avoid_: ADR, recommendation

**Decision Relation**:
Uma ligação tipada entre duas Engineering Decisions: uma substitui, depende de ou conflita com a outra. A IA só sugere; vale depois de confirmada. Substituir nunca apaga a decisão anterior.
_Avoid_: Link, graph edge

**Context Claim** (claim, regra):
Uma afirmação tipada e temporal sobre o Project (premissa, restrição, objetivo ou convenção), com validade e origem. Nasce de uma regra confirmada, de uma sugestão derivada de uma decisão ou de ação direta. A interface a chama de regra.
_Avoid_: Memory, factoid

**Rule Scope** (escopo de regra):
Os componentes a que uma Context Claim está ligada por arestas `applies_to`. Com ligação, a regra só vale quando a tarefa toca um deles; sem ligação, vale para o Project todo.
_Avoid_: Tag, global rule

**Qualifier**:
Uma ressalva explícita (autoria, alcance, validade) guardada junto do item, que não pode se perder no resumo do motivo. Ausência é desconhecimento, nunca autoridade global.
_Avoid_: Caveat, tag

**Observation**:
Um fato descritivo e verificável lido de um manifest (pacote ou dependência declarada), mantido localmente. Não é Claim, decisão nem aresta humana, e não entra na fila de revisão.
_Avoid_: Claim, fact

**Entity**:
Um componente ou uma tecnologia do Project no grafo, com padrões de caminho e aliases. Aposentar nunca apaga.
_Avoid_: Module, tag

**Edge**:
Uma ligação do grafo entre uma decisão, claim ou Entity e uma Entity (`affects`, `uses`, `applies_to`, `part_of`). Nasce humana e confirmada, ou derivada como sugestão; nunca é apagada, só invalidada.
_Avoid_: Relation, link

**Vínculo estrutural**:
Uma Edge de decisão para componente que nasce de um sinal conferido no repositório, e não só do texto: o arquivo citado que existe, o componente que declara a dependência citada, o que define o símbolo citado, a raiz do workspace ou a CI. Registra quem a confirmou (pessoa, regras, IA ou herança).
_Avoid_: Link automático

**Menção afirmativa**:
Uma ocorrência do nome de um componente no texto de uma decisão que diz algo sobre ele. Não conta o que o texto só cita para excluir ("independente de X", "sem X", "X foi descartado"), o nome do próprio projeto em prosa nem o nome dentro de outro caminho.
_Avoid_: Citação

**Componente fantasma**:
Componente cuja pasta não existe mais (ou nunca existiu) no projeto, em geral nascido de um caminho que um documento cita. O app o aponta; só uma pessoa o aposenta.
_Avoid_: Componente órfão

**Knowledge Graph** (grafo, mapa):
As Entities e Edges de um Project com as consultas sobre elas: mapa, vizinhança, lente de arquivo, impacto, linha do tempo e conflitos.
_Avoid_: Graph database, GraphRAG

**Review Item** (alvo de revisão):
Algo que espera decisão sobre autoridade: candidato, relação sugerida, claim derivada ou vínculo com o mapa. É a unidade do modo automático e de seu registro.
_Avoid_: Task, notification

**Automatic Mode** (modo automático):
Interruptor, por instalação, em que um juiz de IA decide os Review Items em lote, com limites de chamada. Tudo fica no registro "Feito sozinho", com motivo, e descartes podem ser desfeitos. O padrão é manual.
_Avoid_: Autopilot, auto-confirm

**Assessment**:
O registro de uma análise de captura: motivo, classificação e tentativa, gravados juntos. Não constitui fato nem decisão.
_Avoid_: Decision, truth, verdict

**Context Pack**:
Uma seleção pequena, temporária e citável de Decisions, Claims e Observations preparada para uma tarefa ou data, dentro de um orçamento de tamanho.
_Avoid_: Full project dump, permanent prompt

**Injection**:
O bloco compacto de contexto, dentro de um orçamento de tokens, anexado ao turno do agente. O modo é por Project: Desligado, Medir (calcula e registra, não envia) ou Injetar.
_Avoid_: Prompt stuffing

**Agent Query**:
Uma consulta MCP do agente (`get_decision`, `search_context`, `file_context`), registrada só com ferramenta, resultado e tamanho, para medir o uso do contexto.
_Avoid_: Log, telemetry event

**AI Execution Profile**:
Uma escolha configurável de provedor, modelo, limites e política de privacidade usada para extrair candidatos e produzir textos. Consentimento e credencial são validados antes de cada chamada.
_Avoid_: Model, Provider

**AI Provider**:
Uma integração que executa solicitações de IA: heurística local, API compatível com OpenAI, conta ChatGPT, OpenCode Zen/Go ou Claude Code local. Não possui regras do domínio decisional.
_Avoid_: Candidate Extractor
