# 15 — Gate 4: Decision Inbox

**What to build:** Listar/detalhar candidatos e Evidence, com ajuste, confirmação, rejeição e adiamento.

**Blocked by:** 14 — Gate 3: resiliência e proveniência da análise.

**Status: done (backend aprovado; tela/teclado com a sessão de front)**

- [x] Campos substantivos são editáveis e lote não confirma sem revisão. *(garantia estrutural no backend; edição na UI = front)*
- [ ] Fluxo principal e estados funcionam por teclado. *(critério de UI — sessão de front; contrato do backend exposto abaixo)*

## Decisões registradas

- **Escopo dividido**: este ticket entrega a parte backend — use cases, máquina de estados e read model da inbox. A tela (lista virtualizada, painel de detalhe, disclosure de diff/fontes, edição inline, teclado, estados vazio/loading/erro) é da sessão de front, que consome a fachada pública abaixo (nenhum HTTP novo — §9 mantém os 4 endpoints; UI consome em processo).
- **Máquina de estados (§5)**: `pending|snoozed` aceitam ações; `accepted|edited_and_accepted|dismissed` são terminais. Transições: `confirm(None)` → `accepted`; `confirm(Some edits)` → `edited_and_accepted`; `adjust` salva edições sem mudar status; `reject`/`dismiss_batch` → `dismissed`; `snooze`/`snooze_batch`: pending→snoozed; `unsnooze`: snoozed→pending. Tudo por CAS (`UPDATE … WHERE id AND status IN (…)` + linhas afetadas ⇒ `not_found`/`invalid_state`).
- **"Lote nunca confirma" é estrutural, não convenção** (3 rodadas de review): a porta `InboxStore` NÃO tem parâmetro de destino livre — só operações semânticas com destino fixo na implementação (`confirm_one` individual, `dismiss_batch`→dismissed, `snooze_batch`→snoozed); `ValidatedEdits` tem campos privados e único construtor `CandidateEdits::validate` (vazio/espaço/limites rejeitados fora do módulo). Testes percorrem todos os métodos públicos e provam que nenhum lote chega a `accepted`/`edited_and_accepted`.
- **Edits validados**: question ≤500, choice ≤1000, rationale ≤4000 chars (trim, não-vazios); caps em constantes públicas (`MAX_BATCH_IDS=100`, page 50/100).
- **Read model**: `CandidateSummary` (projeto, sessão/adapter via LEFT JOIN em `adapter_checkpoints` com fallback para receipt — mesma regra de `load_evidence`; data = `received_at`) e `CandidateDetail` (+ rationale, evidence_refs, diff_summary, artefatos na ordem das refs, só referenciados). Paginação keyset `(created_at, id)` DESC com cursor opaco, sem sobreposição; filtro default `[pending, snoozed]`.
- **Sem migração nova**: 0005 intocada; sem 0007 (query coberta por `idx_decision_candidates_status` em escala local — decisão revisada pelo review e mantida). Versão do schema segue 6.
- **`confirm` NÃO cria `engineering_decisions`** *(nota do fechamento do ticket 16 — a partir dele, a conversão acontece dentro de `confirm_one` na mesma transação e `confirm` retorna `ConfirmOutcome { status, decision_id }`; ver `16-decisoes-pesquisaveis.md`)*.
- Review: r1 REPROVADO (porta genérica permitia batch-confirm) → r2 REPROVADO (`ValidatedEdits` construível diretamente) → r3 **APROVADO**.

## Evidências

- `cargo test -p application --locked`: **80 pass** (49 lib + 19 extração + 12 inbox), 0 fail.
- `cargo test -p storage-sqlite --locked`: **50 pass**, 0 fail (inclui CAS concorrente, ordem de refs, destinos fixos da porta).
- `cargo test -p ai-provider --locked`: 13 pass + 1 ignorado (regressão 13/14 intacta).
- `cargo test -p architecture --locked`: 11 pass + 1 fail (`ui/icons.rs:57`, WIP do front, pré-existente).
- rustfmt 0; clippy 0 (application/storage-sqlite); `cargo check -p desktop-gpui --all-targets` 0; `cargo deny check` 0; `cargo audit` 0; **`Cargo.lock` intocado** (zero deps novas).
- Migração 0005 sem diff (confirmado no review).
- E2E `jobs-recovery`/`capture-outbox`: **última execução verde na rodada do ticket 14**; reexecução nesta rodada **bloqueada por política WDAC do host** (binário novo recusado em `target\debug` e em cópia `%TEMP%`, ~45 tentativas; política de segurança nunca contornada). O delta do ticket 15 (inbox) não é exercido por esses scripts.

## Dívida registrada

- **Tela da Decision Inbox (lista/detalhe/diff/edição/teclado)** — sessão de front; contrato exposto: `Inbox::new(store)`, `list`, `detail`, `confirm(id, Option<CandidateEdits>)`, `adjust`, `reject`, `dismiss_batch`, `snooze`, `snooze_batch`, `unsnooze` + tipos reexportados em `application` (`InboxFilter/InboxPage/CandidateSummary/CandidateDetail/CandidateEdits/CandidateStatus/InboxError/DiffSummary/ArtifactView` e constantes).
- **E2E pendente por WDAC do host** — reexecutar os dois scripts assim que a política liberar o bin novo (farei no próximo fechamento de ticket).
- `observed_at`/sessão são `Option` quando nenhum checkpoint aponta para a captura (limitação do schema 0003 — sessão por captura não é persistida).
- Cursor é string opaca `"<created_at>|<id>"` (não versionada; não sai do processo).
- Sinais desconhecidos persistidos são tolerados (skip) em vez de falhar a linha.
- Filtro textual/detalhe de evidência sob disclosure completo = polish de UI com o front.
