# Fluxo de trabalho

- Faça um commit local assim que cada alteração estiver concluída e validada proporcionalmente.
- Use commits pequenos, separados por assunto; evite acumular alterações para um commit grande.
- Inclua todo código, testes e documentação pertinentes à alteração. Preserve mudanças anteriores do usuário.
- Versione `/docs/` junto com as alterações pertinentes, conforme solicitado pelo usuário.
- Registre limitações de validação quando o ambiente impedir algum check.
- Push e merge dependem da solicitação do usuário; o pedido de commits automáticos não os autoriza por si só.

# Padrão visual do app desktop (GPUI)

Antes de criar ou alterar qualquer tela em `apps/desktop-gpui`, leia `VISUAL-IDENTITY.md`: é a regra versionada. Toda tela nova segue o mesmo padrão das existentes (Revisão, Decisões, Projetos); se algo não couber no padrão, pare e proponha a mudança ao usuário em vez de criar uma variante local.

- **Reaproveite antes de criar.** Botões: `ui::controls::{action_button, icon_action}`. Padrões de layout e estado: `ui::patterns` (`reading_page`, `form_field`, `action_footer`, `panel_title`, `section_label`, `count_chip`, `status_pill`, `kbd`, `mark_selected` + `hover_tint`, `toast`, `error_banner`, `skeleton_list`, `empty_panel`, `reading_title`, `word_wrapped`, `fade_in`). Evidências e código: `screens::evidence`. Ícones: `ui::icons::icon(IconName, tamanho, cor)`, um glifo por conceito. Tooltips: `ui::tooltip`. Datas: `screens::format`.
- **Nada de valores soltos.** Cores só em `ui/tokens.rs` (existindo nas duas paletas); espaçamento por `SpacingScale`, alturas por `ControlSize`, raios por `theme.radius`, tipografia por `TypeScale` com pesos 400/500/600. Família de fonte sempre por `Theme::font_*()`, nunca por nome literal.
- **Mesma gramática.** Leitura numa coluna de 760 px com rótulos de seção; ações de uma superfície num `action_footer` fixo; seleção de lista com `mark_selected`; uma única ação primária por região; lavanda só para seleção, foco e ação primária.
- **Estados completos.** Toda superfície tem carregando (esqueleto), vazio (`empty_panel` com o que a faz encher), erro recuperável (`error_banner` com ação real) e confirmação (`toast`). Só dados e ações reais: nada inventado para preencher espaço.
- **Teclado e acessibilidade.** Foco visível via `focus_ring`, `aria_label` em todo controle, tooltip em controle só de ícone, atalhos registrados no `main.rs` com contexto que exclua `SearchField`, e a nova tela alcançável pela paleta (Ctrl K).
- **Validação visual.** Compilar e testar não bastam: capture o binário recém-gerado (`--demo`) nas duas paletas e na janela compacta. Pergunte antes de rodar capturas que tomam o foco ou clicam na tela.
- **Registre o padrão.** Se criar um padrão novo, coloque-o em `ui::patterns` e documente em `VISUAL-IDENTITY.md` no mesmo commit.
