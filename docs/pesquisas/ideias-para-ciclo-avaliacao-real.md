# Ideias práticas para o próximo ciclo de avaliação real

**Data:** 03/10/2026.
**Pergunta:** quais melhorias pequenas e experimentos adicionais podem atacar os problemas da avaliação real, sem repetir as propostas de expansão anteriores?
**Status:** Aberta. Pesquisa documental e desenho de experimentos; nenhuma hipótese foi executada, nenhuma implementação ou escolha de stack/modelo foi feita. Não altera o escopo aprovado.

## Síntese

A recomendação é testar **sete incrementos do percurso existente**, começando por
preservação de ressalvas, interpretação dos estados vazios e qualidade da seleção.
O objetivo verificável é recuperar conhecimento pertinente com seu alcance correto
e medir o trabalho necessário para mantê-lo. Não há promessa de produtividade.

As propostas detalham partes da [síntese de melhorias](melhorias-apos-avaliacao-real.md);
não são sete novos recursos de expansão. O ganho adicional desta pesquisa é definir
unidades menores de mudança, controles negativos, denominadores e condições para
abandonar cada hipótese. Todas as quantidades e metas dos experimentos abaixo são
**propostas Xemnas**, não resultados publicados pelas fontes externas.

## 1. Base local e revisão da avaliação inteira

Foram lidos os [mapas de documentação](../README.md) e [pesquisas](README.md), a
[avaliação consolidada](../operacao/avaliacao-repositorio-real.md), seus cinco
relatos de [onboarding](../operacao/avaliacao-repositorio-real/critic-onboarding.md),
[evidência](../operacao/avaliacao-repositorio-real/critic-evidence.md),
[recuperação](../operacao/avaliacao-repositorio-real/critic-retrieval.md),
[arquitetura](../operacao/avaliacao-repositorio-real/critic-architecture.md) e
[operações](../operacao/avaliacao-repositorio-real/critic-operations.md), além da
[auditoria estática](../operacao/avaliacao-repositorio-real/core/core-audit.md),
[retomada release](../operacao/avaliacao-repositorio-real/core/retomada-release.md),
[núcleo instrumentado](../operacao/avaliacao-repositorio-real/core/nucleo-instrumentado.md),
[relações finais pela GUI](../operacao/avaliacao-repositorio-real/core/relacoes-gui-final.md),
[controle ADR](../operacao/avaliacao-repositorio-real/core/adr-control-real.md) e
[fixture arquitetural](../operacao/avaliacao-repositorio-real/core/fixture-arquitetural-pendente.md).
Esta é uma revisão dos relatos e evidências descritas; não uma nova auditoria dos
JSONs, imagens, código, banco ou ambiente da rodada.

Também foram lidas as [dez oportunidades anteriores](oportunidades-produto-memoria-decisional.md),
as [vinte ideias sobre dores](20-ideias-dores-reais-devs.md), o
[vocabulário vigente](../produto/CONTEXT.md) e o
[ADR-0009](../arquitetura/adr/0009-revisao-consultiva-de-conhecimento.md).
Fonte externa fundamenta um princípio; **hipótese Xemnas** descreve nossa adaptação.
Uma lacuna estática não é um incidente reproduzido.

| Área revista | O que a evidência local permite concluir | Consequência para este ciclo |
| --- | --- | --- |
| Captura, adoção e recuperação | Extração arquitetural explícita, adoção com vínculos, replay idêntico e retry de job Failed passaram em cenários estreitos; o resumo perdeu a ressalva de simulação. [Retomada](../operacao/avaliacao-repositorio-real/core/retomada-release.md) | Preservar os casos positivos e investigar fidelidade; não declarar a extração indisponível. Ideias 1, 3 e 7. |
| Busca, contexto e MCP | Houve falso positivo sobre cor do terminal, miss em inglês, entrega real citada e `file_context` útil após vínculo manual. [Avaliação, recuperação e fatos](../operacao/avaliacao-repositorio-real.md#fatos-observados-e-hipóteses-que-ainda-precisam-de-prova) | Avaliar negativos, idiomas, escopo e cobertura por arquivo separadamente. Ideias 2, 4 e 5. |
| Revisão, exportação e manutenção | Regras opostas foram enviadas isoladamente; regras parcialmente sobrepostas coexistiram; exportações específicas passaram. O controle por exportação respondeu corretamente sem provar economia de esforço. [Núcleo](../operacao/avaliacao-repositorio-real/core/nucleo-instrumentado.md), [controle ADR](../operacao/avaliacao-repositorio-real/core/adr-control-real.md) | Não chamar zero findings de consistência. Usar controles de conteúdo e custo de curadoria. Ideias 1 e 7; seleção de pares permanece na síntese anterior. |
| Mapa, relações e Visão | O inventário inicial não explicava responsabilidades; os retestes confirmaram/rejeitaram relações e abriram entidades a partir do fluxo. A Visão melhorou com decisões confirmadas. [Relações finais](../operacao/avaliacao-repositorio-real/core/relacoes-gui-final.md) | Não reabrir relações como ausentes nem exigir outro grafo. Reparar cobertura localizada. Ideia 4. |
| Primeiro uso, UI e diagnóstico | Fila vazia era ambígua; toast de regra, sobreposição e atualização tardia foram observados. Instalação limpa não foi demonstrada. [Avaliação, recomendações](../operacao/avaliacao-repositorio-real.md#recomendações-prioritárias-e-critérios) | Começar por estado com próxima ação e correções já identificadas; pacote release continua precisando de avaliação própria. Ideia 6. |
| Integridade, tempo e envio externo | Adoção parcial, replay divergente, checkpoint, `as_of` e redação de sugestões são lacunas estáticas; purga deixou dois jobs concluídos. [Auditoria](../operacao/avaliacao-repositorio-real/core/core-audit.md), [núcleo](../operacao/avaliacao-repositorio-real/core/nucleo-instrumentado.md) | Manter as melhorias 1, 9, 10, 11 e 14 da síntese como trabalho próprio. Ideia 1 ajuda a avaliar; estas sete hipóteses não substituem as correções. |

Os cinco críticos eram agentes do mesmo modelo, não cinco participantes humanos;
bridge de avaliação não comprova instalação de produção. Build bloqueado não é
teste verde. Estas distinções permanecem no
[relatório auditado](../operacao/avaliacao-repositorio-real.md#validação-cobertura-e-limites).

## 2. Fontes primárias efetivamente acessadas

Consulta via `webfetch` em **03/10/2026**. As fontes principais de cada entrada
foram abertas, não apenas encontradas numa busca; o link permanente de F7 foi
extraído da página, como indicado. Páginas dinâmicas refletem a consulta, não um
snapshot arquivado. Não foram usados rankings para escolher fornecedor/modelo.

| Ref. | Fonte e versão/data identificável | Afirmação conferida e limite |
| --- | --- | --- |
| F1 | [SWE-bench — página oficial](https://www.swebench.com/) e [guia de avaliação](https://www.swebench.com/SWE-bench/guides/evaluation/) | O benchmark aplica patches e executa testes de repositórios; o guia distingue instâncias resolvidas, não resolvidas e erros sem relatório. Cache por `run_id`/instância pode reutilizar resultado mesmo com diff diferente. Isso fundamenta identidade de execução e estado final, não mede utilidade da memória decisional. |
| F2 | [SWE-bench Pro — repositório oficial](https://github.com/scaleapi/SWE-bench_Pro-os) e [README V2 consultado](https://raw.githubusercontent.com/scaleapi/SWE-bench_Pro-os/main/v2/README.md) | A versão atual consultada declara V2 com 642 tarefas/11 repositórios, protocolo travado e reavaliação do patch em ambiente novo. Declara controles com patch de referência e patch vazio e revisão de instruções/verificadores. São resultados dos autores, não reproduzidos aqui; V1 e V2 não devem ser misturados. |
| F3 | [Anthropic — Demystifying evals for AI agents](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents), 09/01/2026 | Distingue tarefa, tentativa, trajetória e resultado final; recomenda casos positivos/negativos, ambientes isolados, múltiplas tentativas, leitura de trajetórias e calibração humana de juízes. Avalia-se harness e modelo em conjunto. É orientação de engenharia do fornecedor, não estudo independente do Xemnas. |
| F4 | [Sufficient Context — registro dos autores](https://arxiv.org/abs/2411.06037) e [texto v3](https://arxiv.org/html/2411.06037v3), 23/04/2025 | §§3–5 distinguem informação relacionada de informação suficiente para responder; avaliam abstinência e o compromisso entre cobertura e acurácia das respostas emitidas. Contexto suficiente pode conter resposta incorreta; contexto insuficiente às vezes ajuda. Estudo de QA, não de agentes Rust ou memória decisional. Texto acessado até a conclusão; apêndices não usados nesta síntese. |
| F5 | [Self-RAG — abstract dos autores, v1](https://arxiv.org/abs/2310.11511), 17/10/2023 | O abstract relata risco de recuperar número fixo de passagens indiscriminadamente e descreve recuperação adaptativa. Acesso somente ao abstract; não fundamenta limiar, algoritmo ou melhoria quantitativa no Xemnas. Referência anterior, ainda consultável, não resultado de modelos atuais. |
| F6 | [W3C PROV-DM — recomendação vigente consultada](https://www.w3.org/TR/prov-dm/), 30/04/2013 | Introdução e §2 separam entidades, atividades, agentes responsáveis, derivação e revisão. Proveniência ajuda a avaliar confiança; não certifica a verdade do conteúdo. A leitura retornou conteúdo parcial; as seções citadas estavam disponíveis. Norma conceitual estável, não recomendação de adoção de RDF/ontologia. |
| F7 | [Wikidata — Help:Qualifiers](https://www.wikidata.org/wiki/Help:Qualifiers), revisão exibida de 12/09/2026, [link permanente](https://www.wikidata.org/w/index.php?title=Help:Qualifiers&oldid=2544568641) | Qualificadores podem restringir validade temporal, aplicação e método de determinação de uma afirmação; referências e qualificadores têm papéis diferentes. Página primária do projeto, não estudo sobre síntese por LLM. O link permanente foi fornecido pela página acessada, não aberto separadamente. |
| F8 | [Anthropic — Effective context engineering for AI agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents), 29/09/2025 | Recomenda contexto de alto sinal, referências leves com carregamento sob demanda e divulgação progressiva; reconhece custo de exploração e perda de detalhes na compactação agressiva. Orientação do fornecedor; não prova que encurtar todo contexto melhora toda tarefa. |
| F9 | [Anthropic — Writing effective tools for AI agents](https://www.anthropic.com/engineering/writing-tools-for-agents), 11/09/2025 | Recomenda tarefas realistas reservadas para avaliação, respostas de ferramentas relevantes, descrições inequívocas e erros com correção acionável. Formato ideal varia com tarefa/agente. Não exige novas ferramentas para cada estado nem comprova ganhos transferíveis ao Xemnas. |

**Achados externos que mudam o plano:** resultado persistido deve ser separado de
mensagem de sucesso ([F1](https://www.swebench.com/SWE-bench/guides/evaluation/),
[F3](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents));
relevância não basta para demonstrar suficiência ou verdade
([F4, §3](https://arxiv.org/html/2411.06037v3#S3)); referência à fonte não substitui
qualificadores de alcance ([F6, §2](https://www.w3.org/TR/prov-dm/#section-prov-overview),
[F7](https://www.wikidata.org/wiki/Help:Qualifiers)). A adaptação abaixo é nossa.

## 3. Sete incrementos e seus experimentos

### 1 — Casos contrastivos com identidade de execução

**Problema observado:** a rodada combinou versões de binário, modos Fake/provider
e instrumentos distintos; o relatório precisou corrigir interpretações tardias.
[Base local](../operacao/avaliacao-repositorio-real.md#como-a-avaliação-foi-feita).

**Fonte externa:** F1 distingue erros de resultado e alerta sobre cache; F2
reavalia em ambiente novo e valida referência/controle vazio; F3 recomenda partir
de falhas reais e não confundir trajetória com estado final.
[F1](https://www.swebench.com/SWE-bench/guides/evaluation/),
[F2](https://raw.githubusercontent.com/scaleapi/SWE-bench_Pro-os/main/v2/README.md),
[F3](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents).

**Hipótese Xemnas / incremento:** preparar uma ficha por caso com entrada,
resultado esperado, proibido e fonte verificável; registrar commit/hash do binário,
fixture, modo de extração, configuração de contexto, instrumento e tentativa.
Começar como protocolo documental, não como novo laboratório no app.

**Experimento:** oito casos pareados, total de 16: relevante/irrelevante;
português/inglês; local/projeto; simulado/executado; arquivo ligado/desligado;
descarte/erro; replay idêntico/divergente; conteúdo antes/depois de revisão.
Reproduzir a base antes de mudar uma variável. Para saídas não determinísticas,
planejar três tentativas; resultados bloqueados continuam no relatório.

**Métrica e gate proposto:** 16/16 fichas com oráculo e identidade completos;
apresentar acertos/tentativas por caso, falhas de produto, instrumento e ambiente
separadas. O gate é rastreabilidade, não exigir 16/16 comportamentos aprovados antes
de registrar a base. Um controle que aceita saída sabidamente errada invalida o
verificador, não demonstra qualidade do app.

**Dependências:** fixture pública/sintética, responsável pelo oráculo e identidade
do release. **Riscos/limites:** poucos exemplos podem superajustar; detalhes dos
estados não precisam ser uma sequência obrigatória de ferramentas. Esforço relativo
baixo para o protocolo, variável para falhas injetadas.

**Diferença para pesquisas anteriores:** operacionaliza o laboratório já proposto
e a melhoria 10; não cria uma plataforma de avaliação ou mapa de verificação novo.

### 2 — Admitir ausência sem esconder contexto parcial útil

**Problema observado:** consulta sobre cor do terminal trouxe `Override`; outra
consulta encontrou semântica geral, mas não o motivo de uma mudança específica.
[Base local](../operacao/avaliacao-repositorio-real/critic-retrieval.md).

**Fonte externa:** F4 distingue contexto suficiente de informação apenas relacionada
e mostra que bloquear todo contexto insuficiente pode descartar informação útil;
F5 alerta contra preenchimento indiscriminado com passagens.
[F4, §§3 e 5](https://arxiv.org/html/2411.06037v3#S3),
[F5, abstract](https://arxiv.org/abs/2310.11511).

**Hipótese Xemnas / incremento:** experimentar uma política de admissão para
contexto automático com três resultados: nenhum item pertinente; item pertinente
com lacuna explicitada; item que sustenta a pergunta no escopo registrado. Começar
pela rubrica e sinais atuais, sem um novo juiz no caminho de cada turno. Não
atribuir probabilidade de acerto ao score lexical nem proibir o agente de investigar.

**Experimento:** 30 consultas rotuladas: dez respondíveis, dez com informação
parcial e dez sem conhecimento aplicável. Dividir por famílias de intenção:
18 consultas para calibração e 12 reservadas, quatro de cada classe, sem paráfrases
da mesma intenção cruzando os conjuntos. Comparar seleção atual e política candidata
com o mesmo corpus, orçamento e termos. Dois revisores conferem rótulos antes.

**Métrica e gate proposto:** taxa de consultas negativas que recebem algum item;
precisão dos itens entregues; recall dos itens necessários; cobertura de entrega;
respostas sustentadas/respostas emitidas e extrapolações na condição parcial.
Para zero itens, precisão é indefinida, não 100%. Meta exploratória nos reservados:
zero contaminações nas quatro negativas e nenhuma nova perda nas quatro positivas;
reportar as parciais individualmente. Amostra mínima não estima risco de produção.

**Dependências:** rótulos e orçamento fixos; rubrica de escopo. **Riscos/limites:**
abstinência excessiva pode esconder restrições; suficiência não prova verdade; uma
pergunta de implementação pode precisar de código além da memória. Esforço baixo
para avaliar e médio se o contrato de retorno precisar mudar.

**Diferença:** refinamento da melhoria 3 e do recibo anterior: avaliar pertinência
e lacuna separadamente, sem novo chat nem escolha de mecanismo de busca.

### 3 — Escolha e ressalva como unidade de entrega

**Problema observado:** o resumo e regras derivadas omitiram “simulação local”,
embora a evidência preservasse o aviso.
[Base local](../operacao/avaliacao-repositorio-real/core/relacoes-gui-final.md).

**Fonte externa:** F6 separa responsabilidade e derivação; F7 mostra que aplicação,
tempo e método qualificam afirmações; F8 reconhece perda de detalhes na compactação.
[F6, §2](https://www.w3.org/TR/prov-dm/#section-prov-overview),
[F7](https://www.wikidata.org/wiki/Help:Qualifiers),
[F8](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents).

**Hipótese Xemnas / incremento:** testar uma unidade mínima indivisível contendo
escolha, autoria/autoridade conhecida, alcance, ressalva de validação e referência
à versão. O Context Pack inclui a unidade ou declara sua omissão; não entrega só
a escolha cortando a ressalva por orçamento. Fonte/autoria desconhecida permanece
desconhecida. Protótipo documental pode reutilizar texto/campos existentes; esquema
persistente novo exige contrato, não está decidido aqui.

**Experimento:** seis decisões sintéticas, cobrindo escolha local, simulação,
confirmação pelo projeto, teste bloqueado, teste executado e fonte sem autoria.
Examinar extração, edição, sugestão de regra, exportação e Context Pack com três
orçamentos, inclusive um que não comporte a unidade. Pedir ao receptor para explicar
alcance e validação, sem fornecer a transcrição inteira.

**Métrica e gate proposto:** qualificadores críticos preservados/esperados por
transformação; afirmações de autoridade ou teste sem suporte; unidades omitidas
explicitamente por orçamento; tokens entregues. Meta: nenhuma promoção indevida
de simulação/local para regra do projeto e nenhuma alegação de teste executado
nos exemplos bloqueados, mesmo que seja necessário entregar menos itens.

**Dependências:** identificar ressalvas críticas nas fontes, versão e semântica do
orçamento; correção temporal antes de testar `as_of` como história fiel.
**Riscos/limites:** ressalva também pode ser extraída errada; metadado não prova
verdade. Esforço baixo para ensaio de formato, médio para propagação entre contratos.

**Diferença:** recorte verificável da melhoria 2; não cria contexto por branch,
ontologia PROV ou registro de resultados de decisão.

### 4 — Reparar cobertura por arquivo dentro da adoção

**Problema observado:** `file_context` passou a retornar conhecimento após vínculo
manual; a prévia posterior já sugeria `ignore` e declarava componentes ausentes.
[Base local](../operacao/avaliacao-repositorio-real.md#resultado),
[retomada](../operacao/avaliacao-repositorio-real/core/retomada-release.md).

**Fonte externa:** F8 propõe referências leves e consulta progressiva; F9 recomenda
respostas relevantes e correção acionável para erros de uso.
[F8](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents),
[F9](https://www.anthropic.com/engineering/writing-tools-for-agents).

**Hipótese Xemnas / incremento:** na prévia existente, tornar clara a consequência
dos vínculos para consulta por arquivo. No retorno vazio, distinguir caminho sem
componente, componente sem conhecimento ligado e conhecimento omitido pela política,
quando esses fatos forem verificáveis. Oferecer abrir/revisar a associação existente;
arquivo citado em Evidence é sugestão de vínculo, não confirmação de responsabilidade.

**Experimento:** oito cenários com caminhos exatos: vínculo válido; ausente;
arquivo apenas anexado; dois componentes candidatos; entidade retirada; arquivo
renomeado; caminho fora do Project; omissão por orçamento. Comparar adoção atual e
mensagem/prévia candidata, sem criar associações automaticamente.

**Métrica e gate proposto:** diagnósticos corretos/8, associações indevidas,
ações humanas entre confirmar e obter `file_context` aplicável, respostas positivas
antes/depois. Meta: 8/8 estados explicáveis com Evidence ou limite explícito e zero
vínculos cross-project/ambíguos confirmados pela proposta. Menos ações só conta se
mantiver a associação correta.

**Dependências:** prévia, entidades e consulta atuais; backend expõe causa real.
**Riscos/limites:** Evidence pode citar arquivo incidental; renomeação não autoriza
migrar vínculo silenciosamente. A atomicidade da adoção continua uma correção
separada. Esforço baixo/médio; não exige construir um analisador de código.

**Diferença:** aproxima a adoção já entregue da recuperação já entregue; não repete
mapa comportamento→código, geração de grafo ou descoberta arquitetural automática.

### 5 — Invariância de consulta por idioma e distração

**Problema observado:** `polarity counters` falhou enquanto consultas em português
encontraram a decisão; FTS com OR é um risco estático de correspondência fraca.
[Crítica](../operacao/avaliacao-repositorio-real/critic-retrieval.md),
[auditoria](../operacao/avaliacao-repositorio-real/core/core-audit.md).

**Fonte externa:** F3 recomenda testar quando o comportamento deve e não deve
ocorrer; F9 usa tarefas reservadas para detectar superajuste. Nenhuma dessas fontes
prova que aliases resolvem recuperação bilíngue.
[F3](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents),
[F9](https://www.anthropic.com/engineering/writing-tools-for-agents).

**Hipótese Xemnas / incremento:** medir se uma normalização pequena ou poucas
equivalências confirmadas por Project bastam antes de experimentar nova busca.
Para um mesmo significado, trocar idioma não deveria alterar os itens necessários;
acrescentar “decisão/projeto/terminal” sem mudar intenção não deveria admitir um
item antes irrelevante. Isso é uma propriedade a avaliar, não uma equivalência
linguística garantida. Não traduzir símbolos nem expandir homônimos globalmente.

**Experimento:** dez intenções, cada uma em português, inglês e versão com termo
distrator: 30 consultas, incluindo cinco intenções sem resposta. Reservar quatro
famílias inteiras, duas positivas e duas negativas. Comparar base, normalização
mínima e equivalências curadas, com política da ideia 2 fixa. Sem tradução externa
automática nem novo glossário obrigatório.

**Métrica e gate proposto:** famílias que recuperam o conjunto necessário nas três
variantes; recall por idioma; admissão indevida após distração; p50/p95 de latência;
minutos para curar equivalências. Meta reservada: duas famílias positivas cobertas
nas três variantes e nenhuma entrega nas seis negativas. Se a curadoria exigir
lembrar o termo exato de cada decisão, a hipótese de redução de esforço falhou.

**Dependências:** rótulos de intenção e campos existentes; comparação de identidade,
não de ordem literal. **Riscos/limites:** aliases ampliam falso positivo e envelhecem;
esse ensaio não seleciona modelo, índice ou stack. Esforço baixo no corpus, baixo/médio
na eventual política lexical.

**Diferença:** teste de robustez da melhoria 3, não outro glossário, biblioteca ou
promessa de busca semântica multilíngue.

### 6 — Próxima ação baseada na causa do estado vazio

**Problema observado:** análise `ok` com zero candidatos e categoria `detail` era
válida, mas fila vazia não explicava descarte versus processamento/falha; atualização
tardia confundiu o estado depois de retry.
[Base local](../operacao/avaliacao-repositorio-real.md#fluxo-real-de-sessão-outbox-e-provider),
[retomada](../operacao/avaliacao-repositorio-real/core/retomada-release.md).

**Fonte externa:** F3 distingue sucesso narrado de resultado final; F9 recomenda
retornos de erro específicos e acionáveis. A taxonomia de vazio abaixo é uma
hipótese de UX nossa, não uma recomendação publicada para inboxes.
[F3](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents),
[F9](https://www.anthropic.com/engineering/writing-tools-for-agents).

**Hipótese Xemnas / incremento:** na Revisão/diagnóstico existente, apresentar
último recebimento, estado da análise e uma próxima ação compatível: conferir
conexão se não recebeu; aguardar quando em execução; abrir motivo quando descartou;
reprocessar quando erro permitir; revisar candidato quando existir. Ausência de
recebimento não comprova desconexão. Zero candidato `detail` não pede retry por
padrão. Fake/demo fica explícito; sucesso de regra usa o nome correto.

**Experimento:** seis roteiros públicos/sintéticos: nunca recebeu; aguardando job;
análise em execução; descarte detail; Failed recuperável; concluído com candidato.
Acrescentar a transição Failed→concluído para medir atualização. Primeiro avaliar
as mensagens em cartões documentais com quatro voluntários (24 interpretações,
ordem alternada); depois, se escolhido, fluxo real nas telas. Participação futura,
sem gravar nomes, contas ou sessões pessoais.

**Métrica e gate proposto:** causa e próxima ação identificadas corretamente por
roteiro; retries desnecessários; consultas a logs; atraso entre persistência e
exibição. Meta exploratória: pelo menos 90% de interpretações corretas e nenhum
retry sugerido para detail válido. Reportar pessoas, roteiros e tentativas; agentes
avaliadores não contam como participantes humanos.

**Dependências:** eventos/assessment reais e refresh; estados que o backend não
distingue exigem caso de uso. UI futura segue identidade visual vigente e captura
autorizada. **Riscos/limites:** UI pode ficar desatualizada; mensagem não substitui
prova de integração. Esforço baixo/médio. Instalação limpa continua teste distinto.

**Diferença:** recorte das melhorias 4–6 com decisões acionáveis; não propõe um
novo onboarding, feed de notificações ou painel de observabilidade.

### 7 — Revisão dirigida pelo trecho que sustenta cada campo

**Problema observado:** candidato Fake precisou de reescrita; candidatos reais
vieram claros, mas perderam qualificadores. Não transferir a deficiência da demo
para todos os resultados do provider.
[Crítica inicial](../operacao/avaliacao-repositorio-real/critic-evidence.md),
[retomada real](../operacao/avaliacao-repositorio-real/core/retomada-release.md).

**Fonte externa:** F6 distingue derivação, citação e responsabilidade; F9 recomenda
retornar informação de alto sinal em vez de dumps; F3 recomenda conferir suporte
das afirmações e calibrar julgamento humano.
[F6, §2](https://www.w3.org/TR/prov-dm/#section-prov-overview),
[F9](https://www.anthropic.com/engineering/writing-tools-for-agents),
[F3](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents).

**Hipótese Xemnas / incremento:** testar na revisão existente a aproximação de
pergunta, escolha, motivo e ressalva aos trechos que os sustentam. Destacar campo
sem suporte e mostrar somente o delta editado, com acesso à Evidence completa.
Começar com associação manual no experimento; não presumir que o contrato atual
possui offsets por campo ou que o modelo pode declarar a evidência correta.

**Experimento:** 12 candidatos públicos/sintéticos, quatro corretos, quatro com
ressalva omitida e quatro com inferência sem apoio. Duas condições: revisão atual
e trechos alinhados. Preparar uma variante equivalente de cada candidato para a
segunda condição e contrabalançar a distribuição entre quatro revisores voluntários,
sem reutilizar o mesmo texto para a mesma pessoa. Alternar ordem para evitar
releitura como vantagem; identificar modo Fake/provider em ambas.

**Métrica e gate proposto:** defeitos encontrados/8; alterações corretas/alterações;
minutos ativos, ações e custo de preparar trechos, medidos separadamente de espera
do job. Meta: localizar os quatro erros de autoridade/validação e não piorar
correção dos quatro candidatos bons. Redução de ações isolada não basta. Se preparar
as associações custar mais que revisar texto completo, reduzir ou abandonar o desenho.

**Dependências:** Evidence citável e versões; formatos/offsets exigem contrato se
automatizados. **Riscos/limites:** trecho escolhido pode omitir contraevidência e
ancorar o revisor; manter fonte completa alcançável. Esforço baixo no piloto manual,
médio para associação confiável no produto.

**Diferença:** reduz o trabalho no ponto observado da captura→revisão; não adiciona
quadro de hipóteses, quiz, dossiê de reprodução ou mapa de verificação.

## 4. Sequência e protocolo de decisão

1. **Fixar a base com a ideia 1.** Identidade, rótulos e exemplos reservados entram
   antes do ajuste. Um erro de instrumento não pode virar reprovação do produto.
2. **Ensaiar 3, 6 e 7 documentalmente.** São mudanças localizadas de unidade de
   conteúdo, mensagem e leitura. Corrigir toast/sobreposição/refresh permanece
   trabalho já identificado, sem exigir experimentação de uma nova linguagem visual.
3. **Comparar 2 e 5 uma variável por vez.** Primeiro política de admissão com busca
   fixa; depois formulações/normalização com admissão fixa. Medir recall e negativos
   conjuntamente, não otimizar abstinência até tudo ficar vazio.
4. **Aplicar 4 ao percurso de adoção escolhido.** Prévia e contexto por arquivo
   precisam concordar; não expandir para um mapa completo do código.
5. **Só então comparar tarefa inteira.** Se os gates locais passarem, executar
   tarefas públicas/sintéticas equivalentes nas condições sem memória adicional,
   ADR bem escrito e Xemnas. Todos mantêm acesso à documentação normal do Project.
   Fixar critérios, ferramentas, orçamento e configuração antes; repetir tentativas
   não determinísticas, alternar ordem e preservar falhas/timeouts/bloqueios.

O passo 5 mede aplicação correta de restrições, extrapolações e custo de captura,
revisão, vínculos e recuperação. É extensão do controle anterior, não prova obtida.
Contagem de decisões, confiança do extrator, citação pelo agente e sucesso de
entrega não demonstram redução de esforço total. A separação entre trajetória e
resultado segue [F3](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents);
o compromisso entre respostas emitidas e correção segue
[F4, §5](https://arxiv.org/html/2411.06037v3#S5).

**Registro mínimo futuro por tentativa:** identificador não pessoal do caso,
versão da fixture/política, entrada esperada, itens disponíveis/selecionados/omitidos,
resultado observado, classificação da falha, tempo humano ativo, tempo de máquina,
ações manuais e decisão de seguir/parar. Referências e versões bastam por padrão;
retenção de conteúdo integral não é pressuposta. Um recibo prova entrega, não
leitura ou cumprimento, conforme a distinção já adotada na
[pesquisa anterior](oportunidades-produto-memoria-decisional.md#8-recibo-de-contexto--explicar-o-que-estava-disponível-ao-agente).

## 5. Dependências transversais, riscos e limites

- **Prioridades existentes:** fronteira de envio externo, atomicidade, replay e
  checkpoint, história `as_of`, pares de regras e política de purga continuam na
  [síntese de 17 melhorias](melhorias-apos-avaliacao-real.md). Não foram rebatizadas
  como ideias novas; cada uma exige prova proporcional própria.
- **Contratos:** persistência de qualificadores, diagnósticos de seleção, offsets
  por campo ou identidades novas depende de proposta de contrato. O
  [ADR-0009](../arquitetura/adr/0009-revisao-consultiva-de-conhecimento.md) permanece
  sob demanda/efêmero; este plano não acrescenta jobs ou monitoramento à revisão.
- **Custo operacional:** manter restrições locais de orçamento/latência já descritas
  na [pesquisa anterior](oportunidades-produto-memoria-decisional.md#condições-para-construir-sem-desviar-o-produto).
  Nenhuma ideia depende de escolher stack, embeddings, reranker, fornecedor ou modelo.
  Esforços indicados são avaliações qualitativas, não prazos garantidos.
- **Qualidade da avaliação:** rubricas podem errar; casos reservados não podem virar
  exemplos de ajuste. Dois revisores discordantes devem registrar a dúvida, não
  fabricar um consenso. Controles negativos e estado final são necessários, mas
  não certificam cobertura global ou correção de todas as tarefas.
- **Transferência externa:** SWE-bench/Pro avaliam patches, F4/F5 estudam QA/RAG,
  PROV/Wikidata modelam conhecimento e Anthropic publica práticas próprias.
  Nenhuma fonte demonstra eficácia destas sete adaptações no Xemnas. Datas antigas
  são explicitadas; “consultada hoje” não significa “publicada hoje”.
- **Limite desta entrega:** leitura documental e web pública, sem executar app,
  benchmark, teste, entrevista ou acessar dados pessoais. Não foram implementados
  recursos nem criados tickets. A síntese e os índices foram atualizados na mesma
  entrega documental. Não houve nova execução dos cenários avaliados.

Ao escolher uma hipótese, o resultado do experimento deve atualizar a recomendação:
**seguir** se preservar correção/alcance com custo aceitável; **reduzir** se somente
parte do percurso ajudar; **abandonar** se a curadoria adicional anular o benefício
ou produzir novas extrapolações. Ganho de produtividade, demanda comercial e efeito
causal em grande escala permanecem sem evidência.
