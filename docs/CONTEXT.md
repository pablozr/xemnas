# Engineering Decision Intelligence

Este contexto descreve a memória decisional de um projeto de software: como informações vindas do trabalho de humanos e agentes se tornam claims verificáveis, decisões confirmadas e contexto recuperável.

## Language

**Project**:
O espaço de trabalho de software cujo contexto, decisões, evidências e evolução são acompanhados.
_Avoid_: Workspace, repository

**Adapter**:
Uma integração substituível que traduz dados de uma ferramenta externa para Source Artifacts do produto, sem conter regras de decisão.
_Avoid_: Connector core, provider logic

**Source Artifact**:
Um registro imutável ou versionado vindo de uma fonte externa, como turno de agente, diff, commit, arquivo, issue ou métrica.
_Avoid_: Context, truth

**Capture Envelope**:
Um pacote versionado e idempotente produzido por um Adapter para transportar Source Artifacts e seus metadados de origem até o produto, sem interpretá-los como decisões.
_Avoid_: Event, Decision Candidate

**Context Claim**:
Uma afirmação tipada e temporal sobre o Project, sustentada por Evidence e sujeita a confirmação, contradição ou expiração.
_Avoid_: Memory, factoid

**Evidence**:
Uma referência citável a um Source Artifact que apoia ou contradiz um Context Claim, Decision Candidate ou Engineering Decision.
_Avoid_: Source, proof

**Decision Candidate**:
Uma possível escolha durável inferida do trabalho real, ainda sem autoridade até confirmação humana.
_Avoid_: Decision, suggestion

**Engineering Decision**:
Uma escolha humana confirmada, com escopo, rationale, premissas, evidências e condições de reconsideração.
_Avoid_: ADR, recommendation

**Assessment**:
Uma análise temporal e reproduzível feita sobre Claims, Evidence e Decisions; não constitui fato nem decisão.
_Avoid_: Decision, truth, verdict

**Context Snapshot**:
Uma visão derivada do que era considerado válido para determinado Project, escopo e instante.
_Avoid_: Project Context file, summary

**Decision Episode**:
Um agrupamento de Source Artifacts que mostra a formação de uma possível decisão; é uma visão de apresentação, não uma identidade canônica.
_Avoid_: Task, session

**Decision Inbox**:
O conjunto assíncrono de Decision Candidates aguardando revisão humana, sem bloquear o trabalho que os originou.
_Avoid_: Approval queue, notification feed

**Knowledge Library**:
Uma coleção global, curada pelo usuário, de materiais de referência reutilizáveis entre Projects, como papers, livros, normas e páginas web. Seu conteúdo informa análises, mas não constitui automaticamente contexto ou verdade de nenhum Project.
_Avoid_: Project Context, source of truth

**Knowledge Source**:
Um documento ou recurso identificável dentro da Knowledge Library, preservando autoria, origem, data, versão e localização citável.
_Avoid_: Evidence, Context Claim

**Project Knowledge Link**:
Uma associação explícita entre um Project e uma Knowledge Source ou coleção considerada relevante para ele; indica disponibilidade para consulta, não concordância nem adoção.
_Avoid_: Imported decision, project fact

**Context Pack**:
Uma seleção pequena, temporária e citável de Decisions, Claims, Evidence e Knowledge Sources preparada para uma tarefa ou pergunta específica.
_Avoid_: Full project dump, permanent prompt

**Engineering Assistant**:
Um papel consultivo que usa Context Packs para analisar alternativas, trade-offs e compatibilidade com o Project, sem possuir autoridade para confirmar Engineering Decisions.
_Avoid_: Decision maker, autonomous architect

**AI Execution Profile**:
Uma escolha configurável de provedor, modelo, capacidades, limites e política de privacidade usada para executar um Assessment ou propor Decision Candidates.
_Avoid_: Model, Provider

**AI Provider**:
Uma integração que executa solicitações de IA diretamente ou por meio de um gateway como OpenCode, sem possuir regras do domínio decisional.
_Avoid_: Engineering Assistant, Candidate Extractor
