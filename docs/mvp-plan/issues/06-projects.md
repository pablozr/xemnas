# 06 — Gate 1: cadastro vertical de Projects

**What to build:** Registrar, listar e remover acompanhamento de diretórios sem alterar seu conteúdo.

**Blocked by:** 04 — Gate 1: fundação modular e validação contínua; 05 — Gate 1: primitives Quiet Glass reutilizáveis.

**Status:** done

- [x] Localização canônica e identidade sobrevivem ao reinício.
- [x] Remover acompanhamento não apaga o Project em disco.

## Decisões registradas

- **Sem NavigationRail nesta tela.** O shell tem um único destino real e o componente `Tooltip` do ticket 09 ainda não existe; adicionar uma barra lateral de um item seria UI decorativa. A navegação entra quando houver o segundo destino.
- **Aresta `desktop-gpui -> storage-sqlite` permitida.** Adicionada à allow-list do guard `only_allowed_dependency_edges` em `tests/architecture/tests/architecture.rs`: a raiz de composição da aplicação é o único ponto que escolhe a implementação de persistência; nenhum outro crate pode referenciá-la.
- **I/O sempre no background executor (ASYNC-001).** `spawn_io` move a handle `Projects<R>` para `cx.background_executor().spawn(...)`, devolve-a junto com `IoOutcome` e `finish()` aplica o resultado na thread da UI. `rusqlite::Connection` é `Send`, não `Sync`, então a handle é movida (nunca compartilhada) e a `take`/retorno serializa as operações — sem chamada de storage na thread da UI, sem spinner preso.
- **O caminho completo fica na row.** Título = `ProjectSummary::display_name()` (basename), segunda linha = `ProjectLocation::to_string()` (caminho canônico completo), com valor completo no `aria_label` antes da truncagem visual.
- **Nenhum diagnóstico técnico na UI.** `ProjectError` vira mensagem de produto em PT-BR (`inline_message`/`storage_failure`); o detalhe SQLite vai para `tracing::error!` (PRIV-001), com teste `storage_failure_never_renders_the_technical_detail`.
- **Correção visual de 05 dentro desta rodada.** O mini highlight por cima dos cards era retangular de largura total e sobressaía na curva: o GPUI recorta filho só por retângulo (`overflow_mask` em `crates/gpui/src/style.rs:638` não conhece raio), então `overflow_hidden` não resolve. Em `apps/desktop-gpui/src/ui/glass.rs` o top highlight, a selection edge (2 px) e o bottom edge agora são insetados pelo `radius`.

## Evidências

```
cargo fmt --all -- --check                                    EXIT=0  warn=0
cargo clippy --workspace --all-targets -- -D warnings         EXIT=0  warn=0
cargo test --workspace --locked                               EXIT=0  58 testes
cargo audit                                                   EXIT=0  4 unmaintained (rustsec advisory)
cargo deny check                                              EXIT=0  11 warning[duplicate] do GPUI (aceitos)
```

- **Fase A (backend):** `crates/domain/src/projects.rs`, `crates/application/src/projects.rs` (porta `ProjectRepository`, `ProjectError`, uuid v7), `crates/storage-sqlite/src/projects.rs` + migração `0001_create_projects.sql` + `tests/projects.rs`. Tabela `projects(id, location UNIQUE, registered_at)`; registro em `location` já canônica; `remove` só apaga a linha.
- **Fase B (UI):** `apps/desktop-gpui/src/{main.rs,lib.rs,app.rs,screens/mod.rs,screens/projects/mod.rs}` — primeira janela real `xemnas` (1440×1024) com a tela Projects (registrar/listar/remover com confirmação inline, estados vazio/carregando/erro, foco visível, Enter registra).
- **E2E no build final:** `%TEMP%\opencode\xemnas-e2e.ps1 -ExePath target\debug\xemnas.exe` → `RESULT=PASS`, `E2E_EXIT=0` (root `%TEMP%\xemnas-e2e-20260928-191126`): registrar pela UI → fechar → reabrir → a linha sobrevive (banco criado sob `XEMNAS_DATA_DIR`, localização canônica persistida) → remover → diretório e `keep.txt` intactos no disco. Critérios (a) e (b) demonstrados ponta a ponta.
- **Verificação visual do highlight, antes/depois, por script** (`tools/check-glass-corners.ps1`, sem computer use — captura com DPI-aware, detecta topos de card por pixel, mede o inset contra `radius`, gera PNG anotado, EXIT 0/1/2):
  - **Antes** (`quiet-glass-gallery.exe`, build de 15:08, pré-correção): `RESULT=FAIL`, `SCRIPT_EXIT=1`, inset 0–1 px em cards reais de vidro — linha de largura total sobressaindo na curva.
  - **Depois** (`xemnas.exe`, 18:55): `RESULT=PASS`, `SCRIPT_EXIT=0`, **inset 11 = 1 px de borda + 10 px de `radius.surface` em 8/8 cards**.
  - Sonda de pixels própria na imagem "depois", topo do card `y0=263`: `x48..50 = 15,9` (canvas), `x52..58 = 27/39/26/22` (borda na curva), `x60+ = 64,5` (highlight iniciando após a curva).
  - PNGs: `%TEMP%\xemnas\evidence\corners-before\` e `%TEMP%\xemnas\evidence\corners-after\`.
- **Review final:** `REPROVADO` na rodada anterior (I/O síncrono na UI; row sem caminho; diagnóstico em inglês) → corrigido → **`APROVADO`**, com o veredito de que os 3 pontos estão corrigidos, a correção do `glass.rs` é geometricamente coerente e os critérios seguem comprovados.

## Dívida registrada

- `tools/check-glass-corners.ps1`: `DataDir` padrão aponta para temporário desta rodada; aprova se **qualquer** quantidade positiva de cards passar (não valida contagem esperada); compara `radius` lógico com pixels físicos em DPI ≠ 100%; mede só a borda superior (não a selection nem a inferior). Não invalida a correção desta rodada — tratar quando a ferramenta de verificação visual for endurecida.
