# 04 — Gate 1: fundação modular e validação contínua

**What to build:** Estabelecer workspace Rust, dependências direcionais, tracing sanitizado e pipeline de qualidade.

**Blocked by:** 03 — Gate 0: decisão formal de UI.

**Status:** done

- [x] Build e validações mínimas passam.
- [x] Violações arquiteturais e dependências inseguras são detectáveis.

## Evidências

Workspace de 8 membros na raiz (`edition = "2021"`, `license = "MIT"`, `resolver = "2"`, `[profile.dev] opt-level = 1` conforme ADR 0001) mais `LICENSE` MIT, `deny.toml`, `Cargo.lock` versionado e `.github/workflows/ci.yml` em `windows-latest` rodando os cinco comandos na ordem.

Membros: `apps/desktop-gpui` (bin), `crates/{domain,application,integration-contracts,storage-sqlite,local-api,telemetry}`, `tests/architecture`. Sem `adapters/opencode/`, sem `migrations/`, sem `CONTEXT.md` na raiz e sem dependência `gpui`/`gpui_platform` (entra no ticket 05).

Guarda ARCH-001 em `tests/architecture/tests/architecture.rs`, rodando dentro de `cargo test --workspace`, com 11 testes:

- `only_allowed_dependency_edges` — arestas por `path` conforme `docs/stack-e-arquitetura-rust-gpui.md` L113-145.
- `domain_has_no_infrastructure_or_ui_dependencies` / `application_has_no_infrastructure_or_ui_dependencies` — `gpui`, `gpui_platform`, `rusqlite`, `axum`, `tower-http`, `reqwest`, `ureq`, `openai`, `async-openai` banidos nos dois crates.
- `domain_and_application_have_no_http_dependencies`.
- `gpui_dependency_confined_to_desktop_app`.
- `gpui_types_never_appear_in_sources_outside_desktop_app` — varre `.rs` e proíbe `use gpui` / `gpui::` / `gpui_platform::` fora de `apps/desktop-gpui/**` (as consts `GPUI`/`GPUI_PLATFORM` evitam auto-incriminação).
- `domain_sources_do_not_reach_outside_the_process` — proíbe `std::process`, `process::Command`, `extern "C"` e `#[link` em `crates/domain/**`, citando arquivo:linha.
- Allow-list vazia (`DOMAIN_ALLOWED_CRATES = &[]`) para dependências diretas do `domain`: toda entrada nova exige mexer no guard de propósito. Decisão registrada no código: a árvore transitiva **não** é resolvida via `cargo metadata` — allow-list + travas de fonte cobrem os atalhos realistas e evitam `cargo` aninhado dentro do teste.
- Leitura de dependências cobre `dependencies`, `dev-dependencies` e `build-dependencies` no topo e sob cada `[target.'cfg(...)']`, preservando `package = "..."`, com cada declaração preservada por separado (um alias repetido em dois targets não pode esconder a declaração proibida).
- Toda dependência `path = ...` tem de resolver para um membro registrado; falha de resolução ou de canonicalização é violação, não silêncio.

Violação demonstrada em duas ocasiões e revertida: (1) `use gpui::App;` comentado em `crates/domain/src/lib.rs` → `gpui_types_never_appear_in_sources_outside_app` FAILED citando arquivo:linha; (2) `deps.insert` last-wins restaurado → `same_alias_in_two_targets_keeps_every_declaration` FAILED (`the forbidden package must survive an alias shared with another target`). Varredura final: 0 arquivos `.rs` com `gpui` fora do app, 0 ocorrências de `deps.insert(`.

`crates/telemetry` (PRIV-001, 13 testes): redação por **nome** de campo (deny-list documentada, `message` fora) e **por forma de valor** em todo valor renderizado, incluindo `message` — `Bearer …` (inclusive base64 padrão com `/`), JWT (`eyJ` + ≥2 pontos), `sk-`/`sk_`. Borda esquerda por `is_word_char` (alfa-numérico, `_` ou `-` bloqueiam), então `token=JWT`, `api_key=sk-…` e `authorization=Bearer …` são redigidos; piso de 12 caracteres em `sk-`/`sk_` evita engolir segmentos de URL; `&`, `=` e `?` encerram o run. Sem `unwrap()`/`expect()` em caminho de produção; `init()` não panica em re-init e devolve `Result` para `RUST_LOG` inválido.

Validação (rodada final, PowerShell 5.1):

```
cargo fmt --all -- --check                     EXIT=0
cargo clippy --workspace --all-targets -- -D warnings   EXIT=0
cargo test --workspace --locked                EXIT=0   33 testes, 0 falhas
cargo audit                                    EXIT=0   47 dependências, 0 vulnerabilidades
cargo deny check                               EXIT=0   linhas warning|error = 0
```

`deny.toml` estrito para o grafo atual (só `MIT`, `Apache-2.0`, `Unicode-3.0`); o ticket 05 reintroduz as entradas do GPUI — fonte git pinada, licenças transitórias e `ignore` de advisories — com justificativa.

Revisão independente em três rodadas (review por risco); veredito final `APROVADO`.
