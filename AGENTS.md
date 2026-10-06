# Pilares

- **Performance e baixo custo são o pilar do projeto** e o primeiro critério de toda decisão técnica ou de interface: entre duas soluções equivalentes, a mais barata. Custo é CPU em repouso, memória, chamadas e tokens de IA e dependências.
- Na prática: monte só o que está na tela (virtualize), carregue o resto por demanda e resuma antes de listar; exibir tudo de uma vez nunca é a opção. Trabalho pesado vai para fora da thread da interface, e nada de clonar coleções dentro de `render` (cada passo de scroll refaz a view dona da rolagem).
- Meça antes e depois (`XEMNAS_PERF=1`, `XEMNAS_DEMO_SCALE=N`) e diga o número, e o que não foi medido. Regras, padrões e orçamentos: `docs/arquitetura/desempenho-e-escala.md`.

# Fluxo de trabalho

- Faça um commit local assim que cada alteração estiver concluída e validada proporcionalmente.
- Use commits pequenos, separados por assunto; evite acumular alterações para um commit grande.
- Inclua todo código, testes e documentação pertinentes à alteração. Preserve mudanças anteriores do usuário.
- Toda documentação fica em `docs/`, organizada por assunto; o mapa e a precedência entre documentos estão em `docs/README.md`. Consulte-o antes de criar um arquivo novo e mantenha-o atualizado.
- Para localizar um fluxo, símbolo ou teste, consulte `docs/arquitetura/mapa-do-codigo.md`.
- Pesquisas, planos de tela e ideias futuras ficam em `docs/pesquisas/`, com linha de status e entrada no índice da pasta. Ao implementar, leve o que for duradouro para `docs/design/VISUAL-IDENTITY.md`, o design system, `docs/arquitetura/` ou um ADR e apague a pesquisa no mesmo commit, junto com imagens e protótipos que só ela usava; só fica a que uma regra vigente cita como fundamento.
- Versione `/docs/` junto com as alterações pertinentes. Trabalho planejado vive em `docs/roadmap/tickets/`; ao fechar um ticket, registre o que foi feito e o commit na tabela "Entregue" de `docs/roadmap/README.md` e apague o ticket no mesmo commit. Se o produto mudou, atualize `docs/produto/estado-atual.md` no mesmo commit.
- Registre limitações de validação quando o ambiente impedir algum check, na mensagem do commit e no resumo ao usuário.
- Push só para a branch de trabalho da sessão, quando o usuário pedir ou o ambiente exigir; nunca para `master`. Pull request e merge dependem de pedido explícito do usuário. O pedido de commits automáticos não autoriza push por si só.

# Padrão visual do app desktop (GPUI)

Antes de criar ou alterar qualquer tela em `apps/desktop-gpui`, leia `docs/design/VISUAL-IDENTITY.md`: é a regra versionada. Toda tela nova segue o mesmo padrão das existentes (Revisão, Decisões, Projetos); se algo não couber no padrão, pare e proponha a mudança ao usuário em vez de criar uma variante local.

- **Reaproveite antes de criar.** Botões: `ui::controls::{action_button, icon_action}`. Padrões de layout e estado: `ui::patterns` (`reading_page`, `form_field`, `action_footer`, `panel_title`, `section_label`, `count_chip`, `status_pill`, `kbd`, `mark_selected` + `hover_tint`, `toast`, `error_banner`, `skeleton_list`, `empty_panel`, `reading_title`, `word_wrapped`, `fade_in`). Evidências e código: `screens::evidence`. Ícones: `ui::icons::icon(IconName, tamanho, cor)`, um glifo por conceito. Tooltips: `ui::tooltip`. Datas: `screens::format`.
- **Nada de valores soltos.** Cores só em `ui/tokens.rs` (existindo nas duas paletas); espaçamento por `SpacingScale`, alturas por `ControlSize`, raios por `theme.radius`, tipografia por `TypeScale` com pesos 400/500/600. Família de fonte sempre por `Theme::font_*()`, nunca por nome literal. Texto da interface só por `crate::i18n`, com os dez idiomas (regras e glossário em `docs/design/idiomas.md`).
- **Mesma gramática.** Leitura numa coluna de 760 px com rótulos de seção; ações de uma superfície num `action_footer` fixo; seleção de lista com `mark_selected`; uma única ação primária por região; lavanda só para seleção, foco e ação primária.
- **Estados completos.** Toda superfície tem carregando (esqueleto), vazio (`empty_panel` com o que a faz encher), erro recuperável (`error_banner` com ação real) e confirmação (`toast`). Só dados e ações reais: nada inventado para preencher espaço.
- **Teclado e acessibilidade.** Foco visível via `focus_ring`, `aria_label` em todo controle, tooltip em controle só de ícone, atalhos registrados no `main.rs` com contexto que exclua `SearchField`, e a nova tela alcançável pela paleta (Ctrl K).
- **Validação visual.** Compilar e testar não bastam: capture o binário recém-gerado (`--demo`) nas duas paletas e na janela compacta. Pergunte antes de rodar capturas que tomam o foco ou clicam na tela.
- **Registre o padrão.** Se criar um padrão novo, coloque-o em `ui::patterns` e documente em `docs/design/VISUAL-IDENTITY.md` no mesmo commit.

# Sessões paralelas

- Backend e front trabalham em sessões e branches separadas.
- A sessão de backend não altera `apps/`, exceto a composition root (`apps/desktop-gpui/src/main.rs`) com o menor diff possível, avisando no resumo que o front precisa puxar a branch.
- A sessão de front não coloca regra de negócio em `apps/`: a UI chama casos de uso do `application`; se faltar um, peça ao backend em vez de implementar na tela.

# Validação

- Backend: `cargo fmt --all -- --check`, `cargo clippy --locked -p <crates alterados> --all-targets -- -D warnings` e `cargo test --locked -p <crates alterados> -p architecture`.
- Núcleo (captura, extração, grafo, observações, revisão, aprovação automática, seleção e entrega de contexto): toda mudança inclui ou atualiza um portão de assertividade e um de desempenho e roda `python tools/core-quality.py`; ao melhorar um número, suba o piso no mesmo commit. Detalhes em `docs/arquitetura/qualidade-do-nucleo.md`.
- Front: além do acima, `cargo check -p desktop-gpui` no Windows. Em container Linux o GPUI não compila; registre a limitação.
- `rustfmt` desiste em silêncio de expressões longas demais: `fmt --check` verde não garante código formatado; revise linhas acima de 100 colunas.
