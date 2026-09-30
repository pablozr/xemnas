# Fluxo de trabalho

- Faça um commit local assim que cada alteração estiver concluída e validada proporcionalmente.
- Use commits pequenos, separados por assunto; evite acumular alterações para um commit grande.
- Inclua todo código, testes e documentação pertinentes à alteração. Preserve mudanças anteriores do usuário.
- Versione `/docs/` junto com as alterações pertinentes. Ao fechar uma pendência registrada em `docs/mvp-plan/issues/`, anote no próprio ticket o que foi feito e o commit.
- Registre limitações de validação quando o ambiente impedir algum check, na mensagem do commit e no resumo ao usuário.
- Push só para a branch de trabalho da sessão, quando o usuário pedir ou o ambiente exigir; nunca para `master`. Pull request e merge dependem de pedido explícito do usuário.

# Sessões paralelas

- Backend e front trabalham em sessões e branches separadas.
- A sessão de backend não altera `apps/`, exceto a composition root (`apps/desktop-gpui/src/main.rs`) com o menor diff possível, avisando no resumo que o front precisa puxar a branch.
- A sessão de front não coloca regra de negócio em `apps/`: a UI chama casos de uso do `application`; se faltar um, peça ao backend em vez de implementar na tela.

# Validação

- Backend: `cargo fmt --all -- --check`, `cargo clippy --locked -p <crates alterados> --all-targets -- -D warnings` e `cargo test --locked -p <crates alterados> -p architecture`.
- Front: além do acima, `cargo check -p desktop-gpui` no Windows. Em container Linux o GPUI não compila; registre a limitação.
- `rustfmt` desiste em silêncio de expressões longas demais: `fmt --check` verde não garante código formatado; revise linhas acima de 100 colunas.
