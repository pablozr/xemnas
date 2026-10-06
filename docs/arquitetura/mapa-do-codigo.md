# Mapa do código

Entradas atuais para localizar um fluxo sem percorrer toda a árvore. Consulte
somente a linha relevante e suas dependências diretas; a divisão-alvo está na
[referência arquitetural](stack-e-arquitetura-rust-gpui.md), não neste mapa.
Os caminhos abaixo são relativos a este documento; símbolos são pistas de busca,
não promessas de âncoras de linha estáveis.

## Backend e testes focados

| Fluxo | Entrada e símbolos | Testes |
| --- | --- | --- |
| Captura | [captures.rs](../../crates/application/src/captures.rs): `CaptureApi`, ingestão | [captures.rs](../../crates/storage-sqlite/tests/captures.rs), [full_cycle.rs](../../crates/storage-sqlite/tests/full_cycle.rs) |
| Grafo e mapa | [graph/mod.rs](../../crates/application/src/graph/mod.rs): `KnowledgeGraph`; [query.rs](../../crates/application/src/graph/query.rs): `project_map`; [derive.rs](../../crates/application/src/graph/derive.rs): `added_dependencies`; [discover.rs](../../crates/application/src/graph/discover.rs): `declared_components`, `assemble`, `merge_package_aliases` (apelidos derivados do pacote); [ai_link.rs](../../crates/application/src/graph/ai_link.rs): `ai_link_reason`, `ai_link_quote` (motivo de vínculo proposto pela IA) | [graph.rs](../../crates/storage-sqlite/tests/graph.rs); testes unitários em `derive.rs` |, [graph.rs](../../crates/storage-sqlite/tests/graph.rs); testes unitários em `derive.rs` e `ai_link.rs` |
| Vínculos propostos pela IA | [link_suggestions.rs](../../crates/application/src/link_suggestions.rs): job `suggest_links` (`LinkFinder`, `LINK_PROMPT`, `parse_links`, `needs_links`), enfileirado na adoção em [adoption.rs](../../crates/application/src/adoption.rs); registrado em `apps/desktop-gpui/src/main.rs` | [link_suggestions.rs](../../crates/storage-sqlite/tests/link_suggestions.rs), [link_corpus.rs](../../crates/storage-sqlite/tests/link_corpus.rs) (`link_quality_gate`, fixture gerada por [link_suggestions_live.rs](../../crates/ai-provider/tests/link_suggestions_live.rs)), `a_link_the_ai_proposed_is_judged_with_its_quote_and_reason` em [auto_approval.rs](../../crates/storage-sqlite/tests/auto_approval.rs) |
| Context Pack | [context.rs](../../crates/application/src/context.rs): `ContextProvider`, `ContextPacks`; [context_settings.rs](../../crates/application/src/context_settings.rs): `ContextMode`, `ContextSettings`; [terms.rs](../../crates/application/src/terms.rs): `TaskTerms` (plurais e ponte PT/EN); [search_terms.rs](../../crates/application/src/search_terms.rs): `SearchTermFinder` (termos de busca gerados na adoção, último recurso) | [context_pack.rs](../../crates/storage-sqlite/tests/context_pack.rs), [context_settings.rs](../../crates/storage-sqlite/tests/context_settings.rs), [context_corpus.rs](../../crates/storage-sqlite/tests/context_corpus.rs) (`context_quality_gate`, `sealed_v4_quality_gate`, `calibration_v5_quality_gate`; experimento de embeddings com vetores de [context-embeddings](../../tools/context-embeddings/src/main.rs)), [dogfood_context.rs](../../crates/storage-sqlite/tests/dogfood_context.rs) (dados reais, só local), [search_terms.rs](../../crates/storage-sqlite/tests/search_terms.rs) |
| Substituição de decisão | [relations.rs](../../crates/application/src/relations.rs): `DecisionRelations::supersede`; [decisions.rs](../../apps/desktop-gpui/src/screens/decisions.rs): `Action::Relate`, “Substitui” | [relations.rs](../../crates/storage-sqlite/tests/relations.rs) |
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
