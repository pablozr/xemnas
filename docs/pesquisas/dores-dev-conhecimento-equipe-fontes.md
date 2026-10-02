# Dores de desenvolvedores: conhecimento, equipe e decisões

**Data:** 02/10/2026
**Pergunta:** quais dores empiricamente observadas em onboarding, revisão,
coordenação e conhecimento arquitetural podem originar produtos úteis no Xemnas?
**Status:** Aberta — pesquisa exploratória; propostas não implementadas nem
validadas com usuários do Xemnas.

## Escopo e leitura da evidência

Foram consultados artigos originais, cópias de autores/universidades e páginas de
editoras. Amostra, método e limitações aparecem abaixo; percentuais descrevem os
estudos, não todos os desenvolvedores. Não há experimento que demonstre efeito
do Xemnas. Estudos antigos fundamentam perguntas a testar hoje; ferramentas,
trabalho remoto e agentes podem mudar frequência e forma das dores.

O enquadramento usa [CONTEXT](../produto/CONTEXT.md),
[MVP-SPEC](../produto/MVP-SPEC.md) e as
[dez propostas anteriores](oportunidades-produto-memoria-decisional.md).
O Xemnas preserva decisões humanas com evidências; propostas e fontes não ganham
autoridade automaticamente. As ideias abaixo acrescentam situações de uso,
evitando reapresentar retomada, radar, alternativas ou recibo de contexto.

## Fontes empíricas

### E1. Onboarding inclui orientação social e técnica

Begel e Simon, *Struggles of New College Graduates in their First Software
Development Job*, SIGCSE 2008.
[Publicação institucional](https://www.microsoft.com/en-us/research/publication/struggles-of-new-college-graduates-in-their-first-software-development-job/).

**Método/amostra:** observação no trabalho de oito recém-formados em seus primeiros
seis meses na Microsoft; 85 horas. **Achado:** dificuldades foram classificadas em
comunicação, colaboração, aspectos técnicos, cognição e orientação. Saber programar
não eliminava o problema de se situar na organização e no trabalho.
**Limites:** pequeno estudo qualitativo de iniciantes numa empresa, em 2008;
não estima prevalência atual nem duração média de onboarding. **Gancho:** testar
se entender decisões e interlocutores de uma área ajuda numa primeira tarefa.
É evidência da dor, não de que documentação substitui mentoria.

### E2. Revisar exige entender por que a mudança existe

Bacchelli e Bird, *Expectations, Outcomes, and Challenges of Modern Code Review*,
ICSE 2013.
[Artigo original na universidade do autor](https://www.ifi.uzh.ch/dam/jcr:4d711d23-ce10-4069-8214-eae91797e44b/icse2013.pdf).

**Método/amostra:** 17 desenvolvedores observados/entrevistados em 16 equipes da
Microsoft, 570 comentários classificados, surveys com 165 gestores e 873
programadores. **Achado:** entendimento de código e da mudança foi central;
transferência de conhecimento e consciência da equipe também foram resultados
da revisão. 91% dos programadores respondentes disseram demorar mais ao revisar
arquivos pouco familiares. **Limites:** autorrelato, empresa/ferramenta específicos
e cenário anterior aos agentes; não comparar o percentual diretamente ao tempo
economizado por um produto. **Gancho:** um pacote curto de intenção pode ser mais
útil que outro gerador de comentários de estilo.

### E3. Concentrar contribuição não mede compreensão com precisão

Avelino, Passos, Hora e Valente, *A Novel Approach for Estimating Truck Factors*,
ICPC 2016.
[PDF original dos autores/UFMG](https://homepages.dcc.ufmg.br/~mtov/pub/2016-icpc.pdf),
[registro dos autores](https://arxiv.org/abs/1604.06766).

**Método/amostra:** mineração de autoria em 133 projetos populares GitHub, com
validação via respostas de desenvolvedores de 67 projetos. **Achado:** a heurística
estimou fator de concentração de até dois em 65% dos projetos. Houve concordância
total/parcial sobre autores principais em 84% das respostas válidas, mas sobre
o fator estimado em apenas 53%. **Limites:** histórico de commits é proxy;
heurística, filtros e limiar de cobertura afetam o resultado. Não demonstra que
65% dos projetos parariam após duas saídas. **Gancho:** avaliar transferência
de compreensão por tarefas, em vez de produzir um ranking automático de pessoas.

### E4. Dívida gera custo que os autores percebem durante o trabalho

Besker, Martini e Bosch, *Software developer productivity loss due to technical
debt—A replication and extension study examining developers’ development work*,
Journal of Systems and Software, 2019.
[Manuscrito institucional](https://research.chalmers.se/publication/511450/file/511450_Fulltext.pdf),
[editora](https://www.sciencedirect.com/science/article/abs/pii/S0164121219301335).

**Método/amostra:** 43 desenvolvedores fizeram relatos duas vezes por semana por
sete semanas, com 16 entrevistas de complemento e fase de replicação independente.
**Achado:** média de tempo perdido atribuída à dívida de 23%; testes adicionais
estavam entre as atividades afetadas. **Limites:** estimativa autorrelatada,
atribuição subjetiva, contextos selecionados; não é fração universal nem tempo que
o Xemnas recuperaria. A extração completa do PDF falhou em chamadas posteriores;
método/resumo foram triangulados com trechos públicos do manuscrito e da editora.
**Gancho:** registrar o compromisso que criou a dívida e seus custos observados,
sem transformar TODO em decisão ou calcular juros fictícios.

### E5. Dependências de trabalho atravessam a estrutura do código

Cataldo, Herbsleb e Carley, *Socio-Technical Congruence: A Framework for Assessing
the Impact of Technical and Work Dependencies on Software Development
Productivity*, ESEM 2008.
[Artigo original do autor](https://herbsleb.org/web-pubs/pdfs/cataldo-socio-2008.pdf).

**Método/amostra:** quatro releases, 39 meses, 114 desenvolvedores em oito equipes
e três localidades; 2.375 solicitações de mudança envolvendo múltiplas equipes.
Modelos estatísticos combinaram dependências e registros de coordenação.
**Achado:** congruência entre coordenação e dependências estava associada a menor
tempo de resolução; dependências lógicas por mudanças conjuntas tiveram relação
mais informativa que somente chamadas/dados. **Limites:** um sistema, proxies,
coordenação não observada e possíveis confundidores; associação não prova efeito
causal de avisos ou mensagens. **Gancho:** tornar visíveis acordos de interface,
com confirmação humana; coalteração é sinal, não obrigação comprovada de contato.

### E6. Pessoas priorizam critérios arquiteturais diferentes

*What rationales drive architectural decisions? An empirical inquiry*, manuscrito
depositado pelos autores em 2023.
[Texto original completo](https://arxiv.org/html/2309.14164),
[registro e versões](https://arxiv.org/abs/2309.14164).

**Método/amostra:** questionário aberto com 63 profissionais e entrevistas com
13 deles; recrutamento em feira, conferência e redes dos pesquisadores.
**Achado:** facilidade de desenvolvimento, manutenção, desempenho e experiência
prévia apareceram entre os critérios; prioridades diferiam por experiência.
**Limites:** maioria com menos de quatro anos de experiência, conveniência e
respostas retrospectivas; diferenças não autorizam estereotipar juniores/sêniores.
O depósito não foi tratado como certificação de revisão por pares.
**Gancho:** fazer critérios explicitados por cada participante conversarem,
preservando objetivos e fontes, sem escolher o vencedor pelo cargo.

### E7. Conhecimento arquitetural também se transmite pela prática

Dasanayake et al., *Software Architecture Decision-Making Practices and
Challenges: An Industrial Case Study*, ECSA 2015; cópia depositada em 2016.
[Artigo completo dos autores](https://arxiv.org/pdf/1610.09240),
[registro](https://arxiv.org/abs/1610.09240).

**Método/amostra:** entrevistas presenciais de uma a duas horas com dez
profissionais responsáveis por arquitetura em três PMEs de dois países europeus,
mais documentos internos; coleta em 2014. **Achado:** processos majoritariamente
informais e gestão do conhecimento como tema importante de melhoria;
pair design, revisão, troca de tarefas, conversas e prototipagem compunham a
transferência de conhecimento. **Limites:** três empresas semelhantes,
interpretação qualitativa e contexto antigo; não comprova benefício de uma
ferramenta de armazenamento. **Gancho:** combinar registros com uma explicação
aplicada e verificação humana de entendimento.

### E8. Evidência brasileira recente para decisões informais

Carvalho e Conte, *Software architecture decision-making process: The
practitioners’ view from the Brazilian industry*, Science of Computer Programming,
2025.
[Artigo/página da editora](https://www.sciencedirect.com/science/article/pii/S0167642325000413).

**Método/amostra:** 12 entrevistas semiestruturadas com profissionais que
participam de decisões arquiteturais em empresas diferentes; transcrição e
análise com métodos de Grounded Theory. **Achado:** experiência, PoCs, padrões e
requisitos organizacionais influenciavam escolhas; decisões em grupo eram comuns,
sem abordagem sistemática em muitos casos. **Limites:** relato qualitativo não
representativo da indústria nacional. Foram acessíveis resumo e trechos públicos
das seções da editora; abertura direta posterior falhou, então a análise não
alega leitura integral nem utiliza estatísticas ausentes. **Gancho:** testar um
ritual pequeno de explicitar acordos, sem impor um processo pesado de arquitetura.

## Sete ideias adicionais para a síntese

Propostas do pesquisador, inspiradas nas dores. Cada uma requer investigação com
usuários; as fontes não prescrevem estes recursos. Começar com dados locais e
exportações manuais. Colaboração/sincronização seriam extensões explícitas do
escopo, não funcionalidades existentes.

### A. Primeiro percurso pelo projeto

**Cena:** quem chega ao código precisa realizar a primeira mudança, sem conhecer
as restrições que o mantenedor considera óbvias. **Produto:** um mantenedor escolhe
uma pequena tarefa; o Xemnas monta um percurso de três decisões, evidências e
arquivos e pede ao visitante explicar como a tarefa respeita as restrições.
O mantenedor valida a explicação. **Base:** E1/E7. **Distinção:** onboarding
aplicado e teach-back, diferente de retomada após uma pausa. **Teste:** cinco
primeiras tarefas comparadas ao onboarding atual, com critérios de compreensão
predefinidos, tempo e quantidade de esclarecimentos. **Risco:** trilha rígida,
teste constrangedor e substituir conversa por leitura; participação voluntária,
sem nota pessoal. **Viabilidade:** curadoria manual e exportação local primeiro.

### B. Intenção antes do diff

**Cena:** o reviewer recebe centenas de linhas e não sabe qual comportamento
importa preservar. **Produto:** preparar um cartão anexável ao review com pergunta
resolvida, decisões aplicáveis, invariantes esperadas e evidência da mudança;
autor corrige/confirma antes de exportar. **Base:** E2. **Distinção:** contexto
para compreensão humana antes da revisão, não bot de aprovação nem recibo de
injeção. **Teste:** dez reviews com/sem cartão, observando questões de esclarecimento
e entendimento correto antes de avaliar defeitos. **Risco:** narrativa plausível
que encobre implementação divergente; intenção e evidência devem ser separadas,
sem afirmar conformidade automática. **Viabilidade:** exportação Markdown manual;
capturar PRs/diffs exigiria adapter e política próprios.

### C. Ensaio de sucessão do conhecimento

**Cena:** só o mantenedor sabe explicar por que um módulo opera daquela forma.
**Produto:** selecionar cinco perguntas de manutenção que outro dev tenta responder
com a memória; comparar com evidências e correção do mantenedor. Lacunas viram
um plano voluntário de conversa/pairing, não um score de indivíduo.
**Base:** E3/E7. **Distinção:** provar transferência por entendimento aplicado;
não estimar bus factor por commits. **Teste:** resposta correta, fontes encontradas
e tarefa explicada antes/depois de uma transferência curta. **Risco:** rastrear
competência ou desligamento; registros pertencem ao exercício, com consentimento.
**Viabilidade:** pacote local exportável e avaliação humana; benefícios de equipe
dependem de integração/compartilhamento que hoje são futuros.

### D. Acordo de dívida técnica

**Cena:** um atalho foi aceito para entregar a tempo, mas meses depois só resta
um TODO sem motivo ou compromisso. **Produto:** ligar a decisão ao benefício
obtido, limite aceito, responsabilidade explicitamente escolhida e esforço
observado para conviver com o atalho. Isso subsidia renegociação consciente.
**Base:** E4. **Distinção:** registrar a negociação que autorizou o custo, não
inferir dívida pelo código nem repetir avaliação genérica do resultado da decisão.
**Teste:** recuperar o contexto de dez atalhos e observar se um acordo permite
priorizar/pagar/manter com justificativa melhor que TODOs atuais. **Risco:** promessas
de prazo artificiais e números de juros sem fundamento. **Viabilidade:** campos
opcionais e observações manuais; não implantar telemetria antes de provar utilidade.

### E. Passaporte de interface

**Cena:** backend e front trabalham separados; um contrato muda e alguém só percebe
na integração. **Produto:** associar à decisão de interface consumidores conhecidos,
invariantes, exemplos aceitos e pontos que exigem consulta humana. Exportar um
cartão de acordo para a mudança; pessoas confirmam/revisam escopos voluntariamente.
**Base:** E5. **Distinção:** preparar coordenação sobre uma fronteira real, diferente
de simular impacto ou isolar branches. **Teste:** cinco mudanças de contrato,
comparando falhas de entendimento e dúvidas descobertas antes da integração.
**Risco:** mapa incompleto virar uma falsa lista de todos os afetados; mostrar
cobertura e desconhecidos. **Viabilidade:** relações/consumidores explicitados
manualmente primeiro; notificações e multiusuário exigem novo escopo.

### F. Mesa de critérios

**Cena:** discussão parece “tecnologia A contra B”, mas um dev quer simplicidade,
outro desempenho e outro operar em hardware limitado. **Produto:** participantes
declaram critérios e cenários verificáveis; Xemnas organiza os conflitos de
objetivos com fontes, sem voto automático, score opaco ou escolha por senioridade.
**Base:** E6/E8. **Distinção:** explicitar e negociar prioridades entre pessoas;
alternativas estruturadas são apenas entradas, não a finalidade principal.
**Teste:** cinco discussões reais, avaliando se participantes conseguem reproduzir
os critérios dos outros e identificar um teste ou acordo concreto.
**Risco:** impor racionalidade burocrática; permitir três critérios e uma saída.
**Viabilidade:** sessão local facilitada por humano e exportação antes de colaboração.

### G. Registro de dissenso útil

**Cena:** o grupo decide seguir uma direção, mas uma preocupação válida desaparece
sob o texto “consenso”. **Produto:** pessoa registra objeção, evidência, resposta
da decisão e condição para reabrir a questão; autoridade continua com quem confirma.
Não inferir nomes, sentimentos, concordância ou objeções a partir de silêncio.
**Base:** E6/E7/E8, como inspiração; esses estudos não demonstram eficácia de um
registro de dissenso. **Distinção:** preservar entendimento e compromisso social
ao adotar uma escolha, diferente de registrar tecnologias descartadas.
**Teste:** cinco decisões em grupo, verificando compreensão recíproca e recuperação
de ressalvas após duas semanas. **Risco:** arma política, dados pessoais e histórico
de culpa; registrar somente o necessário e oferecer exportação escolhida.
**Viabilidade:** anotação explícita por decisão primeiro; gestão de equipe fica futura.

## Aplicação e lacunas

As apostas locais mais próximas são o cartão de intenção e o percurso curado de
onboarding. Acordos de dívida também cabem num piloto manual. Passaportes, dissenso
e negociação têm valor potencial, mas seu uso compartilhado precisa ser validado
antes de ampliar o Xemnas para plataforma de equipe.

Nenhuma amostra acima mede demanda pelo produto, disposição a pagar, novidade
exclusiva ou ganho causal. A continuação adequada é pedir episódios concretos,
reconstituir o fluxo atual e testar pequenos artefatos com critérios de compreensão,
sem contadores de “bugs evitados”, produtividade individual ou leitura inferida.
