# Mercado e ideias de produto

**Data:** 2026-10-06 (condensa pesquisas de 30/09 a 02/10/2026).
**Pergunta:** quem já faz algo próximo do Xemnas, onde colidimos e quais ideias têm
a evidência mais forte?
**Status:** Aberta. Nada aqui é escopo aprovado nem demanda validada. As fontes
primárias foram lidas em outubro de 2026; o que o fabricante diz prova
posicionamento, não eficácia. Os documentos originais foram apagados na
consolidação; o histórico está no git.

## Onde está o espaço

O mercado já tem memória persistente, grafos temporais, regras aprendidas de PR,
contexto por MCP e avaliação de agentes. Nenhum desses atributos isolado é
novidade. A hipótese de diferenciação é a combinação: **escolha humana
confirmada, com evidência de código e condições de validade, entregue ao agente
na hora certa, local e barata**. Não há evidência para afirmar exclusividade.

## Concorrentes que importam e onde colidem

| Produto | O que faz (fonte primária) | Colisão com o Xemnas |
| --- | --- | --- |
| Mem0 / OpenMemory | Memória compartilhada por MCP, dashboard; [guia](https://mem0.ai/library/coding-agents/how-to-make-your-clients-more-context-aware-with-openmemory-mcp) | Portabilidade entre agentes. Local não é offline: a inferência usa chave de API |
| Zep / Graphiti | Episódios de origem, validade temporal, invalidação com histórico, busca híbrida; [repositório](https://github.com/getzep/graphiti) | Grafo, tempo e proveniência isolados. Falta a autoridade humana |
| Letta | Memória em Git e blocos compartilhados; [SDK](https://docs.letta.com/agent-sdk/memory) | Memória editável pelo agente; a nossa é protegida por confirmação |
| Supermemory | Memórias que atualizam e derivam outras; [produto](https://supermemory.ai/product/) | Não é só nuvem nem só vetor; já tem servidor local |
| Pieces | Captura silenciosa, timeline, MCP, processamento local; [LTM](https://docs.pieces.app/products/core-dependencies/pieces-os/long-term-memory) | Captura e recuperação do trabalho no desktop |
| Hindsight | Retain/recall/reflect, bancos por projeto, observações com evidência; [repositório](https://github.com/vectorize-io/hindsight) | O mais próximo em memória técnica evolutiva e local |
| Cursor Rules, Continue | Regras por escopo, globs e ativação por descrição; [Cursor](https://cursor.com/docs/rules), [Continue](https://docs.continue.dev/customize/deep-dives/rules) | Alternativa familiar; exportar sem a semântica de ativação muda o comportamento |
| Qodo | Regras com origem, escopo, aplicação em revisão e métricas | Governança de regras é categoria concorrida |
| CodeRabbit | Learnings de feedback, escopo e estatísticas | Memória precisa de dono e correção; preferência aprendida não é decisão confirmada |
| Swimm | Documentação acoplada ao código, com sincronização | Atualizar o trecho não prova que a justificativa vale |
| Sourcegraph Deep Search | Investigação com buscas e arquivos lidos expostos | Mostrar como a resposta foi montada |
| CodeScene | Ilhas de conhecimento pelo histórico de autoria | Achar onde falta contexto antes de mudar |
| Structurizr, Ilograph, Eraser | Um modelo, várias vistas; percursos guiados; diagramas por pergunta | Vistas coerentes e ADRs no grafo não são novidade |
| Log4brains, adr-tools | ADRs em Markdown, timeline, site estático | Catálogo de ADRs sozinho diferencia pouco; portabilidade é requisito |
| Superhuman, Linear | Triagem por teclado, adiar, desfazer | Referência para a fila de Revisão |

## Ideias com evidência mais forte

Uma linha cada; todas são hipóteses sem validação com usuários. O que já virou
trabalho concreto está em `backlog-de-ideias.md`.

- **Radar de premissas e sala de reconsideração.** Condições de "reconsiderar
  quando" ligadas a fontes; a mudança numa fonte abre uma revisão, não um veredito.
  Piloto manual com dez premissas antes de qualquer detector. Base: validade
  temporal do [Graphiti](https://github.com/getzep/graphiti).
- **Ensaio de mudança.** "Se eu trocar esta escolha, o que reviso?" por dependências
  registradas, sem prometer impacto completo num grafo parcial.
- **Retomada orientada à tarefa.** Ao voltar, o que mudou desde um marco; reúsa
  Contexto e Visão. É o primeiro ganho curto.
- **Recibo de contexto.** Explicar uma entrega real: decisão, versão, motivo da
  inclusão e o que ficou de fora.
- **Memória das alternativas rejeitadas.** Por que a opção caiu e quando voltaria a
  valer, para o agente não repetir a proposta.
- **Contexto por branch.** Separar experimento de decisão em vigor; só quando houver
  caso real de confusão.
- **Conhecimento geral do dev (padrões entre projetos).** Situação, resposta,
  condições de aplicabilidade, proveniência e estado; nasce como candidato na
  Revisão e nada sai de um projeto sem aceite. Usos, do mais barato ao mais caro:
  análogos na Revisão, padrões ao criar um projeto, injeção no contexto (só depois
  de medir que ajuda), aviso de consistência. Cuidados: isolar clientes, busca
  local primeiro, padrões envelhecem.
- **Quadro de hipóteses, dossiê de reprodução e memória dos caminhos que não
  explicaram o bug.** Base: [METR 2025](https://metr.org/blog/2025-07-10-early-2025-ai-experienced-os-dev-study/)
  (a percepção de velocidade diverge do medido) e o custo de retomar interrupções
  em [Parnin e Rugaber](https://chrisparnin.me/pdf/parnin-icpc09.pdf).
- **Intenção antes do diff, diff por intenção e mapa de verificação (obrigação para
  evidência).** Revisar entendendo o que precisa ser verdadeiro. Base:
  [Anthropic, habilidades e IA](https://www.anthropic.com/research/AI-assistance-coding-skills)
  (terminar a tarefa não é compreender o código) e
  [Perry et al.](https://arxiv.org/html/2211.03622v3) (confiança excessiva em código
  gerado).
- **Envelope de delegação por tarefa.** Objetivo, limites e critérios para um agente
  autônomo. Base: [METR 2026](https://metr.org/blog/2026-02-24-uplift-update/).
- **Atlas de exemplos internos e glossário domínio-símbolo.** Precedentes do próprio
  projeto com validade e escopo; equivalências confirmadas.
- **Onboarding e primeira tarefa com explicação de volta.** Base:
  [dificuldades do primeiro emprego](https://www.microsoft.com/en-us/research/publication/struggles-of-new-college-graduates-in-their-first-software-development-job/).
- **Dossiê de migração de dependência** (base:
  [quebras de API](https://homepages.dcc.ufmg.br/~mtov/pub/2017-saner-breaking-apis.pdf))
  e **memória da triagem de avisos** (base:
  [por que não usam análise estática](https://research.google/pubs/why-dont-software-developers-use-static-analysis-tools-to-find-bugs/)).
- **Máquina do tempo do projeto** ("como era em 15/09?"), **guardião de mudança**
  (`xemnas check` sobre um diff), **pergunte ao projeto** (só com decisões citadas),
  **perspectivas salvas do grafo**, **publicar como site estático**, **placar por
  agente** e **voz do mascote só com dado real**.

## Estudos de dores

Quatro levantamentos de 32 estudos (cognição e interrupções, conhecimento de equipe,
IA e verificação, contratos e reprodução). Os que mais pesam:

- Retomar tarefas interrompidas: [Parnin e Rugaber](https://chrisparnin.me/pdf/parnin-icpc09.pdf),
  [pistas de memória](https://chrisparnin.me/pdf/cues-chi09.pdf).
- Perguntas difíceis sobre código: [Ko et al.](https://faculty.washington.edu/ajko/papers/Ko2007InformationNeeds.pdf),
  [perguntar e responder](https://www.cs.ubc.ca/~murphy/papers/other/asking-answering-fse06.pdf).
- Coordenação em equipes: [Cataldo et al.](https://herbsleb.org/web-pubs/pdfs/cataldo-socio-2008.pdf).
- IA e produtividade: [METR 2025](https://metr.org/blog/2025-07-10-early-2025-ai-experienced-os-dev-study/),
  [experimentos de campo](https://www.microsoft.com/en-us/research/publication/the-effects-of-generative-ai-on-high-skilled-work-evidence-from-three-field-experiments-with-software-developers/).
- Depuração real: [Debugging revisited](https://www.microsoft.com/en-us/research/publication/debugging-revisited-toward-understanding-debugging-needs-contemporary-software-developers/).

Ressalva: parte das fontes teve acesso parcial; nenhuma foi validada no Xemnas.

## Referências de design

Zeron ([zeronsh/zeron](https://github.com/zeronsh/zeron)) para desempenho e
movimento em GPUI; Raycast, Linear e Superhuman para acabamento e teclado. A
lição comum: acabamento por disciplina, não por enfeite. As regras que viraram
padrão estão em `docs/design/VISUAL-IDENTITY.md`.

## O que já foi descartado

- Outro chatbot genérico sobre o repositório, feed de centenas de observações,
  ADR gerado e tratado como aprovado, plataforma de revisão de PR, score opaco de
  "saúde arquitetural", ranking de pessoas, marketplace de memórias.
- Embeddings no contexto (reprovados no v4 e no v5; ver `busca-semantica-local.md`).
- Fork amplo do GPUI e uso do `zui` (crate com licença GPL, base antiga, custo de
  rebase; o Quiet Glass não depende de blur).
- Expansão de consulta por IA a cada prompt (query2doc) e RM3: custo por prompt e
  mais ruído em coleção pequena.
- Biblioteca global com OCR amplo, chat livre, muitas integrações e decoração
  adicional enquanto o núcleo não tem confiança cotidiana.
