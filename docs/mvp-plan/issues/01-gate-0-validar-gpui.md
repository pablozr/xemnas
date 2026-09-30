# 01 — Gate 0: validar interação e desempenho do GPUI

**What to build:** Um spike descartável com shell Quiet Glass, navigation rail, lista virtualizada de 10 mil itens e painel de detalhe.

**Blocked by:** None — can start immediately.

**Status:** done

- [x] Medições de startup, memória e scroll foram registradas.
- [x] Teclado, foco visível, fallback sem blur e Narrator foram demonstrados.

**Evidências (2026-09-28)**

Spike descartável em `../xemnas-spike` (diretório irmão da raiz do Project; não é a
arquitetura). Validação do Gate, executada na raiz do spike:
`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace --locked`, `cargo audit`, `cargo deny check` — todos `EXIT=0`.

| Medição | Valor | Orçamento |
|---|---|---|
| `startup_to_window` (release) | 1,0–1,4 s | — |
| `working_set` (release) | 57–60 MB | — |
| Primeiro draw / draw em regime | 3,6 ms / 2,1 ms | < 16 ms |
| Linhas pintadas na lista de 10 mil | `0..13` | só as visíveis |
| Janela pintada após 400 eventos de roda | `1142..1156` | a lista anda |
| Frame de scroll | 3,5 ms | < 16 ms |

- **Teclado:** teste em processo `keyboard_reaches_every_control` (Tab, arrows, Home/End,
  PageUp, Enter) e, externamente, um Tab real que moveu o foco para o item `Projects`.
- **Foco visível:** anel pintado na captura
  `%TEMP%\xemnas-spike\evidence\window-focus-visible.png`.
- **Fallback sem blur:** o spike não contém nenhuma chamada de blur/backdrop — superfícies
  sólidas, conforme `docs/design-system-quiet-glass.md`.
- **Narrator:** árvore UIA lida por `tools/gate0-evidence.ps1` — 28 nós, incluindo
  `ListItem`s, `Confirm decision`, `Adjust decision` e a `Navigation` por nome.

Ciclo de evidência: `powershell -NoProfile -STA -File tools\gate0-evidence.ps1 -ExePath <exe>`
→ `GATE0 RESULT=PASS`.
