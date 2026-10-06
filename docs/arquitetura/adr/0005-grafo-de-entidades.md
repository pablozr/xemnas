# ADR-0005: Grafo de entidades do projeto

- **Status:** Vigente. Estendida pelo [ADR-0014](0014-regras-com-escopo-por-componente.md) (escopo de regra). (registro original: aceito)
- **Data:** 2026-09-30
- **Contexto:** a busca de contexto é lexical (FTS5, AD-14). Quando o pedido ao agente não cita as palavras certas ("corrige esse bug"), o contexto certo não vem, e não há como perguntar "o que vale para este arquivo?" ou "o que depende do SQLite?". O plano `docs/roadmap/fase-4/00-plano-grafo-de-entidades.md` propunha ligar decisões e claims às coisas concretas do projeto. O usuário decidiu implementar o plano completo agora, antes do uso diário, em vez de esperar os sinais de uso real que o plano pedia.
- **Decisão:** um grafo tipado, temporal e confirmado por humano, guardado no SQLite do app, sem banco de grafo e sem extração por IA.

## Modelo

| Peça | Campos | Regra |
| --- | --- | --- |
| **Entidade** | projeto, tipo (`component` ou `technology`), nome, chave normalizada, descrição, criada em, aposentada em | chave = nome em minúsculas só com letras e dígitos; única por projeto e tipo; aposentar nunca apaga |
| **Padrão de caminho** | entidade (componente), glob relativo | como o CODEOWNERS: `*`, `**`, `?`; sem `..`, sem caminho absoluto |
| **Alias** | entidade, texto, chave | "SQLite 3" e "sqlite" resolvem para a mesma tecnologia |
| **Aresta** | tipo, origem (decisão, claim ou entidade), entidade alvo, procedência (`human` ou `derived`), motivo, criada em, confirmada em, invalidada em | nunca apagada: invalidar marca `invalidated_at` |

Tipos de aresta:

- `affects`: decisão → componente;
- `uses`: decisão → tecnologia;
- `applies_to`: claim → componente ou tecnologia;
- `part_of`: componente → componente, sem ciclo e com **um pai** por componente (o contexto é herdado por uma única cadeia).

Uma aresta `human` nasce confirmada. Uma `derived` nasce como **sugestão** (sem `confirmed_at`) e só vale depois de confirmada. Rejeitar uma sugestão a invalida, e a derivação não a propõe de novo.

## Como o grafo nasce (sem IA)

- **Componentes propostos** a partir dos arquivos que as decisões realmente tocaram (`diff_summary.files` do candidato de origem): o prefixo `crates/<nome>`, `apps/<nome>`, `packages/<nome>`, `adapters/<nome>`, `libs/<nome>`, `services/<nome>` ou `modules/<nome>`, senão a primeira pasta. A proposta é identificada pelo caminho: quando duas pastas têm o mesmo nome (`apps/api`, `services/api`) ou o nome já existe, ela se chama pelo caminho. Vira entidade só com um clique do usuário.
- **Evidência da própria decisão (2026-10-01).** Arquivos e dependências de uma decisão vêm dos artefatos que ela cita, não da captura inteira: duas decisões do mesmo turno não herdam os arquivos uma da outra. Proposta única que só cita a conversa continua com os diffs da captura.
- **Sugestões `affects`**: arquivos de uma decisão vigente que casam com o padrão de um componente.
- **Sugestões `applies_to` (2026-10-01)**: uma regra adotada da Revisão guarda o candidato de origem (`source_candidate_id`, migration 18) e é sugerida para os componentes que a evidência dela tocou, enquanto vale.
- **Tecnologias propostas e sugestões `uses`**: dependências adicionadas em `Cargo.toml` ou `package.json` nos hunks citados pela decisão, resolvidas por chave ou alias. O parser lê a seção do manifest (`[dependencies]`, `"devDependencies"`); `[package.metadata]`, `"engines"` e afins não são dependências, troca de versão não é adição, e sem seção visível só conta valor com cara de versão.
- **Sugestões por menção (2026-10-03).** O texto de uma decisão vigente (pergunta, escolha, motivo, premissas, escopo e consequências) é lido em busca dos itens vivos do mapa: nome, aliases e a parte literal dos padrões de caminho (`crates/core/**` → `crates/core`). A comparação ignora caixa e acento e exige palavra inteira (`-` e `_` continuam a palavra, então `storage` não casa com `storage-sqlite`). Caminho sempre conta; nome ou alias com 4 ou mais caracteres conta em qualquer lugar; nome de 3 (`api`, `cli`) só em maiúsculas ou entre crases ou aspas; menor que isso nunca. Cada menção vira `affects` (componente) ou `uses` (tecnologia) com o motivo `citado no texto: "<trecho>"`, sujeita às mesmas regras de sugestão; o que já tem aresta pelo arquivo ou pela dependência mantém esse motivo. No modo automático (`auto_approval`), vínculo por menção não é aceito pelas regras: vai ao juiz de IA com o trecho; vínculo por arquivo ou dependência continua aceito sem chamada.
- **Documentação é evidência, não a parte afetada (2026-10-03).** Arquivos de documentação (`.md`, `.mdx`, `.markdown`, `.rst`, `.adoc` em qualquer lugar; `.txt` só dentro das pastas de documentação, porque `requirements.txt` é manifesto) não geram `affects`/`applies_to`, proposta de componente nem vínculo na prévia da Revisão. Uma decisão tirada de um ADR chega às partes de que fala pelas menções do seu texto. Vínculos com a pasta de documentação já confirmados continuam.
- **Identidade**: a chave mantém `+` e `#`, então C, C++ e C# são itens distintos.
- **Ação do usuário**: qualquer entidade, padrão, alias ou aresta, confirmada na hora.

- **Componentes declarados (2026-10-01).** Membros do workspace Cargo (`[workspace] members`), `workspaces` do `package.json` e `packages` do `pnpm-workspace.yaml` são lidos dos manifestos do projeto (padrões `pasta` e `pasta/*`), com nome e descrição do pacote. Com o mapa sem componentes, eles são **criados automaticamente**, uma vez; depois, membros novos aparecem como propostas ("declarado no workspace Cargo", com "Criar os N do workspace"). Pastas inferidas dos diffs continuam só propostas.

A derivação é determinística e roda ao abrir o Mapa, depois de cada mudança nele e **ao adotar um candidato**. Do disco do repositório ela só lê os manifestos de workspace citados acima; o resto vem do que o app já capturou.

### Contexto derivado das decisões (revisão de 2026-10-01)

Depois de adotar uma decisão, outro job (`derive_claims`) pede ao provedor no máximo 3 itens de contexto que ela afirma (premissa, restrição, convenção ou objetivo), cada um com citação literal do texto da decisão; o app confere a citação, o tipo, o tamanho e se o projeto já tem a mesma regra (migration 20, `claim_suggestions`). Em Sugestões do Mapa, confirmar cria a regra com a decisão como origem (`source_decision_id`) e `applies_to` nos itens que a decisão afeta; rejeitar não volta. Quando a decisão de origem é substituída, a regra derivada aparece em Contexto › Regras com "Revisar": nada é encerrado sozinho.

### Relações entre decisões sugeridas (revisão de 2026-10-01)

Depois de adotar uma decisão, um job em segundo plano (`suggest_relations`) busca até 6 decisões anteriores em vigor que dividem um item do mapa com ela ou palavras (FTS), e pede ao provedor configurado que julgue cada par: `depends_on`, `conflicts_with`, `supersedes` ou nada. Toda relação precisa de uma **citação literal** do texto das decisões; o app confere a citação, os IDs, a direção (substituir é sempre da nova para a antiga), ciclos e estado antes de guardar a sugestão (migration 19, `relation_suggestions`). Ela aparece em Sugestões do Mapa com a citação e o motivo; só vira relação ao ser confirmada, e uma rejeitada não volta. Sem provedor ativo, nada é julgado. Similaridade escolhe candidatas; nunca decide a relação.

### Adoção com vínculos (revisão de 2026-10-01)

Antes, adotar uma decisão não criava vínculos: até alguém abrir o Mapa e confirmar as sugestões, o contexto por arquivo não a encontrava. Agora a Revisão mostra, junto do candidato, os vínculos que a própria evidência aponta para itens que já existem (`affects`/`uses` para decisão, `applies_to` para regra), todos marcados. Confirmar o candidato confirma também os marcados; os desmarcados são rejeitados e não voltam como sugestão; o resto do projeto é derivado na hora (`application::adoption`). Arquivos sem componente aparecem como tais. A confirmação continua humana: nada é ligado sem estar visível no momento da adoção.

## Consultas

Todas aceitam `as_of`. Uma aresta vale em `t` quando foi confirmada até `t` e não foi invalidada até `t`. Uma entidade existe entre a criação e a aposentadoria. Uma decisão vale até ser substituída.

- **Mapa do projeto**: componentes e tecnologias vivos, com decisões vigentes, claims válidas, última atividade e conflitos por entidade, mais as arestas `part_of`.
- **Vizinhança**: um nó (decisão, claim ou entidade) e o que está a até 2 saltos (arestas do grafo e relações entre decisões), truncada em N nós com o aviso `truncated`.
- **Lente de arquivo**: caminho → componentes cujo padrão casa → decisões vigentes que os afetam e claims válidas que se aplicam.
- **Impacto**: a partir de uma entidade ou decisão, as decisões que dependem dela, transitivamente (`depends_on`).
- **Linha do tempo**: eventos datados do projeto (decisão confirmada ou substituída, claim começa ou termina, entidade criada ou aposentada, aresta confirmada ou invalidada) num intervalo.
- **Conflitos**: pares `conflicts_with` vigentes que tocam a mesma entidade.

## Uso pelo agente

- **Na edição (principal):** o plugin do OpenCode usa o gancho `tool.execute.after` nas ferramentas que mudam arquivos (`edit`, `write`, `multiedit`, `patch`, `apply_patch`) e pede `POST /v1/context` com `trigger: "edit"` e os arquivos editados. O app devolve só as decisões e regras que o mapa liga aos componentes desses arquivos, e o plugin as acrescenta ao resultado da ferramenta, que o agente lê. Cada item vem uma vez por sessão (a deduplicação da injeção), nada vem para arquivos fora de componentes, e o modo do projeto vale (Medir só registra). Leituras (`read`) não disparam: o agente lê muito mais do que edita.
- **No prompt:** o plugin lembra os arquivos que a sessão editou por último e os envia junto do prompt, então um pedido de continuação ("agora ajusta aquilo") recebe o que vale para eles; caminhos citados no prompt também contam.

- O Context Pack aceita arquivos: decisões e claims ligados aos componentes desses arquivos entram antes da busca lexical, dentro do mesmo orçamento.
- O MCP ganha `file_context` (lente de arquivo) e `search_context` aceita `path`, ambos somente leitura, pela API local.

## Interface

Uma aba **Mapa** no projeto (Ctrl 4 e paleta), no padrão de `docs/design/VISUAL-IDENTITY.md`: lista de componentes e tecnologias à esquerda, detalhe da entidade (decisões, claims, partes, conflitos, impacto) na coluna de leitura, caixa de sugestões para confirmar ou rejeitar, lente de arquivo e linha do tempo. Nunca o grafo inteiro de uma vez: sempre um nó e sua vizinhança. *Revisto em 2026-09-30:* além dos blocos, o Mapa ganhou a visão **Grafo** com o mapa inteiro (`KnowledgeGraph::project_graph`), em layout de forças assentado e estável; o detalhe de um item continua mostrando só a vizinhança. Regras visuais em `VISUAL-IDENTITY.md`.

## Consequências

- `domain` ganha `entities` (tipos, chave normalizada, glob e regras de aresta), sem dependências novas.
- Migration 14 cria `entities`, `entity_patterns`, `entity_aliases` e `entity_edges`; apagar o projeto apaga o grafo dele, e o impacto da remoção conta entidades.
- `application::graph` concentra casos de uso e consultas; `storage-sqlite` implementa o port.
- A qualidade das sugestões depende dos diffs capturados; sem diff, a sugestão só vem das menções no texto da decisão, mais fracas (pedem confirmação) que as de arquivo.

## Alternativas rejeitadas

- **Grafo extraído por LLM (GraphRAG):** caro, com entidades duplicadas e relações não confiáveis; contraria a autoridade humana sobre o que vale.
- **Banco de grafo ou grafo genérico de nós e arestas:** o volume é pequeno, o SQLite com `WITH RECURSIVE` basta, e a spec pede relações tipadas e auditáveis (ADR-0003).
- **Grafo de símbolos (chamadas, imports):** ferramentas de código já fazem isso; o xemnas guarda o porquê, no nível de componente e caminho.
- **Ler a árvore do repositório para propor componentes:** o app não lê o repositório hoje; os arquivos das decisões já mostram onde o trabalho acontece.

## Estado hoje

O modelo, a derivação sem IA, as consultas e a interface descritos acima seguem em vigor (`crates/application/src/graph/`). Duas mudanças: as regras agora respeitam o escopo por componente ([ADR-0014](0014-regras-com-escopo-por-componente.md)), e a seleção de contexto é grafo mais busca lexical, sem embeddings ([ADR-0013](0013-contexto-sem-embeddings.md)). O contexto depois de editar arquivos existe no plugin do OpenCode (`adapters/opencode/src/context.ts`).
