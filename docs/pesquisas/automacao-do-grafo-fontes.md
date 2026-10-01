# Automação do grafo de decisões e do contexto

**Data:** 2026-09-30

**Pergunta:** como automatizar relações e contexto ao adotar decisões, preservando
evidência, autoridade humana e histórico? O modelo do ADR-0005 acompanha as pesquisas?

**Status:** Em implementação. Passos 1 a 5 da ordem proposta entregues em
2026-10-01 (ver "Andamento"); 6 (medição em casos reais e embeddings) aberto.

Esta pesquisa combina auditoria do código do workspace em 2026-09-30 com fontes
primárias e o contrato do [ADR-0005](../arquitetura/adr/0005-grafo-de-entidades.md).
Inclui alterações locais ainda não commitadas. Recomendações não alteram o escopo
aprovado; nenhuma correção de comportamento foi implementada nesta investigação.

## Resposta sobre o comportamento atual

| Etapa | Hoje | Evidência no código |
| --- | --- | --- |
| Adotar candidato de decisão | Cria decisão, revisão inicial, índice FTS e evidências; não cria relações ou claims derivados | `application/src/inbox.rs::confirm`, `storage-sqlite/src/inbox.rs::confirm_one` |
| Adotar candidato de regra (alteração local em andamento) | Cria claim `constraint` com texto da escolha; não cria `applies_to` | `storage-sqlite/src/inbox.rs::confirm_one` |
| Propor componentes/tecnologias | Automático ao abrir/atualizar Mapa, a partir dos diffs capturados; entidade ainda exige ação humana | `screens/map.rs::load`, `graph/derive.rs::refresh_suggestions` |
| Ligar decisão a entidade | `affects` e `uses` são sugestões; só valem após confirmação | `graph/derive.rs::suggest`, `graph/mod.rs::EdgeRecord::holds_at` |
| Ligar claim a entidade | Manual; não há derivador de `applies_to` | `graph/mod.rs::link`, `graph/derive.rs` |
| Dependência, conflito e substituição entre decisões | Manuais, via `DecisionRelations` | `application/src/relations.rs::relate` e `supersede` |
| Recuperar contexto | Automático para vínculos vigentes e busca lexical, conforme modo do projeto e orçamento | `context.rs::build_pack`, `injection.rs::prepare` |

Os caminhos da tabela são relativos a `crates/`, exceto `screens/map.rs`, em
`apps/desktop-gpui/src/`. Contexto narrativo da decisão (`rationale`) já é persistido.
Context Claim é outro objeto: confirmar uma decisão não extrai automaticamente
suas premissas, restrições ou convenções. `scope`, `assumptions`, `consequences` e
`reconsider_when` começam como listas vazias na adoção.

Portanto, o contexto **é consultado automaticamente**, mas a cobertura do grafo
**depende de preparação e confirmação manual**. Adotar uma decisão e imediatamente
editar um arquivo pode não entregar essa decisão ao agente se o vínculo não existe.
Isso cumpre o ADR-0005, mas não a expectativa de automatizar os vínculos na adoção.

## Andamento (2026-10-01)

| Achado | Situação |
| --- | --- |
| P1 escopo da captura | Corrigido: arquivos e dependências vêm dos artefatos citados (`04635b7`) |
| P1 parser de manifest | Corrigido: seção, troca de versão e valor sem seção (`404ffaf`) |
| P1 regras sem escopo | Corrigido: `source_candidate_id` e sugestões `applies_to` (`4a00a41`) |
| P2 `as_of` com dados atuais | Aberto |
| P2 hierarquia | Pai único validado na escrita (`a057e5b`) |
| P2 identidade por nome | Chave mantém `+`/`#`; propostas pelo caminho (`a057e5b`); sem constraint única no banco |
| Passo 2: vínculos na adoção | Entregue: prévia na Revisão, confirmar liga os marcados, derivação na hora (`d3d4b34` e seguinte) |
| Passo 5: claims derivados | Entregue: contexto citado da decisão vira regra sugerida com escopo; substituir a origem marca "Revisar" |
| Passo 4: relações entre decisões | Entregue: candidatas por item do mapa e FTS, julgamento tipado com citação literal validada, fila em Sugestões; job após a adoção |
| Passo 3: entidades automáticas | Entregue para componentes declarados (workspaces Cargo/npm/pnpm): mapa vazio se monta sozinho; membros novos viram propostas. Tecnologias continuam vindo das decisões |

## Achados de implementação, por prioridade

### P1 — Escopo de captura é atribuído a cada decisão

`crates/storage-sqlite/src/graph.rs:235` carrega todos os `diff_hunk` de `capture_id`,
sem restringir pelos `evidence_links` da decisão. Além disso,
`crates/application/src/extract/mod.rs:436` recompõe `diff_summary` de toda a
evidência da captura antes de filtrar referências. O resumo de arquivos pode,
portanto, ter a mesma abrangência para decisões diferentes da mesma captura.

Exemplo: uma captura contém mudança de interface e adoção de Redis em outro
artefato. A decisão de interface pode receber sugestão `uses Redis` e `affects`
no backend mesmo quando só cita a evidência de interface. A confirmação humana
reduz o dano, mas o usuário é induzido a confirmar vínculos indevidos; automatizá-los
agora ampliaria esse problema.

**Correção proposta:** derivar arquivos e dependências dos artefatos referenciados
pela decisão. Quando um artefato reúne assuntos diferentes, guardar também o
trecho/localizador pertinente; distinguir captura inteira de evidência específica.
Achado por leitura do fluxo; não reproduzido na suíte SQLite devido ao bloqueio abaixo.

### P1 — Parser de dependências ignora a seção do manifest

`crates/application/src/graph/derive.rs:85–168` reconhece arquivo e linha adicionada,
mas não a seção TOML ou objeto JSON. São tratados como dependências:

```diff
diff --git a/Cargo.toml b/Cargo.toml
@@ -1 +1,3 @@
 [package.metadata]
+channel = "stable"
```

```diff
diff --git a/package.json b/package.json
@@ -1 +1,3 @@
 "engines": {
+  "node": ">=22"
```

Dois testes temporários contra `added_dependencies` esperavam lista vazia e
falharam, confirmando ambos os falsos positivos. Foram removidos após a investigação.
Uma troca de versão de dependência também aparece como adição porque o parser
não compara linhas removidas. Inversamente, formatos TOML legítimos fora dos
prefixos reconhecidos e dependências NPM com tags podem ser ignorados.

**Correção proposta:** obter antes/depois estruturados do manifest e comparar
objetos de dependências. Sem conteúdo suficiente no hunk, retornar desconhecido
em vez de transformar qualquer linha parecida em tecnologia.

### P1 — Regras adotadas não ganham escopo nem proveniência equivalente

A ramificação local `seed.as_rule` em `crates/storage-sqlite/src/inbox.rs:334`
cria claim usando apenas `seed.choice`, sem `applies_to`, referência persistida
ao candidato/captura nessa claim ou ligação às suas evidências. O candidato
continua no banco, mas o modelo da claim não mantém essa relação explícita.
`refresh_suggestions` percorre decisões, não claims: abrir o Mapa não resolve.

No prompt, `context.rs::build_pack` usa constraints/conventions válidas como
regras gerais de fallback. Na edição, `injection.rs::prepare` restringe aos IDs
ligados pelo grafo e pode devolver vazio. Assim uma regra específica pode aparecer
como geral no prompt e desaparecer durante a edição do arquivo pertinente.

**Correção proposta:** preservar origem/evidência e escopo na adoção de regra,
gerar `applies_to`, e distinguir explicitamente regra global de regra localizada.
É lacuna da alteração local em andamento; não atribuir à versão commitada anterior.

### P2 — `as_of` não reconstrói padrões, nomes e texto antigos

`crates/storage-sqlite/src/graph.rs::update_entity` sobrescreve a entidade e apaga
patterns/aliases antes de regravá-los. `graph/query.rs::snapshot` carrega esses
valores atuais mesmo quando recebe data passada. Decisões também são carregadas
com pergunta/escolha atuais, sem selecionar a revisão correspondente ao instante.

Exemplo: trocar o padrão de `src/old/**` para `src/new/**` muda o resultado de
uma lente de arquivo histórica anterior à troca. A consulta filtra o tempo de
existência e das arestas, mas não o estado completo daquele instante.

**Correção proposta:** versionar dados de entidade e escopo, e usar revisões de
decisão em consultas históricas. Isso é necessário mesmo sem adotar bitemporalidade.
Achado estático; a suíte SQLite ficou bloqueada antes de executar.

### P2 — Hierarquia aceita tipos e múltiplos pais que a leitura não respeita

`EdgeKind::PartOf.check` valida apenas `NodeKind::Entity` na origem.
`graph/mod.rs::check_source` não verifica se essa entidade é componente: uma
tecnologia pode ser ligada como parte de componente, contrariando o ADR.
Também não há validação de pai único; `graph/query.rs::parent` usa apenas a
primeira aresta. Se um componente tiver dois pais, a herança de contexto ignora
um deles silenciosamente, embora o mapa possa listar ambos.

**Correção proposta:** exigir componente na origem e decidir entre árvore com
pai único (validada na escrita) ou DAG com todos os pais percorridos na leitura.
Há validação de ciclos, mas ela não resolve essas duas condições. Achado estático.

### P2 — Identidade por nome descarta diferenças significativas

`domain::entities::entity_key` remove pontuação. `C`, `C++` e `C#` tornam-se `c`;
componentes `apps/api` e `services/api` também são propostos só como `api`.
O nome da primeira entidade pode impedir propor a segunda, mesmo com caminho distinto.
`check_unique` verifica no caso de uso, mas não há constraint única de chave
por projeto/tipo no banco; duas escritas concorrentes podem passar a mesma verificação.

**Correção proposta:** identidade canônica por caminho para componentes e por
ecossistema/nome de pacote para tecnologias; nome de exibição separado e aliases
com ambiguidades explícitas. Proteger unicidade e verificações concorrentes na
transação. Achado estático, sem reprodução concorrente nesta sessão.

## Sequência de evolução recomendada

### Direção acordada com o usuário em 2026-09-30

- Aprovar uma decisão deve disparar a materialização dos vínculos e do contexto,
  sem exigir visita ao Mapa ou referências escritas no prompt.
- Arquivos e alterações vêm das ferramentas capturadas (`edit`, `write`,
  `apply_patch` etc.), dos diffs e do episódio da decisão; não de instruções
  detalhadas que o usuário precisaria produzir.
- Recuperação combina grafo, busca textual e embeddings; análise semântica
  interpreta relações entre poucas decisões candidatas. Similaridade isolada
  não cria `depends_on`, `conflicts_with` ou `supersedes`.
- A mesma capacidade de embeddings será considerada para a Knowledge Library
  global de PDFs, livros e outras referências, preservando os domínios separados.
- Local first permanece obrigatório. O mínimo desejado é Windows com 8 GB de
  RAM, CPU e sem GPU dedicada. Python em componente/repositório separado é
  alternativa a avaliar; não implica hospedagem remota.
- Custo e latência orientam a seleção. A aprovação e a consulta rápida de contexto
  não devem esperar OCR, indexação de livros ou inferência semântica longa.

Este registro captura a direção acordada; ainda não escolhe biblioteca, modelo,
limiares de automação ou política final de autoridade dos vínculos inferidos.
O ADR-0005 descreve o comportamento implementado e deverá ser revisado no trabalho
de implementação. Nenhuma destas mudanças já está entregue.

O levantamento de mercado, custos e latência desta direção está em
[memoria-semantica-local-first.md](memoria-semantica-local-first.md), com notas
específicas para runtimes, biblioteca de documentos e produtos existentes.

### Ordem de implementação proposta

1. Corrigir evidência, parser, identidade, escopo das regras e contrato temporal.
2. Na adoção, gravar evento durável para materialização idempotente do grafo,
   incluindo backfill. Exibir falhas/repetição e assegurar contexto imediato quando
   o processamento ainda estiver pendente.
3. Gerar entidades e escopo observado automaticamente. Fazer a revisão dos vínculos
   normativos junto da adoção, com política explícita, reduzindo a segunda fila no Mapa.
4. Para relações entre decisões, recuperar candidatos por projeto, escopo,
   entidades e texto; pedir inferência tipada com evidência, validada pelo domínio.
   IA pode detectar candidatos a dependência/conflito/substituição, sem confundir
   similaridade com a relação final. Substituição altera vigência e merece decisão explícita.
5. Para contextos, extrair claims tipados com escopo, origem e condições de término.
   Substituir uma decisão deve iniciar revisão dos claims derivados dela; não basta
   manter a claim vigente indefinidamente nem encerrar automaticamente toda claim citada.
6. Medir recuperação em casos reais antes de adotar embeddings ou bibliotecas novas.

Essas mudanças exigem revisar o ADR-0005, que hoje determina confirmação individual
e ausência de extração por IA. A automação proposta preserva a escolha de SQLite.

## Validação da auditoria

- `cargo test --locked -p application --lib graph`: 1 teste passou.
- `cargo test --locked -p domain entities`: 6 testes passaram.
- `cargo test --locked -p architecture`: 12 testes passaram.
- Dois testes temporários de falsos positivos do parser: ambos falharam no resultado
  esperado (lista vazia), reproduzindo os problemas. Não ficaram no repositório.
- `cargo test --locked -p storage-sqlite --test graph`: bloqueado na compilação
  por E0062, campos `kind`, `significance` e `criteria` duplicados em
  `crates/storage-sqlite/src/inbox.rs:70–75`, arquivo já modificado ao iniciar.
  A falha não demonstra regressão no grafo; impediu validar suas consultas por execução.
- Não houve alteração funcional, captura visual ou validação de interface.
  A revisão é do estado do workspace, não comparação de PR/branch com ponto base.

## O que as fontes demonstram

### GraphRAG: recuperação por entidades, relações e evidências

O artigo da Microsoft trata especialmente de perguntas globais sobre grandes
coleções de texto. Constrói um grafo de entidades e resumos de comunidades para
produzir respostas agregadas. A avaliação reportada é sobre abrangência e diversidade
das respostas, não sobre correção de relações arquiteturais em código.
Fonte: [artigo GraphRAG, versão 2](https://arxiv.org/abs/2404.16130v2).

A busca local identifica entidades relacionadas à consulta, expande relações e
recupera também trechos originais. Prioriza e filtra esse conjunto para caber num
orçamento de contexto. Portanto, preservar a ligação à evidência e combinar grafo
com texto é parte do método; entregar apenas um resumo do grafo perde informação.
Fonte: [Microsoft, Local Search](https://microsoft.github.io/graphrag/query/local_search/).

O método Standard extrai entidades e relações com LLM. FastGraphRAG substitui parte
disso por NLP e coocorrência; a própria documentação descreve o resultado como mais
ruidoso e menos apropriado para exploração fiel de entidades. Coocorrência não é
uma demonstração de dependência, conflito ou substituição entre decisões.
Fonte: [Microsoft, Indexing Methods](https://microsoft.github.io/graphrag/index/methods/).

### Graphiti/Zep: identidade, tempo e atualização incremental

O artigo Zep preserva episódios originais e liga artefatos semânticos às fontes.
Resolve entidades por candidatos obtidos com busca textual e vetorial, seguidos
de avaliação por LLM. Separa tempo do domínio de tempo de ingestão; usa LLM também
para detectar contradições e invalidar fatos. Recuperação combina texto, vetores
e expansão por vizinhança, com reordenação posterior. Sua avaliação é de memória
conversacional, não de dependências arquiteturais.
Fonte: [artigo Zep, seções 2–4](https://arxiv.org/html/2501.13956v1).

A documentação atual distingue quatro instantes: `created_at` e `expired_at`
registram quando o sistema soube da informação; `valid_at` e `invalid_at`
descrevem sua validade no domínio. Ela explicita que esses campos não estabelecem
a verdade da fonte nem do fato derivado.
Fonte: [Zep, Facts](https://help.getzep.com/facts).

O projeto Graphiti oferece ontologia prescrita, ingestão incremental e busca
híbrida. É possível aproveitar essas ideias sem incorporar sua biblioteca,
serviços ou banco de grafo.
Fonte: [repositório oficial Graphiti](https://github.com/getzep/graphiti).

### ADRs e proveniência

Nygard separa contexto, decisão, status e consequências. Uma decisão revertida
continua registrada como substituída, com referência à substituta. Consequências
de uma decisão podem formar contexto de decisões posteriores; isso explica a
utilidade de relacioná-las, mas não fornece um algoritmo que determine essas
relações automaticamente.
Fonte: [Nygard, Documenting Architecture Decisions](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions).

PROV-O distingue entidade, atividade e agente, com relações de derivação, geração
e atribuição. O padrão de qualificação permite explicar a atividade que produziu
uma relação. É um fundamento para rastreabilidade, sem exigir RDF no xemnas.
Fonte: [W3C, PROV-O](https://www.w3.org/TR/prov-o/).

## Comparação com o contrato atual

**Fatos locais do ADR-0005:** o grafo é tipado, guardado em SQLite, sem extração
por IA. Entidades e sugestões derivam de diffs capturados. Arestas derivadas só
valem depois de confirmação humana. A derivação roda ao abrir o Mapa e após
mudanças nele. O contexto por arquivo usa componentes com padrões e arestas
vigentes; não encontra vínculos que ainda sejam sugestões.

**Avaliação:** preservar histórico, limitar tipos, identificar entidades e
recuperar contexto localizado são escolhas compatíveis com as fontes. O desenho
é um grafo de memória decisional; não é a implementação integral de GraphRAG ou
Graphiti. As pesquisas não exigem trocar SQLite por um banco de grafo.

**Lacuna de produto:** pelo contrato escrito, adotar uma decisão não equivale a
materializar e tornar utilizáveis todos seus vínculos. Sem abrir o Mapa e resolver
sugestões, a recuperação por arquivo pode ficar sem informação pertinente. Isso
precisa ser confrontado com o código e com o fluxo atual de adoção.

**Lacuna temporal possível:** criação/confirmação/invalidação modelam o ciclo
conhecido pelo sistema. Não devem ser chamados de modelo bitemporal completo
sem separar a vigência no domínio e versionar as correções. Para decisões cuja
vigência começa exatamente na adoção, um único eixo pode ser suficiente; importação
histórica e informações tardias demandam outra semântica.

## Recomendações para o xemnas

Estas são propostas derivadas da comparação, não conclusões experimentais das fontes.

1. **Atualizar o grafo na adoção.** Usar o evento de adoção para materializar
   propostas de entidades e vínculos, com processamento idempotente, repetição em
   caso de falha e recomposição de decisões antigas. Não depender da visita ao Mapa.
2. **Separar observação de regra.** Um diff demonstra que um arquivo foi tocado;
   não demonstra que toda a decisão vale para todo o componente. Se desejado,
   automatizar uma relação de escopo observado e mostrar sua fonte. Uma relação
   normativa `affects` só deve ganhar essa autoridade conforme política explícita.
3. **Reduzir cliques na adoção.** Mostrar os vínculos sugeridos junto da decisão
   e permitir confirmá-los na mesma ação. Automatizar confirmação de vínculos
   evidenciais exigiria atualizar o ADR; aprovação da decisão não deve confirmar
   inferências invisíveis sem essa política.
4. **Tratar relações semânticas separadamente.** Arquivo, componente ou tecnologia
   compartilhados podem gerar candidatos a relações entre decisões. Não bastam
   para provar `depends_on`, `conflicts_with` ou substituição. LLM pode propor um
   tipo acompanhado do trecho que o sustenta; o caso de uso valida IDs, projeto,
   tipos, ciclos e estado antes de persistir. Confiança do modelo não é autoridade.
5. **Resolver identidade antes de expandir.** Preferir identificadores existentes,
   aliases e caminhos estruturados. Usar similaridade apenas para candidatos de
   correspondência, preservando ambiguidades. Normalizar nomes removendo sinais
   pode colidir conceitos, como `C`, `C++` e `C#`; o teste deve cobrir esses casos.
6. **Recuperar contexto por múltiplos sinais.** Combinar arquivos, relações vigentes
   e FTS; quando houver necessidade medida, acrescentar embeddings para paráfrases.
   Aplicar filtros de validade antes da seleção, deduplicar e reservar orçamento
   para evidências. Limitar saltos e informar truncamento e ausência de cobertura.
7. **Manter proveniência estruturada.** Registrar evidência de origem, versão da
   regra de derivação, momento e autor da confirmação. Novas evidências devem
   permitir distinguir uma sugestão nova de uma rejeitada anteriormente.
8. **Medir antes de ampliar.** Montar casos reais com contexto esperado: decisão
   adotada antes de abrir Mapa, decisão sem diff, alteração de dependência existente,
   aliases ambíguos, claims, componente pai/filho, substituição e contexto corrigido
   depois da primeira injeção. Medir cobertura, itens indevidos, frescor, latência
   e orçamento usado; ausência de erro técnico não comprova relevância.

## Limitações

- Fontes web foram abertas e conferidas nesta data; páginas e branches principais
  dos projetos podem mudar. O artigo Zep consultado é a versão 1 de 2025, e o
  GraphRAG é a versão 2 de 2025; não confundir esses resultados com o produto atual.
- Documentação de fornecedores demonstra desenho e funcionamento declarado;
  não substitui avaliação independente no corpus do xemnas. Benchmarks de
  conversas e sumarização não comprovam melhoria em decisões de software.
- Os achados locais distinguem reprodução por teste de leitura estática. Não houve
  medição de desempenho nem validação completa da integração SQLite nesta sessão.
- Nenhuma mudança de comportamento foi implementada por esta pesquisa.
