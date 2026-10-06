# ADR 0001 — Aceitar GPUI para a camada de UI

- **Status:** Vigente. O `rev` do GPUI em `apps/desktop-gpui/Cargo.toml` é o registrado aqui. Aceita no Gate 0 (2026-09-28); revisável sob as condições abaixo.
- **Contexto:** MVP-SPEC §15, Gate 0 — "reduzir o maior risco técnico antes de acoplar o produto".
- **Decisores:** agente executor do plano `docs/roadmap/mvp/`, com evidências do spike.

## Contexto

A maior incerteza do produto é a camada de interface: desktop em Rust, no Windows, com
design system Quiet Glass, lista virtualizada de 10 mil itens, foco de teclado visível e
aceitação pelo Narrator. Escolher errado aqui acopla toda a aplicação a uma abstração que
precisaria ser trocada por inteiro depois.

O produto já havia fixado a hipótese em `docs/arquitetura/stack-e-arquitetura-rust-gpui.md`
("GPUI com versão/commit fixado"). O Gate 0 existe para confirmá-la com medições, não
para repeti-la.

## Decisão

Aceitar **GPUI** como camada de UI do app desktop, fixado em

```toml
gpui = { version = "0.2.2", git = "https://github.com/zed-industries/zed", rev = "244023605536a412ab6b8d5b658466b89fb15401" }
gpui_platform = { version = "0.1.0", git = "...", rev = "<mesmo rev>" }
```

O rev é o mesmo para os dois crates e está registrado no `Cargo.lock`. A razão de não usar
o publicado do crates.io é o suporte a **AccessKit** na janela (`crates/gpui/src/window.rs`,
PR zed#56065), que é o que fornece a árvore de acessibilidade que o Narrator consome.

O que esta decisão **não** decide:

- não define o layout de módulos (Gate 1 — `docs/roadmap/mvp/issues/04-fundacao-modular.md`);
- não aprova o spike como arquitetura — ele é descartável e será reescrito como crates;
- não escolhe banco, contrato, segurança, jobs ou distribuição;
- não promete macOS ou Linux.

## Evidências

### Testes em `cargo test --workspace --locked` (perfil dev com `opt-level = 1`)

| Medição | Valor | Orçamento |
|---|---|---|
| Criação da janela | 3,4 ms | — |
| Primeiro draw | 3,6 ms | — |
| Draw em regime | **2,1 ms** | < 16 ms |
| Lista de 10 mil: linhas pintadas | `0..13` | só as visíveis (virtualizada) |
| Após 400 eventos de roda | `1142..1156` | a janela pintada anda |
| Frame de scroll | **3,5 ms** | < 16 ms |
| Worker SQLite em background | `ui_avg = 3,6 ms` | < 16 ms |
| Progresso observado pela UI | `worker_rounds=34`, `capture_count=33` | sem congelamento |

4 de 4 testes verdes: `render_inbox_10k`, `keyboard_reaches_every_control`,
`accessibility_is_enabled`, `background_work_does_not_freeze_ui`.

### Aplicação release em Windows (`tools/gate0-evidence.ps1`)

```
GATE0 startup_to_discovery=1023 ms
GATE0 startup_to_window=1069 ms
GATE0 working_set=60060 KB
GATE0 api_post_without_token=401
GATE0 api_post_with_token=200
GATE0 api_health=200
GATE0 ui_nodes=28
GATE0 owns_foreground=True focused=Projects
GATE0 RESULT=PASS
```

- **Startup:** janela utilizável em ~1,0–1,4 s (1ª execução logo após rebuild, fria de
  disco, ~4,7 s; execuções seguintes ~1,0–1,4 s).
- **Memória:** 57–60 MB de working set.
- **Narrator/UIA:** a árvore expõe `Group` da inbox com o nome vivo
  `Decision Inbox, N receipts`, `ListItem`s, `Button`s `Confirm decision` /
  `Adjust decision`, `Navigation` com `Projects/Inbox/Decisions/Settings` e mais de três
  elementos focáveis.
- **Teclado + focus-visible:** um Tab real moveu o foco para `Projects`; a captura
  `window-focus-visible.png` mostra o anel de foco pintado. Capturas em
  `%TEMP%\xemnas-spike\evidence\`.
- **Fallback sem blur:** o spike não contém nenhuma chamada de blur/backdrop — todas as
  superfícies são sólidas, como exige `docs/design/design-system-quiet-glass.md` §"Glass".
- **API local:** mesmo servidor loopback autenticado — `401` sem token, `200` com token,
  `200` no health.

### Pacote Windows

`xemnas-spike-win64.zip` (5,3 MB, só o `.exe`) gerado em
`xemnas-spike\target\dist\`. Extraído num diretório limpo e revalidado com o mesmo
script: `GATE0 RESULT=PASS`. O pacote executa no Windows.

### Supply chain e qualidade

`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace --locked`, `cargo audit`, `cargo deny check` — **todos EXIT=0**.
`cargo-deny` roda com `allow-git` restrito às 5 fontes registradas no `Cargo.lock`.

## Condições de reconsideração

Trocar somente a camada de UI (mantendo domínio, contratos e storage) se qualquer uma
destas for verdade:

1. **Acessibilidade fica só na mão do AccessKit/zed.** O suporte a AccessKit depende de um
   rev do monorepo do Zed sem release no crates.io; se esse rev sumir, ou a API de
   acessibilidade mudar de forma incompatível, ou o Narrator parar de enxergar a árvore em
   uma atualização, reavaliar — ou congelar o rev e isolar a UI atrás de uma primitiva
   própria antes de seguir.
2. **A evidência de Narrator não puder ser mantida barata.** Hoje ela é um script externo
   de UIA, porque `TestWindow` do gpui não implementa `a11y_init` e o teste em processo
   devolve `debug_a11y_tree_json() == None`. Se o pipeline não conseguir reproduzir essa
   verificação, a aceitação de acessibilidade perde o lastro e a decisão precisa ser
   reaberta.
3. **O orçamento de 16 ms não se sustentar no perfil que será distribuído.** Medido com
   `opt-level = 1` o draw custa ~2 ms; no mesmo código com `opt-level = 0` custa ~20 ms e
   estoura o orçamento. Se a build distribuída mudar de perfil, remedi­r antes de aceitar.
4. **A UI precisar de mais capacidades que o gpui não entrega** (text editing completo,
   IME/CJK, drag & drop complexo, rendering de diffs gigantes) e cada uma virar um fork
   local do gpui.
5. **Cross-platform virar requisito.** Toda a validação é Windows; acessibilidade e
   empacotamento em outros sistemas seriam trabalho novo.
6. **Footguns de layout se multiplicarem.** Já foi observado que um `uniform_list` dentro
   de um `flex_col` pinta zero linhas sem `.flex_1().min_h_0()` — falha silenciosa,
   descoberta por screenshot. Se isso se repetir, erguer uma camada de primitivas que
   envolve o gpui em vez de usá-lo direto nas telas.

## Riscos aceitos

- A entrada de foco externa é frágil: enquanto outra aplicação mantém o foreground, o
  teste de teclado externo perde a corrida. A porta-guia disso é o teste em processo
  (`keyboard_reaches_every_control`); o externo complementa.
- Perfis: `opt-level = 1` em `[profile.dev]` é uma decisão deliberada do workspace, sem
  `debug-assertions` desligado.
- O spike é uma árvore única de ~1150 linhas. Aceitamos isso porque ele é descartável.

## Revisão

Reavaliar no fechamento do Gate 1 (fundação modular) e sempre que uma das condições acima
disparar. Se a decisão mudar, substituir este arquivo por uma nova ADR que cite este.
