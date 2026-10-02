# Produtos de inteligência de engenharia: fontes e oportunidades

**Data da pesquisa:** 2026-10-02.
**Pergunta:** quais produtos já conectam código, conhecimento, decisões e aprendizado,
e onde Xemnas pode entregar valor adicional sem virar outro revisor de PRs?
**Status:** Aberta. Pesquisa documental concluída; hipóteses ainda sem validação com usuários.

## Método e limites

Leitura de documentação oficial, repositório mantido pelo autor e anúncios dos
próprios produtos. Os dez produtos abaixo cobrem busca em código, memória de
review, documentação vinculada ao código, arquitetura e aprendizado operacional.
As páginas foram abertas durante a pesquisa; recursos não foram testados em contas
pagas. Uma afirmação do fabricante prova o posicionamento ou comportamento
documentado, não precisão, adoção ou benefício mensurado independentemente.
Não se conclui exclusividade a partir da ausência de um recurso nas páginas lidas.

O recorte do Xemnas vem de [CONTEXT](../produto/CONTEXT.md),
[visão consolidada](../produto/visao-consolidada-do-produto.md) e
[ADR-0009](../arquitetura/adr/0009-revisao-consultiva-de-conhecimento.md).
Engineering Decisions são confirmadas por humanos; Assessments não são fatos.
A revisão vigente é consultiva, efêmera e sob demanda, sem jobs nem detecção
automática de violações no código. Todas as ampliações abaixo são propostas.

## Comparação baseada nas fontes

| Produto | Comportamento documentado | Aprendizado para Xemnas | Limite da inferência |
| --- | --- | --- | --- |
| CodeScene | Distribuição de conhecimento e ilhas de conhecimento derivadas do histórico de contribuição; risco quando autores saem | Encontrar onde falta contexto antes de mudar um módulo | Histórico de autoria é um sinal indireto de familiaridade, não comprovação do que alguém sabe |
| Sourcegraph Deep Search | Investigação com busca e navegação, resposta com buscas executadas e arquivos lidos | Expor como uma resposta foi construída e o escopo consultado | Busca citada não confirma uma escolha humana nem prova cobertura total |
| Qodo | Regras estruturadas, origem, escopo, exemplos, aplicação em revisão e métricas | Medir utilidade e conflito das orientações ao longo do tempo | Governança e mineração de regras já são uma categoria concorrida |
| CodeRabbit | Learnings provenientes de feedback, escopo, administração e estatísticas de uso | Memória precisa de proprietário, correção e rastreabilidade de uso | Preferência aprendida e decisão confirmada têm autoridades diferentes |
| Swimm | Documentação acoplada ao código e verificação/sincronização de snippets | Diferenciar evidência atualizada de rationale que precisa de revisão | Atualizar trecho não prova que a justificativa continua válida |
| Eraser | Diagramas gerados de repositório e monitoramento para atualização | Mapa deve servir uma pergunta e acompanhar mudanças relevantes | Diagrama atualizado não constitui modelo decisional aprovado |
| Structurizr | Um modelo, múltiplos diagramas, documentação e ADRs navegáveis | Perspectivas coerentes sobre os mesmos objetos | Múltiplas vistas e ADRs no grafo não são novidade |
| Ilograph | Walkthroughs que selecionam, expandem e reduzem detalhes | Ensinar um sistema por percurso, não por grafo completo | Narrativa visual não valida o conteúdo narrado |
| Log4brains | ADRs Markdown versionados, busca, timeline e publicação | Portabilidade e histórico são requisitos básicos de confiança | Um catálogo de ADRs sozinho oferece pouca diferenciação |
| incident.io | Rascunho e revisão de post-mortem usando dados do incidente | Fechar o ciclo entre expectativa, resultado e reconsideração | Dados do incidente ajudam, mas causalidade exige análise |

### 1. CodeScene: priorizar ignorância relevante

A documentação calcula conhecimento pelo histórico de contribuição, reconhece
vieses e distingue módulos dominados por uma pessoa, módulos complexos cujos
autores saíram e familiaridade distribuída. Permite simular perda de conhecimento
e mapear CODEOWNERS. [Documentação de Knowledge Distribution](https://docs.enterprise.codescene.io/latest/guides/social/knowledge-distribution.html).

**Hipótese:** um mapa de lacunas decisionais pode destacar módulos frequentemente
alterados cujas decisões não têm rationale ou evidência localizada. Isso é uma
medida sobre os registros disponíveis, não uma nota sobre desenvolvedores.
Entregar uma pergunta útil — “por que essa exceção existe?” — vale mais que criar
outro ranking de risco sem explicação.

### 2. Sourcegraph: tornar investigação inspecionável

Deep Search usa busca e navegação em um ciclo de investigação, devolvendo lista
das buscas e arquivos usados. Aceita escopo por contexto/repositório e permite
consultas por MCP/API. A documentação diferencia processamento na instância das
chamadas externas ao modelo; implantação própria não significa inferência toda
local. [Documentação de Deep Search](https://sourcegraph.com/docs/deep-search).

**Hipótese:** um Context Pack deve trazer recibo legível: versões, escopo,
seleção e omissões. “Por que este item entrou?” e “o que falta consultar?” podem
ser recursos de primeira classe. Não confundir recuperação relevante com prova
de que todas as decisões pertinentes foram incluídas.

### 3. Qodo: concorrente direto de regras e governança

Qodo descreve regras com categoria, severidade, escopo, origem e exemplos. Extrai
orientações de arquivos e ADRs, aplica seleção por escopo de forma determinística
e mede conformidade, violações e merges com violações abertas. O artigo separa
reconciliação futura por generalização do comportamento atual de manter ou
descartar candidatos sem reescrever regras vigentes. Isso evita interpretar um
plano como recurso entregue. [Rules Lifecycle System](https://www.qodo.ai/blog/how-qodo-builds-the-wisdom-to-govern-part-2-the-rules-lifecycle-system/).

**Hipótese:** a oportunidade de Xemnas precisa ir além de converter conversas em
regras. Uma decisão tem premissas, alternativas rejeitadas, trade-offs aceitos e
condições de reconsideração; nem tudo vira uma política executável em PR.
Preservar uma exceção legítima e explicar sua validade temporal é mais alinhado
ao domínio que competir pela quantidade de alertas gerados.

### 4. CodeRabbit: a memória também precisa de manutenção

Os Learnings têm escopo local/global, origem, edição, exclusão, exportação e
contagem de uso. Há aprovação configurável para aprendizados vindos de chat,
com aprovação automática após o prazo. Na modalidade SaaS, aprendizados de PR
aberto ficam limitados à origem até merge. A própria documentação recomenda
manutenção de aprendizados obsoletos e reconhece conflitos com outras instruções.
[Learnings](https://docs.coderabbit.ai/knowledge-base/learnings).

**Hipótese:** mostrar onde uma decisão foi usada e permitir contestação específica
do contexto entregue. Uso frequente não significa correção; rejeição também não
prova que a decisão esteja errada. Xemnas deve manter confirmação explícita e
histórico, sem copiar aprovação por silêncio nem apagar escolhas substituídas.

### 5. Swimm: sincronizar evidência não basta

O relato oficial de uso interno explica documentação junto ao código, Verify
Check, aprovação automática de sincronizações simples e associação de snippets
a design docs. É evidência histórica de funcionamento descrito pelo fabricante,
não garantia dos planos comerciais atuais. [How Swimm uses Swimm](https://swimm.io/blog/how-swimm-uses-swimm).

**Hipótese:** quando o código vinculado muda, manter dois estados separados:
“referência relocalizada” e “rationale revalidado”. Um rename pode preservar ambos;
uma troca no protocolo de entrega pode preservar o link e invalidar a premissa.
O resultado inicialmente deve ser pedido de revisão citável.

**Limitação:** a abertura das páginas atuais de contexto para agentes e do artigo
detalhado de Auto-sync falhou. Seus resumos de busca não fundamentam afirmações
adicionais nesta comparação; oferta comercial atual de Swimm precisa confirmar.

### 6. Eraser: mapas dirigidos por pergunta

Eraser documenta geração de diagramas a partir de código usando prompt e arquivos
relevantes, além de Eraserbot para acompanhar mudanças. Conectar repositório,
monitorar e sincronizar de volta têm suportes diferentes por plataforma.
[Codebase diagrams](https://docs.eraser.io/codebase-diagrams) e
[conexões Git](https://docs.eraser.io/connecting-git-repositories).

**Hipótese:** Xemnas pode mostrar “por que o fluxo de ingestão tem essas fronteiras?”
com decisões e evidências, em vez de tentar desenhar tudo. A seleção do subgrafo
precisa ser visível e reproduzível; expandir demanda explícita, não complexidade
visual permanente.

### 7. Structurizr: modelo coerente, vistas diferentes

Structurizr oferece múltiplos diagramas a partir de um modelo, perspectivas,
documentação e ADRs. O visualizador navega dos elementos para níveis de detalhe,
documentos ou decisões. [Features](https://docs.structurizr.com/features) e
[Diagram viewer](https://docs.structurizr.com/server/diagrams/viewer).

**Hipótese:** alternar Grafo, Blocos e linha do tempo deve preservar seleção,
escopo e identidade. A visão de arquitetura observada e a de intenção aprovada
podem coexistir, mas suas relações precisam de tipos diferentes e proveniência.

### 8. Ilograph: transformar grafo em aprendizado

Walkthroughs suportam slides com texto, seleção, expansão, destaque, contexto,
ocultação e nível de detalhe. A navegação conserva um recurso selecionado ao
mudar de perspectiva. [Walkthroughs](https://www.ilograph.com/docs/editing/walkthroughs/)
e [navegação de diagramas](https://www.ilograph.com/docs/getting-started/browsing-diagrams/).

**Hipótese:** uma “visita guiada pelas decisões deste módulo” apresenta problema,
restrição, escolha e consequência, com parada em cada evidência. A inovação a
testar está em gerar o roteiro da memória confirmada e da tarefa de quem aprende,
não no mecanismo visual de walkthrough, que já existe.

### 9. Log4brains: preservar saída e revisão humana

O projeto publica ADRs Markdown de Git como site estático, com preview, CLI,
busca, timeline e escopo global/por pacote. A lista “Coming soon” do README
não foi contada como funcionalidade entregue. [Repositório oficial](https://github.com/thomvaill/log4brains).

**Hipótese:** importar ADRs como material revisável e exportar decisões com
identidade/versão pode reduzir custo de adoção. Não presumir que um Markdown
importado foi confirmado no modelo do Xemnas. Nenhuma importação deve destruir
status, datas, alternativas ou relações de substituição existentes.

### 10. incident.io: conectar decisão ao que aconteceu

O editor de post-mortems usa conversas, timeline, PRs, campos e investigações
para gerar rascunhos, enriquecer seções e revisar precisão contra dados do
incidente. Isso fundamenta aprendizado ancorado em evidência, não a afirmação
de que a IA encontra a causa correta. [Anúncio do editor](https://incident.io/blog/post-mortems-launch).

**Hipótese:** após um incidente, perguntar quais premissas de decisões anteriores
foram contrariadas e quais permanecem desconhecidas. Produzir candidatos de
reconsideração, nunca registrar automaticamente “a decisão causou o incidente”.
Importar primeiro um post-mortem local explícito evita exigir integração SaaS
para validar o valor.

## Cuidado de recência: Amp

O anúncio de Handoff, de outubro de 2025, defendia transferências focadas entre
threads. Um anúncio posterior, **Amp, Rebuilt**, descreve a volta da compactação
automática e a retirada do handoff como estratégia central. Logo, copiar o
fluxo antigo como referência atual seria erro. A lição estável é que o contexto
de trabalho muda e precisa conservar o que importa à próxima ação.
[Handoff histórico](https://ampcode.com/news/handoff) e
[Amp, Rebuilt](https://ampcode.com/news/neo).

**Hipótese:** Xemnas deve entregar Context Packs independentes do mecanismo de
compactação do agente, com validade e autoridade próprias. O resumo de uma
conversa continua sendo Source Artifact, não Engineering Decision.

## Oportunidades combinadas para testar

Estas propostas são síntese desta pesquisa, não recursos atuais dos concorrentes
nem reivindicações de novidade absoluta.

| Proposta | Situação concreta e entrega | Experimento inicial | Sinal de valor / motivo para abandonar |
| --- | --- | --- | --- |
| Contratos de premissas | “SQLite basta enquanto a carga permanecer X”; evidência nova abre reconsideração | Registrar manualmente premissas e condições em 5 decisões e revisar um evento real | Usuário encontra uma reconsideração relevante; abandonar automação se só gerar alertas vagos |
| Mudança explicada | Comparar dois Context Snapshots: o que mudou, fonte e consequências declaradas | Replay de decisões reais antes/depois de uma substituição | Pessoa recupera contexto mais rápido e distingue proposta de decisão; falha se inventar causalidade |
| Onboarding por tarefa | “Vou mexer em captura”: percurso curto de decisões, alternativas e arquivos | 3 tarefas reais, comparar pacote guiado com README/busca | Menos perguntas repetidas, acerto em perguntas de compreensão; falha se pacote induzir confiança indevida |
| Rationale em risco | Código referenciado mudou, mas justificativa não foi revalidada | Comparar alterações históricas com decisões vinculadas manualmente | Precisão dos pedidos de revisão e tempo de triagem; falha se todo diff virar alerta |
| Revisão pós-resultado | Incidente ou benchmark confronta expectativas de uma decisão | Importar 2 relatórios reais e relacionar evidências a premissas | Usuário identifica aprendizado reutilizável; falha se virar resumo genérico de post-mortem |
| Recibo de contexto | Explicar o que o agente recebeu, por quê e de qual versão | Exportar pacote manual e colher marcação de itens úteis/ausentes | Rastrear orientação obsoleta e reduzir itens irrelevantes; falha se custo de leitura superar o ganho |

### Prioridade recomendada

Começar com **mudança explicada + Context Pack com recibo + onboarding por tarefa**:
trabalham com a memória decisional já central ao produto e permitem demonstrar
utilidade com ações explícitas. A seguir, testar contratos de premissas usando
entrada manual. Detecção contínua de deriva em código e integrações de incidentes
têm dependências maiores e exigem novo escopo/ADR.

As propostas persistentes não devem ser escondidas dentro de `KnowledgeReviewApi`
como se fossem detalhes de implementação: ADR-0009 limita a revisão a relatórios
efêmeros e sob demanda. Jobs, novos registros de resultado, novas relações ou
monitoramento precisam de decisão de produto e domínio próprias.

## Como avaliar sem confundir atividade com valor

### Contra-argumentos e colisões competitivas

- **Qodo é a colisão mais forte:** regras com origem, escopo e retorno de uso já
  existem. “Memória viva de regras” não basta como posicionamento. O teste deve
  incluir decisões que não são regras: aceitar um custo, adiar uma migração,
  preservar uma exceção ou reabrir uma escolha após mudança de premissa.
- **Swimm e Eraser cobrem sincronização com código:** dizer “documentação que não
  envelhece” mistura atualização de representação com validade de intenção.
  Xemnas precisa mostrar exatamente essa diferença em um caso concreto.
- **CodeRabbit cobre aprendizado de feedback:** curadoria manual pode parecer
  custo extra. O benefício da confirmação precisa compensar esse custo com menos
  contexto incorreto; contar confirmações feitas não prova benefício.
- **Sourcegraph cobre investigação citada:** “pergunte ao código” é insuficiente.
  A pergunta diferenciadora é temporal e decisional: “o que foi aceito naquela
  versão e por que reconsiderar agora?”. Sem decisões registradas, Xemnas tem
  menos material e deve admitir isso, não reconstruir intenção como certeza.
- **Onboarding guiado pode ser só apresentação:** comparar com uma página curta
  escrita pelo autor, não apenas com um grafo enorme. Se a página vencer, manter
  a página como entrega e usar o grafo só onde ajudar.
- **Premissas podem custar demais para manter:** começar com condições simples e
  eventos fornecidos pelo usuário. Não exigir instrumentação operacional ampla
  antes de descobrir se as pessoas retornam ao recurso.

Dois desdobramentos para avaliação posterior: um **ensaio de mudança proposta**
que enumere decisões possivelmente afetadas antes de executar um plano, sem
chamar associação de violação; e **contexto por branch/ref**, para não tratar
uma decisão experimental como vigente no projeto inteiro. Ambos requerem modelo
explícito de versão e autoridade; a presente pesquisa não comprovou exclusividade
dessas ideias no mercado.

### Protocolo inicial

1. Montar conjunto pequeno de decisões reais com evidências, uma substituição,
   uma contradição legítima e pelo menos um caso sem resposta conhecida.
2. Comparar tarefa feita com documentação/busca convencional e com proposta,
   mantendo os mesmos dados e perguntas. Alternar a ordem para reduzir aprendizado.
3. Medir tempo, precisão factual, reconhecimento de incerteza, utilidade avaliada
   pelo usuário e custo de corrigir o sistema; registrar também omissões.
4. Separar diagnóstico sobre fonte desatualizada de inferência semântica sobre
   premissa inválida. O primeiro pode ser determinístico; o segundo exige evidência.
5. Evitar estrelas, volume de nós, número de aprendizados ou quantidade de alertas
   como prova de sucesso. Nenhuma página de fornecedor valida a demanda específica
   de Xemnas; entrevistas e dogfood continuam necessários.
