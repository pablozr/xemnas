# Análise do produto hoje: o que está fraco e o que melhorar

Data: 2026-10-02.

**Pergunta:** olhando o que o xemnas é agora, o que está ruim, o que falta e o que
melhorar primeiro, e como a Visão pode mostrar os fluxos de um jeito que se entenda
de relance (C4)?

**Status:** Em implementação. O passo 2 (Visão com contêineres e fluxo dinâmico,
derivados sem IA nova) foi entregue; os demais seguem abertos. Análise feita sobre
capturas da release de 02/10/2026 (dados de demonstração), o código e os documentos
do repositório. Não houve uso real nem teste com pessoas; o diário de dogfood
(`docs/operacao/dogfood-log.md`) está vazio.

## Leitura geral

O que está de pé e é bom:

- **O ciclo existe de ponta a ponta:** captura, análise, Revisão humana, decisão
  versionada, Mapa, Contexto entregue ao agente, medido e auditável.
- **Autoridade humana e privacidade:** nada vira decisão sem confirmação, a tela de
  IA diz exatamente o que sai da máquina, segredos ficam no cofre.
- **Acabamento visual coerente** e desempenho tratado como pilar (listas virtuais,
  grafo agrupado, layout em fundo, medições em `docs/`).

O que mais pesa, em ordem:

1. **Nunca foi usado de verdade.** O registro de dogfood está em branco ("a semana só
   começa quando houver UI utilizável"). Todo o resto, inclusive esta análise, é
   hipótese. Sem uma semana real, não sabemos o ruído, o tempo de revisão nem o
   valor percebido.
2. **A qualidade dos candidatos depende do provedor de IA,** e o padrão é o
   extrator local, "heurística offline". Quem não configura IA vê um produto bem
   mais fraco do que o que foi desenhado.
3. **Quase tudo que mostra valor é texto.** O ponto fraco que você apontou, a Visão,
   é o caso mais claro, mas o padrão se repete (abaixo).
4. **Falta provar que o contexto ajuda** (`medir-eficacia-do-contexto.md`).

## Problemas por área

Severidade: **alta** (bloqueia o valor), **média** (atrapalha o uso), **baixa**
(polimento). "Evidência" é o que foi visto nas capturas ou no código.

### Visão (resumo e fluxos): alta

- **Fluxos são uma lista numerada de texto.** Cada passo tem título, uma frase e
  chips (um componente, citações). Não há **atores** (pessoa, agente, sistema
  externo), **de onde vem e para onde vai** cada passo, **o que trafega**
  (protocolo, dados), nem **ramificações** (erro, repetição, recusa).
- **Todos os fluxos têm a mesma forma** e o mesmo peso; não dá para ver, de relance,
  quais componentes participam de vários fluxos, nem qual é o caminho principal.
- **Os componentes não se relacionam entre si no modelo:** o Mapa liga decisões a
  componentes (`afeta`, `usa`, `vale para`), mas não "componente chama componente".
  Por isso nenhuma tela mostra a arquitetura como estrutura.
- **Passos podem não ter componente** (`entity_id` é opcional) e o texto do passo é
  da IA, sem checagem contra o Mapa.
- **O resumo** é bom (parágrafos com fontes citáveis), mas termina ali: quem quer
  "onde fica X?" ainda precisa ir ao Mapa em Blocos ou Grafo, que são outra gramática.
- *Proposta:* C4 (abaixo).

### Revisão: média

- **Fadiga de fila** (risco já apontado em `ideias-de-produto.md`): a lista é
  cronológica, sem agrupar por sessão do agente, sem triagem rápida, sem desfazer.
  Ruído de documentos foi reduzido (`documents::digest`), mas falta medir a taxa de
  dispensa real.
- **Comparar versões** de uma decisão não existe lado a lado (Decisões mostra a
  versão atual e uma lista de versões).
- **Sem aviso de conflito no candidato**: contradizer uma decisão em vigor só aparece
  depois, em Sugestões ou na revisão consultiva.
- *Proposta:* agrupar por sessão, aviso de conflito no candidato, desfazer com `Z`,
  comparação de versões (as ideias 3 e 4 de `inspiracao-gpui-e-mercado.md`).

### Decisões e Contexto: média

- **Contexto** mostra o que foi entregue e quanto custou, mas **não diz se ajudou**.
  Os números (5 entregas, 162 tokens por bloco) são exposição, não eficácia.
- **Regras não têm verificador:** nada liga uma regra ao que seria quebrá-la.
- *Proposta:* painel Eficácia e `check` por regra (`medir-eficacia-do-contexto.md`).

### Mapa: média

- **Grafo e Blocos escalam agora**, mas o **significado** ainda é fraco: sem busca,
  sem filtro por escopo, sem "cenas" salvas, sem ligação entre componentes.
- **Sugestões** estava ilegível; foi reescrita com frases e "Ao confirmar, …". Falta
  ver com dados reais.
- *Proposta:* relações entre componentes (a base do C4), busca e visões salvas.

### Projetos e começo de uso: média

- **Projeto novo começa vazio** (o app só conhecia diffs; documentação entra agora
  filtrada). Falta um percurso de primeira vez: importar ADRs e `AGENTS.md`, mostrar o
  que foi lido e pedir confirmação.
- **Sem noção de vários projetos juntos:** cada um é uma ilha (relevante para o
  conhecimento geral do dev).

### Assistente (Xemnas): baixa hoje, alta depois

- O mascote e o painel são honestos (não simulam conversa), mas **não fazem nada que
  a paleta não faça**. É o gancho certo para o futuro assistente de decisão; hoje é
  decoração com função de atalho.

### Base técnica: média

- **Semear 5.000 decisões leva mais de 100 s** (provável custo quadrático em
  `create_entity` ou no semeador): uma importação grande real sofreria o mesmo.
- **Sem medição de uso do próprio app** (telemetria é local e de saúde, não de
  produto), coerente com a privacidade, mas então o dogfood precisa ser deliberado.
- A **base de testes depende do Controle de Aplicativo do Windows** liberar cada
  binário novo; isso atrasa verificação (não é do produto, mas custa tempo).

## Visão em C4

### Por que C4 resolve o que a lista não resolve

O [modelo C4](https://c4model.com/) descreve um sistema em **níveis de zoom**:
contexto (o sistema, as pessoas e os sistemas ao redor), contêineres (as partes que
rodam e se falam), componentes (o que há dentro de um contêiner) e, se preciso,
código. Existe também o **diagrama dinâmico**, que numera as interações de um cenário
sobre os mesmos elementos: é exatamente o que os "fluxos" da Visão tentam ser, só que
sem o desenho. Vantagens para nós:

- **Bate o olho:** formas e setas dizem o que o texto diz em um parágrafo.
- **Um fluxo vira um caminho sobre a estrutura,** não uma lista isolada: dá para ver
  quais contêineres participam de quais fluxos.
- **Zoom em vez de saltos de tela:** da Visão (nível 1) até o componente (nível 3),
  com migalhas, na mesma gramática.
- **É deterministicamente derivável do que já temos** em boa parte.

Um exemplo do nível 2 do próprio xemnas, com o fluxo "da conversa à decisão"
numerado (1 o agente envia a captura à API local; 2 a API enfileira no núcleo; 3 o
núcleo consulta o provedor; 4 a pessoa revisa e confirma no app; 5 o núcleo grava no
SQLite), foi mostrado na conversa que gerou este documento.

### Proposta de produto

Na aba Visão, ao lado de "Fluxos", uma vista **Arquitetura** com três níveis:

| Nível | Mostra | De onde vem |
| --- | --- | --- |
| Contexto | O projeto no centro; pessoas e sistemas externos (agente, provedor de IA, serviços) ao redor, com uma frase de relação | Tecnologias e "sistemas" do Mapa; fluxos |
| Contêineres | Componentes de topo (o que roda), cada um com tecnologia e uma linha de papel; setas rotuladas ("HTTP em loopback", "SQL") | Componentes sem `part_of`, tecnologias ligadas, interações |
| Componentes | O interior de um contêiner (`part_of`) e suas dependências | `part_of`, interações |

E o **fluxo dinâmico**: ao abrir um fluxo, as setas dos seus passos se numeram sobre o
diagrama de contêineres, e a lista de passos à direita (o texto de hoje) fica
sincronizada: passar o mouse num passo acende a seta e os dois elementos.

**Interação:** clicar numa caixa abre a página do componente; duplo clique desce um
nível; migalhas sobem; passar o mouse numa seta mostra o rótulo completo e as
decisões que a governam (as citações de hoje). Cada nível tem estado vazio, de
carregamento e de erro como as outras telas.

**Acessibilidade e escala:** a lista textual continua como equivalente (leitor de
tela e teclado: caixas e setas são alcançáveis em ordem); mais de ~12 caixas num
nível agrupam por `part_of` ou viram "+N" expansível (o mesmo princípio do grafo).

### O que falta no modelo (backend)

1. **Interações entre componentes.** Hoje só decisões se ligam a componentes. Duas
   fontes, em ordem de custo:
   - **Derivadas dos fluxos** (grátis): passo *i* num componente e passo *i+1* em
     outro vira uma interação, com o título do passo como rótulo.
   - **Propostas pela IA na geração da Visão** (um campo novo no resultado
     estruturado: contêineres, interações com rótulo e tecnologia, cada uma com
     citações), validadas contra o Mapa e **sempre como sugestão** (entram em
     Sugestões, não no Mapa, até a pessoa confirmar). Custo: poucos tokens a mais por
     geração, reaproveitável enquanto a impressão digital das fontes não mudar.
2. **Tipos de elemento:** hoje há Componente e Tecnologia. O C4 pede também **Pessoa**
   e **Sistema externo** (agente, provedor). Pode ser um campo `papel` em Tecnologia
   ("externo") e uma entidade leve para pessoas, sem migrar o resto.
3. **Layout:** determinístico, em camadas da esquerda para a direita para o nível
   2, e raias por componente para o fluxo; reaproveita o canvas do grafo (formas,
   setas, marcadores numerados). Sem simulação de forças: o diagrama precisa ser
   estável e previsível.

Regra do produto mantida: **nada inventado pela IA entra como fato**. Uma interação
sem citação ou sem componente no Mapa aparece pontilhada e marcada "não confirmada".

### Como saber se melhorou

Tarefas de dogfood, com tempo e erros antes e depois: "onde acontece X?", "o que o
agente fala com o quê?", "quais componentes um fluxo atravessa?", "o que muda se eu
trocar o provedor?". Meta inicial (a validar): responder cada uma em menos de 30
segundos sem sair da Visão. Registrar em `docs/operacao/dogfood-log.md`.

## Menos texto, mais forma

O padrão que se repete: o produto **mostra valor em frases e números soltos**. A
regra proposta, para toda tela nova e para revisar as atuais:

1. **Todo número ganha uma forma:** barra (orçamento, confiança), anel (fila,
   cobertura), linha pequena (tendência), tira de calor (atividade por semana).
2. **Toda relação ganha uma linha:** quem decide o quê, o que afeta o quê, quem chama
   quem (a Visão em C4 é o primeiro caso; o Mapa e a página do componente já têm).
3. **Todo estado ganha cor e ícone, não uma frase** (em vigor, substituída, em
   conflito, aguardando, enviada).
4. **O texto fica para o porquê:** a justificativa, a evidência, a pergunta. Se uma
   frase só repete o que um número ou uma forma já diz, a frase sai.

Primitivas pequenas a acrescentar em `ui::patterns` (todas desenhadas em canvas,
baratas, com equivalente textual no `aria_label`): `meter` (barra com limite),
`ring` (anel de progresso), `sparkline` (linha de tendência), `heat_strip` (tira
de atividade), `delta_bar` (adições e remoções de um diff) e `mini_graph` (a
vizinhança de uma decisão ou componente, já existente na página do componente).

Onde aplicar, por impacto:

| Tela | Hoje | Forma proposta |
| --- | --- | --- |
| Contexto | "5 entregas, 162 tokens por bloco" em números | Linha de entregas por dia; barra de orçamento por bloco (já há); funil entregue → absorvido → arquivo tocado (Eficácia) |
| Revisão | Lista de títulos e datas | Ícone e cor por tipo e risco do candidato; mini-mapa dos componentes que ele toca; barra de adições e remoções da evidência; confiança como medidor (já há) |
| Decisões | Lista por mês | Trilho de versões com marcos; vizinhança da decisão (relações) em mini-grafo; idade e "reconsiderar quando" como marca |
| Mapa em Blocos | Cartões com texto e contagens | Linha de atividade por bloco; selo de conflito; barra de cobertura (decisões por componente) |
| Visão | Resumo em parágrafos | Pulso do projeto: tira de calor de decisões por semana; arquitetura em C4 (feito) |
| Projetos | Lista com caminho | Anel de fila por projeto; última atividade |
| Sugestões | Cartões com frase (feito) | Mini-diagrama do que seria ligado (duas caixas e uma seta) em cada cartão |

Cada primitiva entra por tela, com os dados que a tela já tem; só as que pedem dado
novo (absorção, atividade por semana) dependem de backend.

## O que fazer primeiro

| # | Passo | Por quê | Esforço |
| --- | --- | --- | --- |
| 1 | **Uma semana de dogfood de verdade** com IA configurada, registrando o diário | Sem isso o resto é hipótese; mede ruído e tempo de revisão | Baixo (uso) |
| 2 | ~~Visão: nível Contêineres + fluxo dinâmico derivados dos fluxos e do Mapa (sem IA nova)~~ **Feito** (`application::architecture`, `screens/overview/diagram.rs`) | Resolve a queixa agora, sem custo de tokens; já é útil com o que existe | Médio (UI) |
| 3 | Interações e tipos de elemento propostos pela IA, entrando em Sugestões | Dá rótulos e protocolos às setas; precisa do contrato do backend | Médio/alto |
| 4 | Revisão: agrupar por sessão, conflito no candidato, desfazer | Ataca a fadiga de fila | Médio |
| 5 | Eficácia do contexto: linha de base histórica e `check` por regra | Prova o valor da peça central | Médio |
| 6 | Importar ADRs e `AGENTS.md` no primeiro uso | Resolve o começo vazio | Médio |
| 7 | Nível Componentes e visões salvas do Mapa | Completa o zoom | Médio |

Os passos 2 e 3 dividem o trabalho entre as sessões: o 3 pede contrato do `application`
(a UI não deve calcular regra de negócio), o 2 só consome o que já existe.

## Limitações

- Capturas de demonstração com poucos dados; o aspecto com um projeto grande real
  pode mudar o diagnóstico (a Visão com 40 fluxos, por exemplo).
- Não li o prompt de geração da Visão linha a linha nem avaliei a qualidade dos
  fluxos que um modelo de verdade produz; o diagnóstico de "fraco" se apoia na forma
  da tela e no modelo de dados.
- Esforços são relativos, não prazos.
