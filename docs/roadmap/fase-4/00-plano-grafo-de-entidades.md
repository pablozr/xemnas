# Fase 4 (proposta) — grafo de entidades do projeto

**Status: proposta, não iniciada.** Implementar só depois da semana de uso real, se os sinais abaixo aparecerem. Exige ADR antes do código (SCOPE-001).

## Ideia em uma frase

Ligar decisões e claims às coisas concretas do projeto (componentes, tecnologias, caminhos de arquivo), num grafo tipado, temporal e confirmado por humano, guardado no SQLite — sem banco de grafo e sem extração por IA.

## Sinais para começar

- O contexto certo existia, mas não veio porque o pedido não citava as palavras (limite da busca lexical).
- Centenas de decisões por projeto e necessidade de navegar por módulo.
- Perguntas como "o que decidimos sobre X?" ou "o que vale para este arquivo?" sem boa resposta.

## O que a pesquisa indica

- **Grafo temporal com invalidação, não exclusão.** Memórias de agente em grafo (Zep/Graphiti) guardam dois tempos por fato — quando valeu no mundo e quando foi registrado — e invalidam em vez de apagar. Já fazemos isso: `valid_from/valid_until` nas claims, `confirmed_at` e `supersedes` nas decisões, `created_at` nas relações.
- **Grafo extraído por LLM é caro e ruidoso.** GraphRAG gasta muitos tokens na construção e produz entidades duplicadas e relações não confiáveis; resolução de entidades é difícil. Contraria a regra de autoridade humana do produto.
- **Grafos de código já existem e são complementares.** Ferramentas como CodeGraph indexam AST, chamadas e imports em SQLite local e servem agentes por MCP ("o que chama o quê"). O xemnas guarda "por quê". Não reconstruir o grafo de símbolos; ficar no nível de componente e caminho, e integrar depois se valer a pena.
- **ADR + C4:** o modelo C4 mostra o "o quê" da arquitetura e ADRs o "porquê"; a dor recorrente é manter a ligação entre os dois. É exatamente a ligação que as entidades criam, derivada do trabalho real.
- **Visualização sem "bola de fios":** nunca mostrar o grafo inteiro; começar focado (20–50 nós), expandir vizinhança sob demanda (ego network / k saltos), filtrar por tipo, agrupar, usar cor para tipo e tamanho para importância.

## Modelo proposto

```text
Entidade { id, projeto, tipo: component | technology, nome, chave_normalizada, criada_em, aposentada_em? }
Padrão   { entidade(component), glob: "crates/storage-sqlite/**" }          -- como CODEOWNERS
Alias    { entidade, texto }                                                 -- "sqlite", "SQLite 3"

Decisão ──afeta──────> Componente
Decisão ──usa────────> Tecnologia
Claim   ──aplica-se──> Componente | Tecnologia
Componente ──parte-de──> Componente                                          -- opcional
```

- Toda aresta tem `origem` (`humana` ou `derivada`), `confirmada_em`, `criada_em` e `invalidada_em` (nunca apagada).
- Arestas derivadas nascem como **sugestões** e só valem depois de confirmadas na revisão, como os candidatos de decisão.
- Resolução de entidades determinística: chave normalizada (minúsculas, sem pontuação) + aliases; fusão só manual.

## Como as entidades nascem (sem IA)

| Fonte | Deriva |
| --- | --- |
| Estrutura do repositório (membros do workspace, pastas de topo) | componentes iniciais com padrões de caminho, propostos para confirmação |
| `diff_summary.files` de cada decisão | sugestão "decisão afeta componente" pelos padrões que casam |
| Dependência adicionada em `Cargo.toml`/`package.json` (o filtro de relevância já detecta) | sugestão "decisão usa tecnologia" |
| Ação do usuário | qualquer entidade ou aresta, confirmada na hora |

## Consultas (SQLite, `WITH RECURSIVE`)

- **Lente de arquivo:** caminho → componentes cujos padrões casam → decisões vigentes e claims válidas na data.
- **Impacto:** decisão ou tecnologia → decisões que dependem dela, transitivamente.
- **Vizinhança:** nó → arestas até k saltos (k ≤ 2), com filtros por tipo e data, truncada em N nós.
- **Máquina do tempo:** qualquer consulta com `as_of`, usando os dois tempos.

## Ganhos para o agente

- Injeção por arquivo, não só por palavra: o que vale para o módulo que está sendo editado entra mesmo quando o pedido é "corrige esse bug". Opção a avaliar: hook `tool.execute.before` nas ferramentas de edição do OpenCode, com o caminho do arquivo.
- `get_decision` / `search_context` no MCP passam a aceitar componente e arquivo.

## O que expor para o front

| Visão | O que mostra | Por que é boa |
| --- | --- | --- |
| **Mapa do projeto** | componentes como blocos (estilo C4), tamanho = nº de decisões, cor = atividade recente ou conflito | a primeira tela "uau": a arquitetura desenhada pelas decisões reais |
| **Vizinhança** | uma decisão no centro e, em volta, relações, componentes, claims e evidências; clicar expande | explora sem bola de fios |
| **Linha do tempo por componente** | faixas por componente, decisões como pontos, setas de substituição, controle deslizante de data | mostra a evolução e permite ver "como estava em março" |
| **Lente de arquivo** | escolhe um caminho e vê regras e decisões que valem ali | a pergunta diária de quem programa |
| **Radar de impacto** | "se eu trocar o SQLite": destaca o que depende | ajuda a decidir antes de mexer |
| **Conflitos e reconsiderações** | decisões ativas conflitantes na mesma entidade; "reconsiderar quando" que talvez tenha acontecido | transforma a memória em alerta útil |

Contrato previsto (independente de layout; o front desenha): `KnowledgeGraph::neighborhood(nó, profundidade, filtros, as_of) -> { nodes, edges, truncated }`, `project_map(projeto, as_of)`, `timeline(projeto, de, até)`, `file_lens(projeto, caminho, as_of)`, `impact(nó)`. Layout determinístico (radial / em camadas) é mais estável que force-directed para listas pequenas e fica com o front.

## Fases

1. ADR; componentes com padrões de caminho; aresta decisão→componente sugerida pelos diffs e confirmada na revisão.
2. Tecnologias (dependências) e aliases.
3. Consultas para o front: mapa, vizinhança, linha do tempo, lente de arquivo, impacto.
4. Injeção por arquivo e MCP com filtros por entidade.
5. Conflitos por entidade e alertas de reconsideração.

## Fora do escopo

Banco de grafo; entidades ou relações extraídas por LLM; grafo de símbolos (funções, chamadas) — deixar para ferramentas de grafo de código e integrar se necessário.

## Riscos

- Padrões de caminho envelhecem com renomeações: verificação que sinaliza padrão que não casa mais nenhum arquivo.
- Excesso de ligações sugeridas: confirmação humana e limite por decisão.
- Caminhos podem ser sensíveis: tudo local, e o diagnóstico exportado continua sem caminhos.

## Referências

- Zep: A Temporal Knowledge Graph Architecture for Agent Memory — https://arxiv.org/pdf/2501.13956
- Graphiti (Neo4j) — https://neo4j.com/blog/developer/graphiti-knowledge-graph-memory/
- Denoising Knowledge Graphs for RAG — https://awesomepapers.io/ai-agents/papers/2510.14271
- When Is Graph Structure Worth Its Cost? — https://arxiv.org/pdf/2609.18099
- Senzing: GraphRAG e resolução de entidades — https://senzing.com/knowledge-graphs-graphrag/
- CodeGraph — https://github.com/colbymchenry/codegraph
- Understand Anything — https://dev.to/arshtechpro/understand-anything-turn-any-codebase-into-an-interactive-knowledge-graph-37ed
- C4 + ADR — https://visual-c4.com/blog/c4-model-architecture-adr-integration
- Visualização de grafos e o problema da bola de fios — https://cambridge-intelligence.com/blog/hairball-effect-in-graph-visualization/ e https://www.researchgate.net/publication/333503871_Taming_a_Graph_Hairball_Local_Exploration_in_a_Global_Context
