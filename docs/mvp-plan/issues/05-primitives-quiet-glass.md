# 05 — Gate 1: primitives Quiet Glass reutilizáveis

**What to build:** Oferecer tokens e controles essenciais sem estilos improvisados nas telas.

**Blocked by:** 04 — Gate 1: fundação modular e validação contínua.

**Status:** done

- [x] Estados normal, hover, foco, disabled, loading, vazio e erro são demonstrados.
- [x] A referência visual canônica foi comparada.

## Evidências

`apps/desktop-gpui` ganhou `src/lib.rs` e módulos `src/ui/{tokens,theme,glass,controls,feedback}.rs` + `src/fonts.rs`, além do bin `xemnas` (janela é do ticket 06) e do bin de evidência `src/bin/quiet_glass_gallery.rs`. `gpui` e `gpui_platform` entram pinados por git rev `244023605536a412ab6b8d5b658466b89fb15401` (ADR 0001), `resolver = "2"` mantido.

Tokens (`ui/tokens.rs`): canvas, superfícies, `glass.fill-low`/`fill-strong`/`fill-emphasis`, `accent.*`, `text.*`, `status.*`, movimento e tipografia. Todas as cores vivem em `tokens.rs` — view não tem cor solta (regra 3 do design system). Primitives: `GlassSurface` (Low/Selected/Emphasis + `.solid()`), `FocusRing`, `PrimaryButton`, `QuietButton`, `IconButton`, `SearchField`, `StatusDot`, `EmptyState`, `ErrorState`. Galeria cobre os sete estados: normal/hover/foco/disabled/loading em `quiet_glass_gallery.rs:24-30`, vazio e erro em `:218-231`.

Fontes: `InterVariable.ttf` (família `Inter Variable`) e `JetBrainsMono-VariableFont_wght.ttf` embutidos por `include_bytes!` + `add_fonts`, com OFL em `apps/desktop-gpui/assets/fonts/`. Funciona no Windows porque `crates/gpui_windows/src/direct_write.rs` usa `CreateInMemoryFontFileReference` — o default do trait em `gpui/src/platform.rs` é no-op. Se `add_fonts` falhar, o fallback é do GPUI/OS e não é garantido por este crate (`theme.rs:28-37`); a constante `FONT_FALLBACK` que prometia Segoe UI foi removida por não ter consumidor.

Contraste (WCAG AA, 4.5:1): teste Rust `tokens.rs::tests::rendered_text_surfaces_meet_wcag_aa` cobre 11 pares texto/superfície mais ícone no limiar 3:1, com composição alpha e luminância relativas; `text.disabled` fica de fora pela exceção WCAG 1.4.3. Medido nas screenshots após a correção: legenda e título do card `Glass Emphasis` = **6.56:1**, rótulo do `PrimaryButton` = **6.97:1** (antes 1.7:1 e 1.2:1). O teste pegou um par que falhava (`status.danger`/`surface-hover` = 4.43:1) e a correção foi de design, não de token: em `QuietButton` danger o vermelho tinge só o ícone e o rótulo fica em `text.secondary`.

Guarda de cores nova em `tests/architecture/tests/architecture.rs::no_color_literals_outside_tokens`: literais `#RGB`/`#RRGGBB`/`#RRGGBBAA` e chamadas `rgb(`/`rgba(`/`rgb8(`/`rgba8(`/`hsl(`/`hsla(`/`hsba(` fora de `**/tokens.rs` são violação; `Color::rgb(` e `Rgba {` só quando ≥3 argumentos numéricos; `//` só conta como comentário fora de string (paridade par de `"`). Demonstrada falhando com `rgb8(255, 0, 0)` injetado em `controls.rs` → `no_color_literals_outside_tokens ... FAILED` citando arquivo:linha; revertida.

Foco é real, não casca para o screenshot: binding de Tab global, três tab stops e avanço circular (`quiet_glass_gallery.rs:35-61,76-80,248-255,322-345`). Diferença idle × focus-visible na área de cliente = **9250 px** (a primeira evidência tinha 0 px e foi reprovada por isso). Limitação registrada: o offset de 2 px do `FocusRing` não é representável neste rev do GPUI; o ring é desenhado na borda.

Comparação com a referência (`docs/design-system-reference.png`, 1487×1058): o arquivo estava corrompido no repositório (609.216 U+FFFD) e foi restaurado byte a byte a partir de `download (7).png`, sha256 `E6AD9DE4335A3C62969B5C6FB0D481C1677845E23AEFADE09194FDD0008957C9`; o corrompido ficou em `%TEMP%\xemnas\evidence\design-system-reference.CORRUPTED.bak.png`. A medição sobre a referência mostrou `accent.on-emphasis` a **6.6:1** sobre o preenchimento e a `text.muted` a **1.2:1** — o que gerou o token `glass.fill-emphasis = rgba(166, 158, 187, 0.95)` e a reclassificação de `glass.fill-strong` como camada de highlight translúcida em `docs/design-system-quiet-glass.md`. Janela em 1440×1024 (full 1456×1063) contra 1487×1058 da referência: delta de resolução registrado, nada redimensionado.

`deny.toml` reintroduz o que o GPUI exige (fonte git pinada, licenças transitórias, 3 ignores de advisory com reason, `allow-git` só `zed`) e mantém `multiple-versions = "warn"`, reportando 11 duplicatas transitórias em vez de silenciá-las. `tools/capture-quiet-glass.ps1` (novo, UTF-8 com BOM) imprime só metadados e nomes esperados, sem despejar a árvore UIA; `tools/gate0-evidence.ps1` intocado.

Validação (rodada final, PowerShell 5.1, verificada por mim):

```
cargo fmt --all -- --check                     EXIT=0
cargo clippy --workspace --all-targets -- -D warnings   EXIT=0
cargo test --workspace --locked                EXIT=0   35 testes, 0 falhas
cargo audit                                    EXIT=0   4 warnings unmaintained permitidos
cargo deny check                               EXIT=0   11 warning[duplicate], 0 error
```

Revisão independente (review por risco); primeira rodada `REPROVADO` por texto ilegível sobre `Glass Emphasis` — bloqueante corrigido —; veredito final `APROVADO`.

Pendências para os tickets 06/15/16: biblioteca definitiva de ícones (hoje placeholders geométricos, sem check no botão primário), offset real do `FocusRing`, `StatusPill` claro, componentes de tela (rail, inbox, disclosure, diff) e o delta de resolução contra a referência.
