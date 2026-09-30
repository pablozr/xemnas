# 17 — Gate 4: exportação manual e E2E do ciclo completo

**What to build:** Exportar decisão em Markdown/JSON após preview e comprovar o fluxo completo.

**Blocked by:** 16 — Gate 4: Engineering Decisions versionadas e pesquisáveis.

**Status: done (backend aprovado; tela de exportação com a sessão de front)**

- [x] E2E cobre cadastro, captura, candidato, Evidence, edição, confirmação, busca e exportação. *(`crates/storage-sqlite/tests/full_cycle.rs` — in-process; ver decisão abaixo)*
- [x] Não há commit ou mutação surpresa no Project. *(snapshot do project dir antes/depois idêntico, sem `.git` novo)*

## Decisões registradas

- **E2E do ciclo completo é integração in-process, não PS1.** A API local mantém só os 4 endpoints por design (§9); confirm/busca/export operam em processo — o ciclo real só se prova chamando os casos de uso na sequência (`full_cycle.rs`: `Projects::register` → ingestão de envelope → `run_extraction` com `FakeCandidateExtractor` → `Inbox::detail` (ordem das evidências) → `adjust` → `confirm(Some)` → `Decisions::search` → `Export::preview`+`write`). Os PS1s existentes seguem cobrindo binário/HTTP/outbox. A execução do fluxo **por teclado na UI real** é Gate 4 do front ("fluxo completo por teclado").
- **Export (`application/src/export.rs`)**: `preview(decision_id, format) -> ExportDocument` gera conteúdo **byte-idêntico** ao de `write`; destino **sempre fornecido pelo chamador** (file picker da UI) — o use case não conhece o location do projeto (regra "ação explícita", §7.6:343; teste de imutabilidade prova que nada cai no working tree).
- **Formatos fixos e determinísticos** (sem timestamps voláteis): Markdown = `# {question}` + seções Escolha/Justificativa/Premissas/Escopo/Consequências/Reconsiderar quando/Proveniência/Evidências/Histórico de revisões (arrays vazios ⇒ `- _(nenhuma)_`, `\n` final); JSON = `ExportedDecision` pretty + `\n`. Preview == arquivo (testado).
- **Instalação atômica segura** (2 rodadas de review): temporário com **nome UUID v7 + `create_new` O_EXCL** (nunca trunca pré-existente/symlink; retry ≤8); `overwrite=false` instala via **`hard_link`** — falha atomicamente com `destination_exists` se o destino existir (inclusive criado após o preview ou symlink), **sem fallback para rename inseguro** (fs sem hard link ⇒ `io`, documentado); `overwrite=true` usa `rename` (substituição atômica; sobre symlink destino, o link é substituído — não seguído). Recusa destino-diretório. Nenhum `*.tmp` órfão (asserções).
- `ExportError::code()`: `storage|not_found|invalid_format|destination_exists|destination_invalid|io`.
- Review: r1 REPROVADO (temp previsível/truncate + TOCTOU do rename) → r2 **APROVADO**.

## Evidências

- `cargo test -p application --locked`: **98 pass** (49 lib + 6 decisions + 10 export + 19 extração + 14 inbox).
- `cargo test -p storage-sqlite --locked`: **58 pass**, incl. `full_cycle_covers_register_capture_evidence_edit_confirm_search_export` (imutabilidade do project dir antes/depois, preview == arquivo, decisão v1 reconstruível).
- `cargo test -p ai-provider --locked`: 13 + 1 ignorado; `architecture`: 11 + 1 (`ui/icons.rs:57`, WIP do front).
- rustfmt 0; clippy 0; `cargo check -p desktop-gpui --all-targets` 0; `cargo deny check` 0; `cargo audit` 0; **`Cargo.lock` intocado** (zero deps novas; uuid/serde_json já existentes).
- **E2E `jobs-recovery.ps1` PASS + `capture-outbox.ps1` PASS** (regressão pós-17).
- Critérios 11/13 da §17 verificados pelo teste (confirmar não escreve no repositório; export com preview e destino explícito).

## Dívida registrada

- **Tela de exportação (preview, escolha de formato/destino, overwrite)** — sessão de front; contrato: `Export::new(store)`, `preview`, `write(&ExportDocument, &Path, overwrite)`, tipos `ExportFormat/ExportDocument/ExportResult/ExportError/ExportedDecision` reexportados do application.
- `application` reexporta tipos de Capture Envelope + `serde_json` na raiz (habilitador do full_cycle sem mexer no lock) — superfície pública mais larga que o ideal.
- `ExportError::InvalidFormat` reservado (inalcançável para a struct atual).
- `overwrite=true` remove o arquivo antigo antes do rename (semântica Windows de rename) — janela mínima de crash documentada; filesystem sem hard link não suporta `overwrite=false` (retorna `io`).
- `full_cycle` usa `DefaultHasher` no snapshot (determinístico dentro da run — suficiente para o antes/depois).
