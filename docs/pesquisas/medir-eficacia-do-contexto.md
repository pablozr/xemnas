# Medir se o contexto injetado ajuda o agente

Data: 2026-10-02.

**Pergunta:** como saber se o bloco de contexto que o xemnas injeta nos agentes
(`<xemnas-context>`) realmente melhora a implementação, e o que medir para decidir
o que entregar, o que cortar e quando automatizar mais?

**Status:** Aberta. Pesquisa e proposta de medição; nada implementado ainda. Parte
da fundamentação foi lida por resumo (marcada "confirmar").

## Resumo e opinião

1. **A expectativa honesta é um efeito pequeno e concentrado, não um ganho geral.**
   O estudo mais direto que achei (ETH Zürich, ICLR 2026 workshop) mediu arquivos
   de contexto como `AGENTS.md` em tarefas reais: **não melhoraram a taxa de
   sucesso em geral e aumentaram o custo de inferência em mais de 20%**; o que
   ajudou foi **especificar práticas fora do padrão**, e visões gerais do
   repositório não ajudaram. Um segundo estudo (ablação com Claude Code e Codex)
   não achou efeito mensurável na correção (limitado a 10–15 pontos percentuais) e
   atribui as falhas a habilidade de implementação, não a falta de conhecimento do
   repositório. Detalhes em [Fontes](#fontes).
2. **Isso não condena o xemnas, mas muda o que ele deve provar.** O xemnas já
   evita o que a pesquisa diz não funcionar: não injeta visão geral, injeta
   decisões e regras confirmadas, por arquivo editado (gatilho `Edit`) ou por
   prompt, com orçamento de 300 tokens. O que falta é **evidência de que cada
   categoria paga o seu custo**. Minha proposta é trocar a pergunta "o contexto
   melhora o agente?" por três hipóteses testáveis:
   - **H1 (adesão):** uma regra ou decisão não óbvia, injetada, é seguida.
   - **H2 (retrabalho):** com contexto, o agente refaz ou contradiz menos o que o
     projeto já decidiu.
   - **H3 (custo líquido):** o contexto não aumenta tokens, voltas e tempo da
     sessão acima do que economiza.
   Correção geral das tarefas fica como métrica de **não-regressão** (não piorou),
   não como promessa.
3. **O custo mais barato de medir é o que mais informa:** *canários* (decisões
   propositalmente não óbvias, testadas com e sem injeção) para H1; *holdout*
   contínuo (parte das sessões em `shadow`, que já existe) para H2 e H3;
   repetições offline em tarefas reais só quando mudar a seleção. Sem isso, ajustar
   o pack é fé.
4. **Quando a medição diz que um item não é usado, ele sai.** Item entregue muitas
   vezes, nunca citado nem relacionado ao que o agente tocou, é peso morto: gasta
   tokens e, segundo a pesquisa, pode atrapalhar. Usar a medição para podar é o
   que justifica o pilar de custo baixo.

## O que a pesquisa diz

| Fonte | O que mediu | O que concluiu | Para o xemnas |
| --- | --- | --- | --- |
| [Evaluating AGENTS.md](https://arxiv.org/abs/2602.11988) (ETH, 2026) | Arquivos de contexto gerados por modelo e escritos por desenvolvedores, em SWE-bench e em tarefas de repositórios com contexto commitado | Sem ganho geral de sucesso; custo +20%; instruções são seguidas; visões gerais não ajudam; ajudam práticas fora do padrão | Injetar só o que um agente não infere do código; tratar visão geral como dano |
| [Do Context Files Help Coding Agents?](https://arxiv.org/abs/2607.27250) (2026) | Ablação em 17 tarefas de 3 repositórios, 288 execuções, Claude Code e Codex; `none`, `always_on`, `selective`; testes ocultos; comparação pareada e teste de equivalência | Estratégia de contexto não move a correção (≤ 10–15 pp); eficiência (chamadas, tempo, tokens, cache) também medida; contexto nunca converteu quase-acerto em acerto | O desenho serve de modelo para o nosso benchmark offline; resultado esperado é "não piora, custa menos ou igual" |
| [Probe-and-Refine](https://arxiv.org/abs/2606.20512) (2026) | Refinar guia do repositório com sondas (bugs sintéticos) em chamadas únicas, sem laço de agente | O guia bem produzido melhorou a taxa de resolução (33,0% contra 28,3% estático e 25,5% sem guia, 4 tentativas, um modelo); ganho vem de achar os arquivos certos, não de código melhor | "Como o contexto é produzido é a variável decisiva": sondas baratas para avaliar seleção; foco em localização |
| [Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) (Anthropic) | Princípios | O menor conjunto de tokens de alto sinal; recuperação "na hora" por identificadores leves, em vez de despejar tudo | Nosso bloco é minúsculo e traz referências `D:xxxx`; o agente puxa o resto pelo MCP |
| RAGAS e TruLens ([resumo](https://atlan.com/know/how-to-evaluate-rag-systems-explained/)) | Métricas de RAG | Precisão de contexto (o recuperado é útil), recall de contexto (trouxe o necessário), fidelidade e relevância | Vocabulário para nossas métricas de seleção (abaixo) |

Ressalvas: li resumos (inclusive de abstracts via ferramenta de sumarização), não os
artigos inteiros; os estudos usam tarefas de bugs em repositórios públicos de
Python/JavaScript, com modelos e agentes específicos, e o `AGENTS.md` deles é um
arquivo estático, não decisões confirmadas por humano com escopo por arquivo. A
transferência para o xemnas é uma hipótese a testar, exatamente o que este plano faz.

## O que já temos para medir

- **`shadow` e `inject`** (`InjectionMode`): o bloco é calculado e registrado em
  ambos; só `inject` chega ao agente. Isso já é um grupo de controle.
- **Registro de cada entrega** (`InjectionRecord`): sessão, projeto, modo, tokens,
  omitidos pelo orçamento, itens e versões. Nunca guarda o texto do prompt.
- **Deduplicação por sessão** (um item não volta na mesma sessão).
- **Gatilhos** `prompt` e `edit` (este só com o que o mapa liga aos componentes
  editados).
- **Ferramentas MCP de leitura** (`get_decision`, `search_context`): o agente
  *puxando* uma decisão é um sinal direto de uso.
- **Captura das conversas e Revisão:** candidatos que repetem ou contradizem uma
  decisão em vigor, e o que a pessoa dispensa, são sinais de retrabalho.

## Escada de evidência

Do mais barato ao mais forte; cada degrau responde a uma pergunta diferente e
nenhum prova causa sozinho.

| Nível | Pergunta | Sinal | Custo |
| --- | --- | --- | --- |
| L0 Exposição | O que foi entregue? | Itens, tokens, omitidos, sessões com bloco (já existe) | Zero |
| L1 Absorção | O agente usou? | Item citado (`D:xxxx` ou trecho) na resposta; item aberto pelo MCP; arquivos do escopo do item editados depois da entrega | Baixo (análise local, só guarda ids e contagens) |
| L2 Adesão | O agente seguiu? | Para regras verificáveis (caminho proibido, dependência vetada, padrão obrigatório): o diff posterior as viola? | Médio (regras com verificador) |
| L3 Retrabalho | Evitou refazer? | Candidatos da sessão que repetem ou contradizem uma decisão em vigor; correções da pessoa; reversões do que o agente fez | Baixo, mas exige comparar grupos |
| L4 Resultado | Ficou melhor e mais barato? | Voltas, chamadas, tokens e tempo por sessão; teste passando; mudança revertida em até 7 dias | Alto, só em experimento controlado |

### Métricas

- **Precisão de entrega:** itens com L1 positivo ÷ itens entregues. Mede ruído.
- **Ociosidade:** itens entregues N vezes e nunca absorvidos. Candidatos a sair.
- **Cobertura:** sessões em que o agente tocou um escopo coberto por uma decisão
  em vigor e recebeu o item ÷ sessões que tocaram esse escopo. Mede perda
  (recall do contexto).
- **Taxa de retrabalho** por 100 sessões: candidatos que repetem/contradizem uma
  decisão em vigor. Menor é melhor; é a métrica principal de H2.
- **Adesão** por regra verificável: seguida ÷ aplicável (H1).
- **Custo líquido** (H3): Δ de tokens, voltas e tempo entre `inject` e `shadow`,
  somando os tokens do bloco, com intervalo de confiança.
- **Não-regressão:** taxa de sucesso/reversão igual ou melhor, com margem declarada.

## Experimentos

1. **Canários (H1, causal, barato).** Em um projeto de teste, criar decisões
   propositalmente não óbvias ("usar a biblioteca A em vez da B; nunca usar `unwrap`
   em `crates/x`") e tarefas curtas cuja solução natural as viola. Rodar o agente
   em modo não interativo com `inject` e com `none`, 3 repetições cada. Mede adesão
   direta em horas, não semanas. É o análogo das sondas do Probe-and-Refine, só
   que sobre decisões nossas.
2. **Holdout contínuo (H2 e H3).** Uma fração fixa de sessões (por exemplo 15%),
   escolhida por hash determinístico do id da sessão, fica em `shadow` mesmo com o
   projeto em `inject`. A comparação `inject` contra `shadow` é a única que fala de
   causa em uso real. Com poucas sessões por semana, precisa de semanas: a tela
   deve dizer "poucos dados" e mostrar o intervalo, nunca um número solto.
3. **Repetição offline em tarefas reais (H3 e não-regressão).** Seguir o desenho do
   segundo estudo: tarefas de PRs mesclados do próprio projeto, testes ocultos,
   estratégias `none`, `always_on` e `selective`, comparação pareada e teste de
   equivalência. Caro (tokens): rodar só quando a seleção do pack mudar, em um
   conjunto pequeno e fixo.
4. **Avaliação offline da recuperação (L0 e cobertura).** Sem agente: para cada
   decisão em vigor, prompts e arquivos que *deveriam* trazê-la; medir recall@k e
   precisão do `build_pack`. Rápido, determinístico, bom para afinar a busca.
5. **Reprodução histórica.** Reaplicar a seleção atual a prompts de sessões
   passadas e perguntar: a decisão que a pessoa teve de repetir ou corrigir depois
   estaria no bloco? É recall contra uma verdade que o próprio uso produziu.

### Estatística realista

Poucos dados e tarefas heterogêneas: usar comparação **pareada** quando houver
pares (canários, repetição offline), **intervalos por bootstrap** em vez de
valor-p isolado, e **equivalência** ("não piorou mais que X") para a
não-regressão. Reportar tamanho de amostra ao lado de todo número, e separar
"sem evidência" de "sem efeito". Não comparar agentes ou modelos diferentes no
mesmo gráfico; a dificuldade de uma tarefa depende do agente (o segundo estudo
mostra correlação de 0,75 entre dificuldades dentro do mesmo agente).

## Como coletar sem violar o que já decidimos

- **Nada de texto de prompt ou de resposta guardado** (regra existente). A
  detecção de L1 roda no adaptador, na hora, e só grava: sessão, item, versão,
  tipo do sinal (`citado`, `aberto`, `escopo_tocado`) e contagem.
- **Local primeiro:** L0, L1, L3 e o holdout não precisam de modelo. L2 pode usar
  um verificador por regra (caminho, padrão, dependência) antes de qualquer juiz
  de modelo; um juiz de modelo é opcional, amostrado e marcado como estimativa.
- **O experimento é visível:** a pessoa vê que o holdout existe, pode desligá-lo
  por projeto e o motivo está na tela (medir exige um grupo sem contexto).

## O que construir, em fases

| Fase | Entrega | Onde | Gate |
| --- | --- | --- | --- |
| 0 | Eventos de absorção (`citado`, `aberto`, `escopo_tocado`) ligados a cada entrega | Adaptadores e `application::injection`; tabela nova (migration) | Sem texto de prompt; testes de privacidade |
| 1 | Painel **Eficácia** em Contexto: funil entregue → absorvido → arquivo do escopo tocado; ociosos com ação "tirar do contexto"; custo em tokens | UI (Contexto) sobre casos de uso novos | Mostrar tamanho de amostra; nenhuma causa afirmada |
| 2 | Holdout determinístico por sessão e comparação com intervalo | Backend e UI | Configurável e visível; "poucos dados" abaixo de um mínimo |
| 3 | Canários e repetição offline (`xemnas bench`) com agentes em modo não interativo | Ferramenta separada, não no app | Custo declarado antes de rodar; resultados em `docs/` |
| 4 | Usar a utilidade medida para ranquear e podar o pack, com exploração controlada (um item de baixa pontuação ocasionalmente, para continuar medindo) | `ContextProvider` | Só com evidência das fases 1 a 3 |

Backend e front em sessões separadas: as fases 0, 2 e 4 são de backend; a 1 é de
UI sobre contratos que a 0 define; a 3 é ferramenta.

## Sobre as duas ideias para o futuro

### 1. Agentes autônomos e o assistente de decisão entre projetos

**Opinião: a direção é boa, e a ordem importa.**

- **Posicionamento.** Orquestradores como o [Symphony da OpenAI](https://openai.com/index/open-source-codex-orchestration-symphony/)
  (entendi "dots" como isso ou o Codex multiagente; me corrija se for outra coisa)
  fazem do quadro de tarefas um plano de controle: cada issue vira um agente que
  codifica, testa e abre PR. O espaço que eles não ocupam é o **plano de
  decisões**: o que o projeto decidiu, por quê, e com autoridade humana. Minha
  sugestão é o xemnas ser o plano de decisões que os orquestradores *consultam*
  (MCP, que já temos), antes de competir como orquestrador.
- **Autonomia exige níveis.** Agente autônomo decidindo sozinho precisa de uma
  política por risco: decisão reversível e barata pode virar candidato com
  aceitação automática e trilha de auditoria; irreversível (dados, segurança,
  contrato público, dependência) continua exigindo a pessoa. A Revisão deixa de
  ser uma fila de tudo e vira o portão do que passa do limite.
- **Conhecimento entre projetos é o ponto mais arriscado.** Uma decisão vale num
  contexto (stack, restrições, escala). Reaproveitar sem as **condições de
  aplicabilidade** gera o pior ruído: um conselho confiante e errado. Começar por
  *sugerir análogos com proveniência* ("no projeto X, com SQLite e app local, vocês
  decidiram Y porque Z") como candidato a ser confirmado no projeto novo, nunca
  injeção automática. Guardar, em cada decisão, as condições em que vale.
- **Fronteiras de confidencialidade.** Misturar projetos de clientes diferentes
  pode vazar contexto. Precisa de grupos de projetos ("espaços") e consentimento
  explícito para o cruzamento; o padrão é isolado.
- **Custo.** Assistente que consulta tudo gasta tokens; seguir o pilar: busca local
  primeiro, resposta de modelo só sob pedido.
- **E tem de ser medido.** Contexto vindo de outros projetos é a fonte de ruído
  mais provável; só entra no bloco se o painel de Eficácia mostrar utilidade.

### 2. Medir a eficácia da injeção

Já escrito acima. Minha opinião: **é pré-requisito para a ideia 1**. A própria
decisão ADR-0003 já pedia "medir a utilidade do pack no uso real antes de qualquer
automação"; a pesquisa de 2026 reforça, porque o efeito médio de contexto estático
é nulo ou negativo. Quanto mais automático e abrangente o contexto, mais é preciso
provar que ele paga o custo.

## Perguntas em aberto

- Quais regras viram **verificáveis** (caminho proibido, dependência vetada,
  padrão obrigatório)? Isso decide o alcance do nível L2. Pode ser um novo campo
  opcional na regra, preenchido pela pessoa ou sugerido na Revisão.
- Qual o volume real de sessões por semana por projeto? Define o tempo de um
  holdout útil e se o painel precisa de um período de aquecimento.
- Os adaptadores (Claude Code, Codex, OpenCode) expõem o suficiente para detectar
  citação e arquivos tocados sem guardar texto? Precisa ser verificado por adaptador.
- Aceitamos um grupo sem contexto em projetos reais? É o preço da medição causal;
  alternativa é só canários e repetição offline.

## Fontes

- Gloaguen et al., [Evaluating AGENTS.md: Are Repository-Level Context Files Helpful for Coding Agents?](https://arxiv.org/abs/2602.11988), ICLR 2026 workshop (resumo lido; confirmar números no artigo).
- [Do Context Files Help Coding Agents? A Two-Agent Ablation Study on Real Repositories](https://arxiv.org/abs/2607.27250) (resumo lido; confirmar).
- [Probe-and-Refine Tuning of Repository Guidance for Coding Agents](https://arxiv.org/abs/2606.20512) (resumo lido; confirmar).
- Anthropic, [Effective context engineering for AI agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents).
- OpenAI, [Symphony: open-source Codex orchestration](https://openai.com/index/open-source-codex-orchestration-symphony/) (resumo de busca).
- [Memorix](https://github.com/AVIDS2/memorix) e [Cross-Agent Memory](https://github.com/VladimirGutuev/cross-agent-memory): camadas de memória compartilhada via MCP (resumo de busca).
- Métricas de RAG (RAGAS, TruLens): [resumo](https://atlan.com/know/how-to-evaluate-rag-systems-explained/).
