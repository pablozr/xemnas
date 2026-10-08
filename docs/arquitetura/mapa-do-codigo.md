# Mapa do código

Entradas atuais para localizar um fluxo sem percorrer toda a árvore. Consulte
somente a linha relevante e suas dependências diretas; a divisão-alvo está na
[referência arquitetural](stack-e-arquitetura-rust-gpui.md), não neste mapa.
Os caminhos abaixo são relativos a este documento; símbolos são pistas de busca,
não promessas de âncoras de linha estáveis.

## Backend e testes focados

| Fluxo | Entrada e símbolos | Testes |
| --- | --- | --- |
| Worktrees do git | [repo_identity.rs](../../crates/application/src/repo_identity.rs): `RepoIdentity`, cache; `find_project_with_identity` em [projects.rs](../../crates/application/src/projects.rs); hook em [worktree.rs](../../apps/mcp-server/src/hook/worktree.rs) | testes em `repo_identity.rs` e `worktree.rs` |
| Captura | [captures.rs](../../crates/application/src/captures.rs): `CaptureApi`, ingestão | [captures.rs](../../crates/storage-sqlite/tests/captures.rs), [full_cycle.rs](../../crates/storage-sqlite/tests/full_cycle.rs) |
| Grafo e mapa | [graph/mod.rs](../../crates/application/src/graph/mod.rs): `KnowledgeGraph`; [query.rs](../../crates/application/src/graph/query.rs): `project_map`; [derive.rs](../../crates/application/src/graph/derive.rs): `added_dependencies`; [discover.rs](../../crates/application/src/graph/discover.rs): `declared_components`, `assemble`, `merge_package_aliases` (apelidos derivados do pacote); [ai_link.rs](../../crates/application/src/graph/ai_link.rs): `ai_link_reason`, `ai_link_quote` (motivo de vínculo proposto pela IA) | [graph.rs](../../crates/storage-sqlite/tests/graph.rs); testes unitários em `derive.rs` |, [graph.rs](../../crates/storage-sqlite/tests/graph.rs); testes unitários em `derive.rs` e `ai_link.rs` |
| Vínculos decisão → componente (menção afirmativa; sinais estruturais conferidos no repositório) | [graph/mention.rs](../../crates/application/src/graph/mention.rs): `entity_terms`, `Folded::mention` (só ocorrência afirmativa: gatilhos de negação `NEGATION_BEFORE`/`NEGATION_AFTER`, escopo até pontuação, palavra de terminação ou `MAX_SCOPE_WORDS`; nome dentro de outro caminho não conta; nome igual ao do projeto só vale como caminho ou código, `Term::restricted`); [graph/discover.rs](../../crates/application/src/graph/discover.rs): `project_names` e `project_keys` (pasta, `[package].name` e `name` da raiz); [graph/derive.rs](../../crates/application/src/graph/derive.rs): `dependency_owners`/`suggest_dependencies` (`DEPENDENCY_REASON`; a dependência citada liga ao componente cujo manifest a declara, até `MAX_OWNERS` donos; `triage_link` aceita quando só um declara); `DeclaredComponent::dependencies` em [discover.rs](../../crates/application/src/graph/discover.rs); [graph/repo_files.rs](../../crates/application/src/graph/repo_files.rs): `RepoFiles` (listagem por `git ls-files` ou caminhada limitada, `resolve`: exato, sufixo único, ambíguo ou ausente), `counted_files` (o arquivo de uma decisão só conta se existe e, numa decisão tirada de documento, se o texto dela o cita), usado por `refresh_suggestions` e por `Adoption::preview`; [graph/cache.rs](../../crates/application/src/graph/cache.rs): `FolderCache` (resposta por pasta, TTL 300 s, até 32 pastas); [link_suggestions.rs](../../crates/application/src/link_suggestions.rs): `link_request` marca o componente "same name as the project" | `mention_quality_gate`, `a_part_named_to_be_left_out_is_not_a_mention` e vizinhos em `mention.rs`; [link_structure_corpus.rs](../../crates/storage-sqlite/tests/link_structure_corpus.rs) |
| Conflitos da revisão automática | [auto_approval.rs](../../crates/application/src/auto_approval.rs): `conflicts_with` do veredito (`Judged`, `parse_verdicts`, `name_items`, `conflict_named`, `nearest` para até 3 decisões em vigor parecidas), `Conflict` no `Entry` (migração 0045); [conflicts.rs](../../crates/application/src/conflicts.rs): `Conflicts::view`/`resolve` (`Resolution`: ficar com esta, com a outra, as duas com escopo), exposto por `ApprovalsApi::conflict`/`resolve`; painel em [inbox/conflict.rs](../../apps/desktop-gpui/src/screens/inbox/conflict.rs) com `compare_pair`/`compare_card` de [patterns.rs](../../apps/desktop-gpui/src/ui/patterns.rs) | `the_judge_names_the_other_side_and_no_id_reaches_the_reason`, `a_decision_in_force_that_resembles_the_candidate_is_named_to_the_judge`, `a_doubted_ai_link_is_discarded_and_a_doubted_mention_still_waits` e o módulo `resolution` em [auto_approval.rs](../../crates/storage-sqlite/tests/auto_approval.rs); testes de unidade em `auto_approval.rs` |
| Vínculos propostos pela IA | [link_suggestions.rs](../../crates/application/src/link_suggestions.rs): job `suggest_links` (`LinkFinder`, `LINK_PROMPT`, `parse_links`, `needs_links`, `claim_needs_links` e `queue_untied_claims` para regras permanentes sem ligação), enfileirado na adoção em [adoption.rs](../../crates/application/src/adoption.rs); registrado em `apps/desktop-gpui/src/main.rs` | [link_suggestions.rs](../../crates/storage-sqlite/tests/link_suggestions.rs), [link_corpus.rs](../../crates/storage-sqlite/tests/link_corpus.rs) (`link_quality_gate`, fixture gerada por [link_suggestions_live.rs](../../crates/ai-provider/tests/link_suggestions_live.rs)), `a_link_the_ai_proposed_is_judged_with_its_quote_and_reason` em [auto_approval.rs](../../crates/storage-sqlite/tests/auto_approval.rs) |
| Lotes e prioridade dos jobs de IA | [batching.rs](../../crates/application/src/batching.rs): `BATCH_SIZE`, `answers` (entradas numeradas de uma resposta em lote), `project_groups`; [jobs.rs](../../crates/application/src/jobs.rs): `JobKind::priority`, `Jobs::register_batch`, `run_next_group`, `JobRepository::claim_more`; `run_many` de `LinkFinder`, `RelationFinder`, `ClaimFinder` e `run_projects` de `SearchTermFinder`; `ProviderLimiter::calls` em [limiter.rs](../../crates/application/src/limiter.rs); ordem de claim e `claim_more` em [jobs.rs](../../crates/storage-sqlite/src/jobs.rs); registrados em `apps/desktop-gpui/src/main.rs` e no runner [e2e_pipeline.rs](../../apps/desktop-gpui/tests/e2e_pipeline.rs) | [batched_jobs.rs](../../crates/storage-sqlite/tests/batched_jobs.rs) (portão de chamadas), [jobs.rs](../../crates/storage-sqlite/tests/jobs.rs) (`suggestion_jobs_run_links_first_then_relations_rules_and_terms`, `a_batch_takes_the_oldest_queued_jobs_of_its_kind_and_settles_each`), testes em `batching.rs` |
| Context Pack | [context.rs](../../crates/application/src/context.rs): `ContextProvider`, `ContextPacks`, `MAX_TIED_RULES`, `MAX_GLOBAL_RULES`; [graph/query.rs](../../crates/application/src/graph/query.rs): `KnowledgeGraph::pack_graph`; [graph/scope.rs](../../crates/application/src/graph/scope.rs): `scopes_in`; [context_settings.rs](../../crates/application/src/context_settings.rs): `ContextMode`, `ContextSettings`; [terms.rs](../../crates/application/src/terms.rs): `TaskTerms` (plurais e ponte PT/EN); [search_terms.rs](../../crates/application/src/search_terms.rs): `SearchTermFinder` (termos de busca gerados na adoção, último recurso) | [context_pack.rs](../../crates/storage-sqlite/tests/context_pack.rs), [context_settings.rs](../../crates/storage-sqlite/tests/context_settings.rs), [context_corpus.rs](../../crates/storage-sqlite/tests/context_corpus.rs) (`context_quality_gate`, `sealed_v4_quality_gate`, `calibration_v5_quality_gate`), [context_scale.rs](../../crates/storage-sqlite/tests/context_scale.rs) (`rule_ranking_scales_to_a_large_project`), [scoped_rules.rs](../../crates/storage-sqlite/tests/scoped_rules.rs), [dogfood_context.rs](../../crates/storage-sqlite/tests/dogfood_context.rs) (dados reais, só local), [search_terms.rs](../../crates/storage-sqlite/tests/search_terms.rs) |
| Substituição de decisão | [relations.rs](../../crates/application/src/relations.rs): `DecisionRelations::supersede`; [decisions.rs](../../apps/desktop-gpui/src/screens/decisions.rs): `Action::Relate`, “Substitui” | [relations.rs](../../crates/storage-sqlite/tests/relations.rs) |
| Robustez do pipeline (mapa desde o registro, nova tentativa de análise de documento, motivo da falha) | `KnowledgeGraph::prepare`, `map_preparer` em [derive.rs](../../crates/application/src/graph/derive.rs), ligados por `Projects::with_map_preparer` ([projects.rs](../../crates/application/src/projects.rs)) e `Documents::with_map_preparer`; `DocumentStore::requeue_failed_analysis`, `MAX_AUTOMATIC_ATTEMPTS` em [documents.rs](../../crates/application/src/documents.rs); `failure_detail` e `AssessmentRecord::error_detail` (migração 0046) em [assessment.rs](../../crates/application/src/extract/assessment.rs); `CaptureProgress::failure_detail` exibido em [diagnostics.rs](../../apps/desktop-gpui/src/screens/settings/diagnostics.rs) | [pipeline_robustness.rs](../../crates/storage-sqlite/tests/pipeline_robustness.rs) |
| Importar um documento de worktree | [documents.rs](../../crates/application/src/documents.rs): `Documents::import_file`, `repository_path`; ação `pick_document` em [context.rs](../../apps/desktop-gpui/src/screens/context.rs) | `a_document_of_a_sibling_worktree_is_imported_and_proposed` em [documents.rs](../../crates/storage-sqlite/tests/documents.rs) |
| Documentos e revisão consultiva | [context.rs](../../apps/desktop-gpui/src/screens/context.rs): `ContextScreen::refresh`, `load_snapshot` | [documents.rs](../../crates/storage-sqlite/tests/documents.rs), [knowledge_review.rs](../../crates/storage-sqlite/tests/knowledge_review.rs) |

Execute um teste de integração pelo arquivo: por exemplo,
`cargo test --locked -p storage-sqlite --test graph`. Para unitários da derivação:
`cargo test --locked -p application graph::derive`.

**Efeito ao abrir Contexto:** `set_project` troca o projeto e limpa estado; não
indexa sozinho. `refresh` carrega `load_snapshot`, que chama `documents.index` e
lê a pasta. A seção `KnowledgeReview` retorna antes desse carregamento. A revisão
local é somente leitura; veja o [fluxo consultivo](../operacao/operacao-e-referencia.md#revisão-consultiva-do-conhecimento).

## Desktop

- [main.rs](../../apps/desktop-gpui/src/main.rs): composition root, serviços e atalhos.
- [app.rs](../../apps/desktop-gpui/src/app.rs): `Destination`, navegação do shell.
- [settings/mod.rs](../../apps/desktop-gpui/src/screens/settings/mod.rs):
  `SettingsSection::{Ai, OpenCode, Diagnostics, Appearance, Language}`;
  configuração de IA está aqui, não em um suposto `settings/ai.rs`.
- [i18n/mod.rs](../../apps/desktop-gpui/src/i18n/mod.rs): `Language`,
  `strings!`/`formats!`; o texto de cada tela fica em `i18n/<área>.rs` e a
  preferência em [ui/appearance.rs](../../apps/desktop-gpui/src/ui/appearance.rs)
  ([regras](../design/idiomas.md)).
- [ui/tokens.rs](../../apps/desktop-gpui/src/ui/tokens.rs),
  [ui/controls.rs](../../apps/desktop-gpui/src/ui/controls.rs),
  [ui/patterns.rs](../../apps/desktop-gpui/src/ui/patterns.rs): tokens e reutilização.
  Antes de alterar tela, leia a [regra visual](../design/VISUAL-IDENTITY.md).

## Captura: fonte e integração instalada

[Adapter index.ts](../../adapters/opencode/src/index.ts) →
[client.ts](../../adapters/opencode/src/client.ts) →
[local-api/server.rs](../../crates/local-api/src/server.rs) (`POST /v1/captures`,
`create_capture`) → [application/captures.rs](../../crates/application/src/captures.rs)
→ [storage-sqlite/store.rs](../../crates/storage-sqlite/src/store.rs).
Com desktop fechado, [adapter/outbox.ts](../../adapters/opencode/src/outbox.ts)
grava o envelope; [application/outbox.rs](../../crates/application/src/outbox.rs)
o importa pela mesma ingestão (testes unitários nesse arquivo).

No Claude Code, os hooks do módulo [mcp-server/src/hook](../../apps/mcp-server/src/hook/mod.rs)
(`prompt.rs` injeção, `transcript.rs` leitura incremental do transcript, `envelope.rs`,
`stop.rs` ponto de retomada, `deliver.rs` API ou outbox) usam os mesmos `/v1/context` e
`/v1/captures`.

`~/.config/opencode/plugins/xemnas.ts` é o wrapper instalado que reexporta o build,
não a fonte versionada. Client nativo, ativação e limites de fallback têm uma
única referência: [integração OpenCode](../operacao/operacao-e-referencia.md#integração-com-o-opencode).

## Busca no Windows

Execute na raiz; `-g` filtra arquivos, sem wildcard no caminho do PowerShell:

```powershell
rg --files crates/application/src -g '*graph*'
rg 'project_map' crates/application/src -g '*.rs'
rg --files crates/application/src/graph
# Se rg não iniciar (inclusive bloqueio SAC), use Git:
git ls-files 'crates/application/src/*graph*'
git grep -n 'project_map' -- crates/application/src
```

O primeiro comando filtra nomes de arquivos, não nomes de diretórios: para o
módulo `graph/`, use o terceiro. Bloqueios de ferramentas: [SAC](../operacao/operacao-e-referencia.md#smart-app-control-sac).
