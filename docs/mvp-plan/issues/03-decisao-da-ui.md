# 03 — Gate 0: decisão formal de UI

**What to build:** Avaliar evidências do spike e registrar a decisão de aceitar GPUI ou trocar somente a camada de UI.

**Blocked by:** 01 — Gate 0: validar interação e desempenho do GPUI; 02 — Gate 0: validar integrações técnicas.

**Status:** done

- [x] Rationale, evidências e condições de reconsideração estão registrados.
- [x] O spike não foi promovido automaticamente a arquitetura definitiva.

**Decisão (2026-09-28):** aceitar GPUI para a camada de UI, fixado no rev
`244023605536a412ab6b8d5b658466b89fb15401` (contém AccessKit / PR zed#56065).

Registrado em `docs/adr/0001-aceitar-gpui-para-a-camada-de-ui.md` (caminho relativo à raiz
do Project) com rationale, números das evidências e seis condições explícitas de
reconsideração.

O spike não virou arquitetura: ele continua isolado em `../xemnas-spike` (árvore única,
sem crates), é declarado descartável nessa ADR, e o Gate 1 (`04-fundacao-modular.md`)
começa do workspace verde descrito em `docs/stack-e-arquitetura-rust-gpui.md`, não do spike.
