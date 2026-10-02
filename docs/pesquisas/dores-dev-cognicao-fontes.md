# Dores de desenvolvedores: cognição, investigação e compreensão

**Data:** 02/10/2026.
**Pergunta:** quais dores observadas em estudos originais de desenvolvimento
podem orientar novas utilidades da memória decisional do Xemnas?
**Status:** Aberta. Pesquisa exploratória; as propostas abaixo não alteram o
escopo aprovado e não foram implementadas ou validadas com usuários do Xemnas.

## Método e leitura da evidência

Busca direcionada a estudos empíricos originais e abertura dos textos completos
em páginas de autores, universidades e arXiv. Foram consultados estudos sobre
interrupções, necessidades de informação, perguntas de manutenção, compreensão
e debugging. É uma seleção orientada ao produto, não uma revisão sistemática:
não houve protocolo pré-registrado, busca exaustiva em bases ou metanálise.

Método, população e limitações acompanham cada fonte. Estudos antigos continuam
úteis para formular hipóteses sobre cognição, mas não estimam automaticamente o
trabalho com agentes em 2026. Uma dor observada tampouco demonstra que o Xemnas
é a melhor solução ou que haverá disposição a pagar. Os resumos são paráfrases.

O enquadramento segue [CONTEXT](../produto/CONTEXT.md),
[MVP-SPEC](../produto/MVP-SPEC.md) e as
[dez oportunidades anteriores](oportunidades-produto-memoria-decisional.md).
Detalhes de investigação são temporários; só escolhas duráveis confirmadas pelo
humano viram Engineering Decisions. As propostas precisam de extensões próprias
de aplicação e contratos, sem deslocar regras de negócio para a UI.

## Estudos originais

### C1 — Retomar envolve recuperar conhecimento, não apenas abrir arquivos

**Parnin e Rugaber, ICPC 2009:**
[Resumption Strategies for Interrupted Programming Tasks](https://chrisparnin.me/pdf/parnin-icpc09.pdf).

**Método/amostra:** análise exploratória de aproximadamente 10 mil sessões
registradas de 85 programadores, usando históricos de interação de IDEs.
**Achado:** a atividade inicial frequentemente inclui navegação e outras formas
de recuperar contexto antes de editar. O estudo distingue a latência até a
primeira edição de outros aspectos da retomada.
**Limite:** os logs não revelam a natureza de cada interrupção; tempo até editar
não equivale a trabalho perdido ou improdutividade. Dados de IDEs de 2005 não
medem diretamente agentes atuais. O PDF é a versão de conferência de 2009;
não confundir com a publicação posterior de título semelhante.
**Implicação:** preservar uma investigação e sua pergunta pode ser mais útil
do que restaurar apenas a lista de arquivos.

### C2 — Pistas concretas podem apoiar a retomada

**Parnin e DeLine, CHI 2010:**
[Evaluating Cues for Resuming Interrupted Programming Tasks](https://chrisparnin.me/pdf/cues-chi09.pdf).

**Método/amostra:** survey com 371 desenvolvedores Microsoft; experimento
intra-participantes com 15 profissionais, três tarefas em jogos C# e condições
contrabalançadas: notas, notas com árvore de interesse e notas com timeline de
conteúdo. A pesquisa externa preliminar de 43 pessoas não é a amostra reportada
nos resultados do survey principal.
**Achado:** as condições com pistas tiveram maior sucesso nas tarefas; os
participantes preferiram a timeline com conteúdo.
**Limite:** tarefas curtas, interrupções artificiais e aviso para preparar a
suspensão. Não transportar o efeito para o Xemnas como ganho prometido.
**Implicação:** mostrar evidência específica de código e raciocínio; avaliar o
benefício contra notas simples antes de gravar mais atividade. O próprio artigo
discute privacidade ao compartilhar timelines.

### C3 — O conhecimento procurado inclui intenção e comportamento esperado

**Ko, DeLine e Venolia, ICSE 2007:**
[Information Needs in Collocated Software Development Teams](https://faculty.washington.edu/ajko/papers/Ko2007InformationNeeds.pdf).

**Método/amostra:** observação de 17 desenvolvedores de uma grande empresa em
sessões de 90 minutos; transcrição e análise das necessidades, fontes e barreiras
de informação, organizadas em 21 tipos.
**Achado:** perguntas de design e comportamento, como justificativa do código,
comportamento pretendido e causa de um estado, podiam ficar pendentes; colegas
indisponíveis eram uma barreira à obtenção de conhecimento.
**Limite:** estudo observacional de uma empresa, em contexto presencial; não
estabelece frequência universal nem causalidade de uma solução.
**Implicação:** ligar comportamento esperado, justificativa e evidência ajuda a
formular consultas que não se resolvem com busca literal de símbolos. Isso
inspira o produto, sem substituir a conversa com pessoas ou inferir intenção
somente a partir do código.

### C4 — A pergunta real se perde entre consultas e resultados parciais

**Sillito, Murphy e De Volder, FSE 2006:**
[Questions Programmers Ask During Software Evolution Tasks](https://www.cs.ubc.ca/~murphy/papers/other/asking-answering-fse06.pdf).

**Método/amostra:** dois estudos qualitativos: nove participantes novos na base,
em 12 sessões em pares sobre ArgoUML, e 16 profissionais em 15 sessões de
mudanças no próprio código. Catalogaram 44 tipos de perguntas.
**Achado:** perguntas amplas exigiam várias consultas menores; juntar resultados
podia provocar perda de orientação, repetição de passos e respostas incompletas.
O estudo também registra busca por precedentes e exemplos relevantes.
**Limite:** pares/think-aloud alteram a atividade; sessões curtas e classificação
interpretativa não demonstram prevalência no universo de devs.
**Implicação:** vincular achados à pergunta que os motivou e guardar caminhos
descartados. Oferecer um recorte útil para a tarefa, em vez de exigir compreender
todo o grafo do projeto.

### C5 — Compreensão atravessa ferramentas

**Xia et al., TSE 2018:**
[Measuring Program Comprehension: A Large-Scale Field Study with Professionals](https://xin-xia.github.io/publication/TSE17.pdf).

**Método/amostra:** campo com 78 profissionais de sete projetos, duas empresas,
3.148 horas de trabalho; coleta de interação entre aplicações e entrevistas com
dez participantes. A classificação identifica atividades relacionadas à
compreensão, inclusive navegador e documentos.
**Achado:** nesse conjunto, a média estimada foi aproximadamente 58% do tempo
em compreensão. Não é uma medida de desperdício: compreender é parte do trabalho.
**Limite:** estimativa dependente da classificação e de hipóteses de tempo de
reação; duas empresas e monitoramento limitado restringem generalização.
**Implicação:** uma memória útil precisa conectar código, documento e evidência
consultada, sem presumir que a IDE contém toda a investigação. Não usar 58%
como economia potencial ou estatística de todos os programadores.

### C6 — Hipóteses de debugging merecem apoio explícito

**Alaboudi e LaToza, VL/HCC 2020:**
[Using Hypotheses as a Debugging Aid](https://arxiv.org/pdf/2005.13652).

**Método/amostra:** estudo preliminar e experimento intra-participantes com
20 desenvolvedores em três pequenos programas com defeitos de API; distribuição
de ajudas controlada por desenho Latin square.
**Achado:** hipóteses iniciais corretas se associaram ao sucesso posterior;
oferecer hipóteses ajudou mais que oferecer apenas localização do defeito no
experimento. Hipótese correta ainda não garantiu correção bem-sucedida.
**Limite:** amostra pequena, tarefas específicas e hipóteses preparadas pelos
pesquisadores. Não valida hipóteses inventadas automaticamente por um LLM nem
autoriza reproduzir o multiplicador de sucesso como promessa comercial.
**Implicação:** explicitar hipótese, previsão, teste e evidência contrária; avaliar
se a ajuda reduz investigação repetida sem criar ancoragem numa causa errada.

### C7 — A interferência depende do contexto da troca

**Abad et al., EASE 2018:**
[Task Interruption in Software Development Projects: What Makes some Interruptions More Disruptive than Others?](https://arxiv.org/pdf/1805.05508).

**Método/amostra:** análise retrospectiva longitudinal de 4.910 tarefas
registradas de 17 profissionais, complementada por survey com 132 praticantes.
**Achado:** características da troca, como origem e mudança de contexto,
apresentaram associações com as medidas de interferência; percepção dos
participantes e padrões nos registros nem sempre coincidiram.
**Limite:** estudo observacional; suspensão e fragmentação são proxies, não uma
medida direta de perda cognitiva. O conjunto não permite prometer uma causalidade
universal ou declarar toda auto-interrupção prejudicial.
**Implicação:** avaliações do Xemnas devem distinguir tarefas/projetos e motivo
da troca. Um recurso útil em debugging pode virar ruído durante tarefas curtas;
o produto não deve impor alertas usando somente uma média agregada.

### C8 — Debugging inclui administrar incerteza e história de versões

**Li e Coblenz, FSE 2026:**
[A Grounded Theory of Debugging in Professional Software Engineering Practice](https://arxiv.org/html/2602.11435v3).

**Método/amostra:** observações/entrevistas de sete profissionais experientes,
mais registros de cinco desenvolvedores em livestream; 17 tarefas e cerca de
11 horas. A página [bibliográfica](https://arxiv.org/abs/2602.11435) informa
aceitação em FSE 2026 e DOI relacionado; foi lida a versão autoral v3.
**Achado:** o modelo envolve reproduzir, desenvolver modelo mental, corrigir e
validar. Experiência, documentação, colegas e história de versões entram no
raciocínio; o artigo propõe tornar incertezas e trilhas explícitas.
**Limite:** amostra pequena e selecionada, tarefas compartilháveis, sessões
agendadas e verbalização; livestream não é amostra representativa. Não mede
efeito de um produto.
**Implicação:** preservar evidência contextual e condições de reprodução, sem
confundir especulação com certeza ou transformar toda investigação em decisão.

## Seis propostas adicionais para seleção

São desenhos de produto derivados pelo pesquisador, não funcionalidades testadas
nos estudos. A numeração local não é uma ordem de prioridade. Experimentos abaixo
são propostas, sem resultados coletados.

### P1 — Quadro de hipóteses de debugging

- **Cena:** humano e agente alternam entre “é cache”, “é versão” e “é concorrência”
  sem registrar o que já contrariou cada explicação.
- **Utilidade:** uma ficha por hipótese com previsão observável, evidência a favor,
  evidência contrária, próximo teste e estado: aberta/refutada/inconclusiva.
- **Base:** C6 e C8. Diferente do Radar de premissas: trata explicações temporárias
  de uma falha, não validade de uma Engineering Decision.
- **Primeiro experimento:** usar fichas manuais em seis bugs; comparar passos
  repetidos e tempo de registro com a forma habitual. Registrar correções e
  desistências, não só sucessos.
- **Risco:** LLM gerar hipóteses plausíveis que ancoram o investigador. Exigir
  previsão/teste e conservar discordâncias; não sugerir certeza numérica inventada.

### P2 — Memória dos caminhos que não explicaram o problema

- **Cena:** outro agente repete a mesma busca e o mesmo experimento inconclusivo.
- **Utilidade:** guardar pergunta investigada, caminho visitado, resultado,
  configuração e razão para parar; recuperar somente em condições comparáveis.
- **Base:** C4 e C8. Diferente da memória de alternativas arquiteturais: registra
  tentativas investigativas, não opções descartadas numa decisão de design.
- **Primeiro experimento:** transferir três investigações entre pessoas/agentes;
  contar repetição dispensável e casos em que repetir foi justificadamente útil.
- **Risco:** “não encontrei” virar “não existe”. Nunca transformar busca sem
  resultado ou teste inconclusivo em proibição; uma versão diferente pode justificar
  reabrir o caminho. Retenção curta por padrão para material temporário.

### P3 — Mapa de comportamento para pontos de entrada

- **Cena:** o bug descreve “confirmar uma decisão duplica o evento”; a pessoa não
  sabe que símbolos ou arquivos representam essa ação.
- **Utilidade:** associar um comportamento reconhecível a entrada, caso de uso,
  teste e decisões relevantes, com ligações verificadas e cobertura explícita.
- **Base:** C3 e C4. É navegação por intenção do usuário, distinta de um grafo
  completo ou de explicar impacto de mudança hipotética.
- **Primeiro experimento:** curar cinco comportamentos e pedir a um dev novo
  localizar onde investigar; medir erros e tempo até um ponto de entrada correto.
- **Risco:** relações estáticas não provam execução. Separar associação humana,
  teste que exerceu o comportamento e caminho inferido; iniciar manualmente.

### P4 — Atlas de exemplos internos com limites de uso

- **Cena:** agente copia uma implementação parecida, mas ela pertence a outro
  contexto e carrega uma exceção que não deveria se espalhar.
- **Utilidade:** exemplares curados do projeto: intenção, trecho/commit, regra que
  ilustram, condições em que aplicar e condições em que não copiar.
- **Base:** C4 e C8. Não é biblioteca global de papers nem gerador de código:
  preserva precedentes internos que exemplificam decisões confirmadas.
- **Primeiro experimento:** cinco exemplares em tarefas equivalentes; avaliar
  transferência correta e cópias inadequadas com revisão cega quando viável.
- **Risco:** o exemplo envelhecer ou virar norma sozinho. Vincular à decisão,
  versão e revisão humana; fonte de código continua sendo evidência.

### P5 — Vocabulário entre problema, domínio e código

- **Cena:** uma issue fala em “revisão”, o código usa termos de inbox, candidato,
  assessment e relatório; busca literal mistura conceitos distintos.
- **Utilidade:** pares curados conceito → termos de usuários → símbolos/arquivos,
  com homônimos, limites e exemplos; servir à busca e aos Context Packs.
- **Base:** C3 e C4. Expande o vocabulário existente do produto para navegação
  verificável de cada Project; não propõe substituir seu CONTEXT atual.
- **Primeiro experimento:** dez consultas reais com sinônimos; avaliar relevância
  e confusões de conceito antes/depois, incluindo termos ambíguos.
- **Risco:** IA consolidar equivalências falsas. Sugerir pares como candidatos,
  exigir confirmação e manter diferenças explícitas entre conceitos próximos.

### P6 — Dossiê de reprodução transferível

- **Cena:** alguém recebe uma correção, mas não consegue repetir a falha nem
  distinguir uma mudança efetiva de diferença no ambiente.
- **Utilidade:** pacote local com revisão do código, entrada mínima sanitizada,
  versões/configuração relevante, passos, esperado, observado, evidência e
  validação da correção. Seleção humana antes de exportar.
- **Base:** C2 e C8. Não é resultado agregado de decisões nem retomada genérica:
  preserva as condições específicas de reproduzir uma investigação.
- **Primeiro experimento:** outra pessoa reproduzir cinco falhas só com o pacote;
  medir sucesso, perguntas extras e informação privada removida.
- **Risco:** executar scripts não confiáveis ou reter segredos. No primeiro
  desenho, registrar referências/comandos como dados, sem execução automática;
  não atribuir causalidade à configuração só por estar no pacote.

## Limites para o produto e avaliação

As fontes sustentam a existência de trabalho cognitivo e de investigação, não a
necessidade de seis novas telas. Experimentos podem começar com fichas e links
locais antes de construir índices ou captura automática. Manter autoridade humana,
escopo temporal e referências verificáveis é mais importante que maximizar captura.

Não usar contagem de edição, navegação ou hipóteses como ranking de desempenho.
Medir junto benefício e custo: repetição evitável, compreensão correta, reprodução
bem-sucedida, erros novos, minutos para manter a memória e material sensível que
precisou ser removido. Detalhes temporários devem poder expirar sem destruir
decisões confirmadas ou suas evidências duráveis.
