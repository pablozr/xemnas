# ADR-0009 — Revisão consultiva de conhecimento sob demanda

**Status:** Vigente. Implementada (`application::knowledge_review`, `KnowledgeReviewApi`, em Contexto).

## Decisão

`KnowledgeReviewApi::inspect` captura um inventário próprio e executa apenas
checks determinísticos. `review` captura outro snapshot, executa os mesmos checks
e tenta o provedor vigente consentido. O relatório é efêmero: não há escrita,
jobs, assessments, overview, migrations, embeddings ou aprovação automática.

SQLite captura decisões (texto e versão), claims brutos, relações, entidades,
edges e propostas pendentes sob um lock e transação de leitura. Consultas FTS
retornam somente IDs: conteúdo e versão sempre vêm do snapshot do projeto.
Referências inválidas, tipos desconhecidos e datas corrompidas não são ocultados.

Checks locais: vigente depende de substituída; conflito registrado entre vigentes
(deduplicação simétrica); regra válida derivada de fonte substituída; propostas
pendentes com fonte/endpoint substituído. A validade usa o início do relatório.
Regras não são encerradas; propostas nunca são tomadas como fatos.

## IA, citações e limites

Relações existentes, sobretudo `supersedes` (nova → anterior), formam unidades.
Candidatos vigentes sem relação são recuperados por entidades confirmadas ou FTS,
sem produto cartesiano. Regras relacionadas entram como contexto; regras sem
vínculo são unidades sem escopo universal presumido. `part_of` é confirmado e
limitado a dois saltos; a seleção não é exaustiva e isso aparece em cobertura.

Máximos: 12 chamadas, 4 candidatos por decisão, 24.000 bytes de entrada e saída,
6 findings e 8 citações por finding. O limite de caracteres do perfil também é
respeitado. Unidades grandes são omitidas explicitamente, não cortadas em silêncio.
A ordem é estável. Sem objetivos explícitos não se infere que uma escolha é ruim.

JSON rejeita propriedades desconhecidas. Tipos, IDs e campos são validados contra
a unidade efetivamente enviada. Trechos são substrings Unicode exatas do campo
redigido; offsets UTF-8 são resolvidos no backend. Tensões e substituições citam
ambos os sujeitos. Finding inválido é descartado e contado; parse inválido é
falha de unidade, nunca ausência de problemas.

## Privacidade e autoridade

`redact_secrets` protege campos enviados, explicações e perguntas. Conteúdo é dado
não confiável, não instrução; o prompt explicita proteção contra injection. Nenhum
conteúdo é logado. Perfil ausente não é semeado. Sem credencial/consentimento a IA
fica indisponível e checks permanecem. Perfil e disponibilidade da credencial são
revalidados entre unidades; mudanças interrompem chamadas seguintes.

Cancelamento é cooperativo antes/depois de chamadas: preserva checks e unidades
concluídas, mas não promete interromper HTTP/retries em andamento. Falhas do
provedor preservam cobertura e evidências locais. O relatório não muda decisões,
regras ou relações, não detecta automaticamente violações no código e não substitui
autoridade humana. Ausência de findings não certifica consistência global.
