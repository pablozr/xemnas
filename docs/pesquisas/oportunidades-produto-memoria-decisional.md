# Oportunidades de produto: tornar a memória decisional útil no momento da mudança

**Data:** 02/10/2026.
**Pergunta:** quais ideias ampliam o valor do Xemnas, a partir do produto atual e
de uma pesquisa profunda de produtos relacionados, com diferenciação defensável?
**Status:** Aberta. Pesquisa e recomendações; não é escopo aprovado nem evidência
de demanda validada. Nenhuma funcionalidade proposta foi implementada nesta nota.

## Recomendação

Construir o Xemnas como **memória das condições que tornam uma decisão válida**.
A pergunta central passa de “o que decidimos?” para “essa escolha ainda faz
sentido, o que muda se a revisarmos e qual contexto o agente precisa agora?”.

Eu priorizaria um conjunto coerente: **Radar de premissas**, **Ensaio de mudança**
e **Retomada orientada à tarefa**. Um laboratório pequeno de contexto acompanha
esses experimentos para descobrir se de fato melhoram o trabalho. São propostas
de evolução do mesmo produto, não quatro aplicativos ou novas abas obrigatórias.

O mercado já oferece memória persistente, grafos temporais, regras extraídas de
PRs, contexto por MCP, citações e avaliações de agentes. Nenhum desses atributos
isoladamente demonstra novidade. A hipótese de diferenciação é combinar escolha
humana explícita, condições de validade, evidência de mudança e resultado observado
numa ferramenta local, útil ao desenvolvedor que alterna projetos e agentes.
Não encontramos evidência suficiente para afirmar exclusividade mundial.

## Evidências usadas e método

- [Memória e contexto para agentes](produtos-memoria-agentes-fontes.md): capacidades,
  alternativas e limites verificados em documentação dos fornecedores/projetos.
- [Inteligência de engenharia](produtos-inteligencia-engenharia-fontes.md): revisão,
  documentação viva, arquitetura, risco e aprendizado com o trabalho.
- Fontes adicionais de contexto, avaliação e observação estão nesta nota, com
  links diretos. Datas e branches públicos podem mudar após a consulta.
- Estado local conferido na base `0ecf8596b19ed2e6865a058e25566eb77cabb16b`, incluindo
  código de aplicação, contratos, MCP e ADRs. Declaração documental de um recurso
  externo não equivale a teste independente nem prova de qualidade no Xemnas.
- Não houve instalação de concorrentes, contratação, entrevistas, acesso a contas
  privadas ou execução de testes de mercado. “Oportunidade” significa hipótese
  testável. As situações de uso abaixo são exemplos, não incidentes do usuário.

### O que o Xemnas já tem — evitar propor novamente como novidade

| Base confirmada | Evidência local | O que ainda é uma extensão |
| --- | --- | --- |
| Captura, revisão humana, versões e premissas/condições de reconsideração | `crates/application/src/decisions.rs`: `DecisionEdits`, `DecisionRevision`, `reconsider_when` | Condições observáveis estruturadas e acompanhamento de resultados |
| Context Packs por tarefa/arquivo e validade temporal | `crates/application/src/context.rs`: `build_pack`, `as_of`, orçamento | Escopo por revisão Git e comparação de cenários; não assumir que `as_of` reproduz todo estado histórico de texto |
| Injeção com modos, orçamento, omissões e deduplicação por sessão | `crates/application/src/injection.rs`: `ContextInjection`, `InjectionRecord` | Saber se a entrega foi recebida/usada e se afetou qualidade |
| MCP de consulta | `apps/mcp-server/src/protocol.rs`: `get_decision`, `search_context`, `file_context` | Novos casos de uso ou escrita de propostas exigem contrato/ADR |
| Documentos de projeto podem gerar candidatos | `crates/application/src/documents.rs`: `Documents::propose`; ADR-0008 | Biblioteca global aplicada a uma decisão e experiências com fontes |
| Mapa, relações, impacto e visão com fontes | `crates/application/src/graph/query.rs`, `overview.rs` | Explicar impacto de uma mudança hipotética, preservando diferença entre relação real e inferência |
| Revisão consultiva sob demanda, checks locais e cobertura explícita | ADR-0009; `knowledge_review.rs`: `inspect`, `review`, `ReviewReport` | Monitoramento, persistência de relatórios ou detectar violações no código não fazem parte desse ADR |
| Diagnóstico de latência, ruído e fila | `crates/storage-sqlite/src/diagnostics.rs`: `REVIEW_SQL` | Tempo ativo de revisão e resultado do uso; `review_time_ms` mede espera até confirmação, não minutos efetivos de atenção |

O diário de [dogfood](../operacao/dogfood-log.md) ainda não contém uma semana
preenchida. Não temos base para concluir qual dor domina o uso real ou que alguém
pagará por essas propostas. O início recomendado é testar em tarefas reais do
próprio Xemnas, antes de ampliar integrações e superfícies.

## Leitura do mercado

| Família | Exemplos pesquisados | Valor já ofertado | Consequência para o Xemnas |
| --- | --- | --- | --- |
| Memória de agentes | Mem0/OpenMemory, Zep/Graphiti, Letta, Supermemory, Hindsight | Persistir, recuperar, atualizar e organizar memória | Não competir apenas com “lembra entre sessões” |
| Contexto de engenharia | Unblocked, Augment, Pieces, Sourcegraph/Amp | Conectar código, histórico e conhecimento ao trabalho | Arquivos + chat + MCP já são um terreno concorrido |
| Regras e revisão | Qodo, CodeRabbit, CodeScene | Convenções, achados, risco e aprendizado de feedback | Não criar mais um bot que comenta toda mudança |
| Conhecimento/arquitetura | Swimm, Structurizr, Eraser, ferramentas ADR | Documentação, modelos, fontes e navegação | Um grafo bonito e ADRs gerados não bastam como promessa |
| Avaliação e observação | Tessl, LangSmith, Kosli, Roadie, LaunchDarkly | Medir comportamento, evidência, checks e resultados | Reutilizar conceitos; focar a decisão do projeto em vez de construir uma plataforma genérica |

Capacidades e ressalvas individuais estão nas notas de fontes; produtos agrupados
não são equivalentes em todos os eixos. Ausência de um recurso numa página não
prova ausência no produto.

### Concorrentes e referências que mudam a estratégia

**Unblocked é concorrência próxima.** Seu
[context engine](https://getunblocked.com/blog/inside-the-unblocked-context-engine/)
descreve resolução de conflitos entre fontes, recência, expertise, citações e
regras por repositório. A distinção possível do Xemnas é tornar aprovação,
premissas e reconsideração objetos explícitos escolhidos pelo usuário, em vez de
vender “explicamos o porquê do código” como território vazio.

**Augment** [combina código, histórico e outras fontes](https://www.augmentcode.com/context-engine)
para selecionar contexto. Seus números de economia são resultados publicados
pelo fornecedor e não foram reproduzidos. O Xemnas precisa demonstrar melhoria
em tarefas e decisões específicas; contar documentos recuperados não mede isso.

**Tessl** documenta [avaliação com e sem arquivos de contexto](https://tessl.io/registry/tessl-master/tessl-master/files/docs/evaluate/evaluating-your-codebase.md)
e comparação com contexto atualizado em cenários históricos. Portanto, avaliar
contexto também não é novidade isolada. O recorte proposto aqui é identificar
qual decisão, regra ou versão ajuda, atrapalha ou está fora do escopo do projeto.

**Kosli** [modela trilhas e evidências de verificações](https://docs.kosli.com/understand_kosli/glossary);
**Roadie** [separa fontes, fatos e checks](https://roadie.io/docs/tech-insights/introduction/);
**LaunchDarkly** [usa métricas e condições de amostragem para detectar regressões](https://launchdarkly.com/docs/eu-docs/home/releases/regression-detection).
Inspiram a ligação escolha → condição → observação. Isso não exige transformar
o Xemnas em controle de deploy, compliance ou monitor de produção.

## Dez ideias de produto

Cada cartão contém dor, experiência, diferença para o que existe, experimento e
risco. “Novo” significa novo em relação à base local inspecionada, não patenteável
ou inexistente em outros produtos. Esforço é avaliação relativa, não prazo.

### 1. Radar de premissas — saber quando é hora de reconsiderar

**Cena:** “Escolhemos X porque só existe um processo gravador.” Uma proposta de
mudança introduz outro escritor. O Xemnas apresenta a premissa original, a nova
evidência e uma pergunta: “Essa condição ainda vale neste escopo?”. Não anuncia
automaticamente que a arquitetura está errada.

**Produto mínimo:** o usuário escolhe uma condição de reconsideração já escrita,
associa um arquivo/manifesto ou observação manual e define quando avisar. O primeiro
detector identifica alterações na fonte, com hash/versão e trecho; mudança no
arquivo é sinal para conferir, não prova de que a premissa ficou falsa. Cada
alerta permite explicar, adiar, marcar como irrelevante ou abrir revisão humana.

**Diferença útil:** ligar mudança à justificativa que sustentava a escolha.
Estende `reconsider_when` e a revisão consultiva; não repete uma regra genérica
de lint nem confunde hipótese de IA com violação determinística. Roadie e
LaunchDarkly inspiram observações explícitas; o centro aqui é a premissa decisional.

**Experimento:** 10 decisões reais com condições observáveis, acompanhadas durante
20 mudanças relevantes; revisão humana de todos os alertas e de mudanças não
alertadas. Medir utilidade, falsos positivos/negativos e tempo gasto triando.
Gate inicial proposto: ao menos 8 de 10 alertas julgados úteis e nenhum alerta
repetido sem nova evidência. Amostra exploratória, sem validade estatística ampla.

**Risco/custo:** fadiga de alertas, condições subjetivas e ausência de telemetria.
Começar sob demanda/ao abrir o app, com eventos capturados; monitor contínuo e
novas fontes exigem escopo explícito. Esforço médio; prioridade máxima.

### 2. Ensaio de mudança — “se eu mudar esta escolha, o que preciso rever?”

**Cena:** antes de trocar armazenamento ou mecanismo de autenticação, a pessoa
rascunha uma alternativa. O app mostra decisões dependentes, regras derivadas,
premissas afetadas e perguntas ainda sem resposta, com caminhos no grafo.

**Produto mínimo:** escolher uma decisão confirmada e descrever sua substituta
hipotética. Uma análise efêmera separa: dependências registradas; possíveis tensões
inferidas com citações; informação não encontrada. O usuário pode gerar um
rascunho para revisão; nada muda o conjunto de decisões em vigor nesse ensaio.

**Diferença útil:** navegar a mudança da intenção antes de implementar. O impacto
atual do grafo já fornece parte da base; a novidade local é comparar cenários e
explicar a propagação. Não é simulação de runtime nem garantia de impacto completo.

**Experimento:** cinco mudanças passadas e cinco propostas reais. Um mantenedor
lista previamente os impactos conhecidos; avaliar cobertura, falsas dependências
e se o ensaio revelou uma pergunta acionável antes de editar. Gate: caminhos
registrados sempre verificáveis, inferências rotuladas e nenhum dado confirmado
modificado. Ganho de descoberta ainda precisa de observação com usuários.

**Risco/custo:** o grafo é incompleto; “não encontrado” não significa “não afeta”.
Limitar escopo, chamadas e tamanho; não reconstruir todo o mapa a cada frase.
Esforço médio/alto; segunda prioridade após o Radar.

### 3. Retomada orientada à tarefa — voltar entendendo o que mudou

**Cena:** após duas semanas em outro projeto, “vou mexer na fila de jobs”. O app
mostra só mudanças de decisões/regras relevantes desde a última visita, o que
continua válido e os pontos ainda abertos, com acesso às fontes.

**Produto mínimo:** escolher tarefa e marco anterior; compor um delta citável de
versões e relações confirmadas usando o Context Pack e a Visão atuais. O marco
tem identidade explícita; não afirmar que o usuário leu tudo por abrir a tela.

**Diferença útil:** um retorno acionável por tarefa, mais específico que resumo
semanal ou resumo de conversa. É principalmente uma composição de capacidades
existentes; novidade de mercado baixa, chance de utilidade imediata maior.

**Experimento:** comparar retomada atual e delta em cinco tarefas, alternando
ordem e usando tarefas diferentes para reduzir aprendizado. Medir tempo para
explicar corretamente restrições/fontes e perguntas que precisaram ser refeitas.
Meta exploratória: redução de tempo sem aumentar erros factuais.

**Risco/custo:** viés de recência pode ocultar regra antiga importante. Separar
“mudou” de “continua valendo”. Esforço baixo/médio; melhor aposta de entrega curta.

### 4. Laboratório de contexto — descobrir qual memória realmente ajuda

**Cena:** uma regra ocupa tokens em toda tarefa, mas nunca melhora o resultado;
ou a versão antiga de uma decisão faz o agente insistir numa arquitetura superada.
O laboratório compara variantes e aponta o item relevante, não só uma nota global.

**Produto mínimo:** exportar conjuntos versionados de contexto para um harness
opt-in. Comparar regras estáticas, contexto atual e contexto com um item removido
ou atualizado, num mesmo snapshot de tarefa. Mostrar testes, violações avaliadas
por humano, correções, tokens e latência separadamente. Não rodar código gerado
automaticamente no projeto de trabalho.

**Base externa:** Tessl e [Unblocked Compare](https://github.com/unblocked/unblocked-compare)
já fazem comparações; [LangSmith](https://www.langchain.com/resources/agent-evals)
discute avaliação de execução, trajetória e conversa. A adaptação é acompanhar
o efeito de decisões humanas versionadas no projeto e usar resultados para sugerir
curadoria; nunca excluir uma regra válida porque piorou uma métrica isolada.

**Experimento:** 12 tarefas históricas, três condições, três repetições quando o
orçamento permitir; avaliador cego à condição, mesmos modelos/ferramentas/limites.
Separar calibração de tarefas reservadas. Falhas/timeouts contam e são reportados,
não removidos para melhorar a média. Uma correlação em poucas execuções não prova
que uma decisão causou o ganho. Começar pequeno e expandir antes de alegar eficácia.

**Risco/custo:** caro, não determinístico e sujeito a vazamento da solução futura.
Construir primeiro como instrumento de dogfood, não como plataforma de evals.
Esforço médio; necessário para provar as outras apostas.

### 5. Memória das alternativas — não rediscutir sem saber o que mudou

**Cena:** o agente volta a sugerir uma tecnologia já descartada. O Xemnas lembra
o motivo e sua validade: “foi descartada por X, que ainda vale; vale reabrir se Y”.
Se a condição mudou, a alternativa reaparece como possibilidade, não proibição.

**Produto mínimo:** capturar opções explicitamente discutidas, evidência de
descarte, motivo e condição de reabertura, com confirmação humana. Não tratar
candidato rejeitado por ruído como alternativa arquitetural rejeitada.

**Referência:** [MADR já registra opções e argumentos](https://adr.github.io/madr/).
A evolução proposta é recuperar essas alternativas no momento certo e reabrir
por mudança de premissa. O modelo atual possui rationale/consequências; opções
estruturadas seriam extensão, sem inventar o que não foi registrado.

**Experimento:** dez sugestões recorrentes em histórico; comparar contexto com e
sem a alternativa e verificar se evita repetição sem impedir uma escolha válida
em cenário novo. Gate: cada motivo tem fonte e não há veto automático ao agente.
Esforço médio; prioridade posterior ao Radar, com o qual compartilha o mecanismo.

### 6. Contexto por branch — separar experimento de decisão em vigor

**Cena:** dois agentes exploram soluções incompatíveis em worktrees diferentes.
Uma hipótese de uma branch não deve aparecer como regra geral para o outro.
Ao integrar o código, a pessoa revisa também o que foi aprendido e decidido.

**Produto mínimo:** associar capturas a revisão Git/estado do worktree e declarar
o alcance das propostas. Comparar intenção entre ramos sem fazer merge Git.
Promoção de uma decisão para o projeto exige ação humana, com origem preservada.

**Base/gap local:** `ProjectRef` usa caminho canônico e `CaptureSource` identifica
sessão/mensagem. Metadados de artefato são abertos, mas não existe nesses contratos
um escopo decisional tipado por branch/ref. Isso não comprova um bug atual;
identifica trabalho de modelagem que a ideia exigiria.

**Experimento:** três cenários de agentes paralelos com escolhas conflitantes,
renomeação de branch e worktree sujo. Gate: nenhum contexto experimental vaza
como confirmado; nome da branch não é identidade estável, usar SHA e fingerprint
quando necessário. Citações preservam revisão e cobertura.

**Risco/custo:** mistura de projeto lógico com checkout, promoção ambígua e
explosão de estados. Esforço alto; aposta diferenciadora para uma segunda fase,
não requisito para provar o valor inicial.

### 7. Biblioteca aplicada — transformar leitura em uma decisão melhor

**Cena:** ao avaliar fila/event sourcing, o usuário seleciona duas fontes da
biblioteca. O assistente relaciona os argumentos às restrições reais do projeto,
mostra conflitos de aplicabilidade e sugere qual experimento reduziria a dúvida.

**Produto mínimo:** uma decisão em análise, duas fontes citáveis e uma tabela de
hipóteses/argumentos aplicáveis. O usuário escolhe um experimento ou cria candidato;
o livro nunca vira regra vigente por ter sido recuperado.

**Diferença útil:** estender a biblioteca já prevista para fechar leitura →
hipótese → teste → decisão. Chat genérico com PDF é insuficiente como diferencial.
Não exige iniciar pela biblioteca inteira ou OCR de toda a máquina.

**Experimento:** cinco escolhas reais com critérios anotados antes da consulta;
verificar citações, pertinência e se a resposta altera uma pergunta/teste útil.
Gate: toda recomendação declara condições e lacunas. Esforço alto se parsing,
OCR e recuperação ainda estiverem pendentes; usar fontes textuais pequenas primeiro.

### 8. Recibo de contexto — explicar o que estava disponível ao agente

**Cena:** uma implementação seguiu uma regra antiga. A pessoa consegue distinguir
“a regra nova não foi selecionada”, “ficou fora do orçamento”, “foi entregue” e
“houve evidência de consulta/citação”, em vez de culpar genericamente o modelo.

**Produto mínimo:** enriquecer a auditoria atual com versão da política, seleção,
omissões justificadas, fingerprints e confirmação de recebimento onde o adapter
permitir. Um snapshot opt-in do bloco redigido permite comparação posterior.

**Diferença útil:** fechar a ligação entre memória e ação. A auditoria atual já
guarda IDs/versões, orçamento e sessão, então não é um recurso partindo do zero.
Rastreamento e evidência existem em LangSmith/Kosli; o recorte é a decisão do Xemnas.
Entrega registrada não prova leitura, atenção, obediência ou raciocínio interno.

**Experimento:** cinco falhas preparadas: decisão ausente, versão desatualizada,
omissão por orçamento, timeout e contexto adequado com saída incorreta. Verificar
se o usuário identifica a etapa com evidência. Retenção/consentimento são parte
do contrato: hoje o prompt da requisição não é armazenado; não mudar isso às escondidas.
Esforço médio; habilitador do laboratório e suporte, não necessariamente nova tela.

### 9. Perguntas que faltam — capturar a lacuna, não fabricar a resposta

**Cena:** uma mudança relevante toca autenticação e contrato público, mas o
projeto não registra por que escolheu certo comportamento. O app oferece uma
pergunta pequena e contextual para o humano, com a evidência do que motivou a dúvida.

**Produto mínimo:** ao revisar candidato significativo, mostrar no máximo uma
lacuna útil: critério ausente, consequência não explicitada ou evidência insuficiente.
“Não encontrei justificativa nas fontes consultadas” evita inventar ausência total.

**Diferença útil:** priorizar o próximo conhecimento a registrar, não aumentar a
quantidade de extrações. Aproveita significância/feedback já existentes no ADR-0006.

**Experimento:** 20 revisões, com registro de pergunta respondida, descartada e
se a resposta alterou entendimento/decisão. Gate exploratório: maioria julgada
necessária e custo de atenção menor que o benefício relatado. Nunca criar tarefa
obrigatória para completar a fila. Esforço baixo/médio; complemento do Radar.

### 10. Resultado da decisão — aprender com o que aconteceu depois

**Cena:** escolhemos cache para reduzir latência; semanas depois, o app aproxima
o objetivo registrado de um benchmark anexado. O usuário conclui se o resultado
foi observado, ficou inconclusivo ou exige reconsideração.

**Produto mínimo:** objetivo e observação opcional anexados à versão da decisão;
data, ambiente e método explícitos. Pode começar com benchmark/manual, sem um
conector de observabilidade. Consequência esperada não vira resultado medido.

**Diferença útil:** memória de engenharia que aprende com resultados; combina
o encadeamento de consequências dos [ADRs](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions)
com evidência observada. Não estima causalidade só porque métrica mudou depois.

**Experimento:** acompanhar cinco decisões com objetivo verificável; perguntar
se o registro ajudou a manter, revisar ou explicar a escolha. Não avaliar pessoas
por “taxa de decisões erradas”. Esforço médio; extensão natural do Radar quando
houver decisões e observações suficientes.

## Escolha e sequência

Avaliação qualitativa minha, baseada na proximidade do código e nos riscos
observados. Não são notas de mercado ou demanda comprovada.

| Ideia | Valor provável para o propósito | Diferenciação como combinação | Esforço | Escolha |
| --- | --- | --- | --- | --- |
| Radar de premissas | Alto | Alta, a validar | Médio | Aposta principal |
| Ensaio de mudança | Alto | Alta, a validar | Médio/alto | Segunda aposta |
| Retomada por tarefa | Alto e frequente | Baixa/média | Baixo/médio | Primeiro ganho curto |
| Laboratório de contexto | Alto para validar eficácia | Média | Médio | Instrumento transversal |
| Memória das alternativas | Médio/alto | Média | Médio | Depois do Radar |
| Contexto por branch | Alto no uso multiagente | Alta, a validar | Alto | Horizonte seguinte |
| Biblioteca aplicada | Alto para decisões difíceis | Média | Alto | Piloto textual restrito |
| Recibo de contexto | Alto para diagnóstico/confiança | Média | Médio | Evoluir auditoria existente |
| Perguntas que faltam | Médio | Média | Baixo/médio | Piloto dentro da Revisão |
| Resultado da decisão | Alto no longo prazo | Média/alta | Médio | Após obter observações |

**Primeiro ciclo:** demonstrar Retomada com dados atuais e avaliar 10 premissas
manualmente. Antes de criar detector, verificar se os avisos seriam úteis.
**Segundo ciclo:** Radar restrito a duas classes de fonte, deduplicação e
explicação. Acrescentar recibos mínimos para entender falhas de contexto.
**Terceiro ciclo:** Ensaio de mudança e um conjunto pequeno de tarefas comparáveis.
Só depois investir em branch-aware, integrações de métricas ou biblioteca ampla.

O melhor pacote demonstrável é: “o projeto mudou; esta premissa merece revisão;
estas decisões dependem dela; a pessoa escolheu uma nova direção; o próximo agente
recebe a versão válida”. Isso une as capacidades atuais e as propostas num percurso.

## Condições para construir sem desviar o produto

- Núcleo local, operação básica offline e máquina de 8 GB/CPU continuam sendo
  restrições de projeto; sugestões de modelo pesado não comprovam viabilidade.
- O adapter tem timeout padrão de contexto de 300 ms. Análises profundas
  precomputam/rodam sob demanda; não entram obrigatoriamente no caminho do turno.
- Agente observa/propõe, humano confirma; branch experimental, hipótese e fonte
  de biblioteca têm autoridade distinta de decisão em vigor.
- Relatórios do ADR-0009 continuam efêmeros e sob demanda até decisão explícita
  em contrário. Persistência, monitoramento e novos jobs precisam de novo contrato.
- Novos casos de uso pertencem ao `application`; UI não calcula regras. Front e
  backend seguem sessões/branches separadas. Nenhum novo módulo implica nova aba.
- Não exigir renderizar o grafo inteiro: explorar escopos e caminhos relevantes,
  seguindo a [pesquisa de escala](escalabilidade-renderizacao-fontes.md).
- Não coletar prompt, código, métrica ou conversa extra só para provar valor sem
  escolha e retenção explícitas. Não fazer upload da memória local automaticamente.

## O que evitar agora

Outro chatbot genérico sobre o repositório, um feed de centenas de observações,
geração automática de ADRs tratados como aprovados, uma plataforma completa de
review de PR, score opaco de “saúde arquitetural”, ranking de pessoas e um
marketplace de memórias antes de provar utilidade individual. Esses caminhos
adicionam custos e/ou competem diretamente em capacidades maduras no mercado.

## Como provar valor e quando abandonar uma ideia

**Métrica principal proposta:** tarefas em que uma decisão válida ajudou a evitar
retrabalho ou esclarecer uma escolha, com evidência verificável e confirmação do
usuário. “Ajudou” deve registrar o caso e seu critério; não criar contador automático
de bugs evitados. Observar também o custo ativo de revisar/confirmar/dispensar.

Separar: contexto calculado, enviado, recebido, aberto/citado e saída compatível.
Nenhuma etapa implica automaticamente a seguinte. Menos tokens com pior resultado
é regressão. Mais decisões gravadas pode ser apenas mais ruído.

Entrevistas propostas com cinco desenvolvedores que alternam projetos/agentes:
pedir o último caso real de regra esquecida, decisão repetida ou contexto errado;
reconstituir evidências, frequência, workaround e custo. Mostrar exemplos só depois
de ouvir o caso. Comparar cada aposta ao processo atual com README/AGENTS/MCP,
não a um agente artificialmente privado de toda documentação.

Abandonar/reduzir Radar se quase todos os alertas forem irrelevantes; Ensaio se
apenas repetir relações óbvias; Retomada se a seleção omitir restrições essenciais;
Laboratório como produto se o custo de preparar tarefas exceder o ganho. Investir
em branch-aware só depois de observar contaminação de contexto em uso paralelo.
Os limiares dos cartões são critérios exploratórios propostos, revisáveis antes
do piloto, não resultados obtidos.

## Limitações e entrega

Pesquisa documental, inspeção do código e síntese criativa. Concorrentes não foram
testados lado a lado e nenhuma hipótese de demanda, novidade exclusiva, economia
ou melhoria de qualidade está validada. Os experimentos definidos aqui ainda não
foram executados. Preços e tamanho de mercado não foram estimados por falta de
necessidade/evidência para escolher a proposta inicial.

Ao escolher e implementar uma direção, levar o que for duradouro para produto,
arquitetura/ADR e padrões visuais pertinentes, e remover esta pesquisa e fontes
exclusivas conforme as regras de `docs/pesquisas/`. Esta nota não muda o MVP.
