# 16 — Gate 4: Engineering Decisions versionadas e pesquisáveis

**What to build:** Converter confirmação humana em decisão local, preservar revisões e oferecer busca FTS5.

**Blocked by:** 15 — Gate 4: revisão completa na Decision Inbox.

**Status: done (backend aprovado; tela Decisions com a sessão de front)**

- [x] Proveniência e versões anteriores ficam acessíveis sem hard delete.
- [x] Busca encontra campos relevantes.
*(Critérios de UI — lista/busca/detalhe/histórico por teclado — são da sessão de front; a fachada `Decisions` abaixo os habilita.)*

## Decisões registradas

- **Conversão acontece DENTRO de `confirm_one`, na mesma transação** (tx `IMMEDIATE`): CAS do status do candidato + `engineering_decisions` + `decision_revisions` v1 + `evidence_links` (dedup por `UNIQUE(decision_id, artifact_id)`, posição compactada) + `decisions_fts`. Falha em qualquer etapa ⇒ rollback total (testado por PK collision ⇒ candidato permanece `pending` e nada da decisão persiste). Idempotência por `UNIQUE(candidate_id)` + CAS (confirm duplo ⇒ `not_found`/`invalid_state`, 1 decisão).
- **Contrato mudou (ticket 15):** `Inbox::confirm` retorna `ConfirmOutcome { status, decision_id }` (navegação direta à decisão criada). Doc-comment do inbox.rs atualizado.
- **Migração 0008 (versão 8)** — sem 0007 (gap aceito; testes de upgrade agora dropam v4+ e reaplicam 4/5/6/8): `engineering_decisions` (UUID v7, `status CHECK('accepted','superseded')`, campos + 4 arrays JSON, `version`), `decision_revisions` (`UNIQUE(decision_id, version)`, snapshot completo no nascimento e em cada revisão), `evidence_links` (`UNIQUE(decision_id, artifact_id)` = dedup por constraint §11), `decisions_fts` FTS5 (**só `question|choice|rationale`** — §11:609; manutenção pela aplicação no mesmo tx: INSERT no promote, REPLACE no revise; sem triggers).
- **Read model do histórico:** `DecisionRevision` COMPLETO (version, created_at, question, choice, rationale, 4 arrays) embutido em `DecisionDetail.revisions` (newest first), sem `history()` separado — reconstrução integral da v1 garantida por teste (revisão de todos os campos + revisão parcial preserva intocados). *(Round 1 do review reprovou a versão só-com-resumo; r2 aprovou a completa.)*
- **`Decisions` use case:** `list` (keyset `(confirmed_at, decision_id)` DESC), `detail` (proveniência candidate/capture/project + evidências na ordem + revisões), `search` (sanitização: tokens quoted AND, operadores FTS removidos, vazio ⇒ `invalid_query`; snippet `[, ], …`; filtro `project_id` pós-match), `revise` (snapshot version-CAS + UPDATE live + REPLACE FTS numa tx; `DecisionEdits` Option; limites 500/1000/4000 e arrays ≤50×1000). **Nenhum método de delete em lugar nenhum** (append-only; `superseded` modelado mas nenhum ação do MVP o seta — §7.6).
- **FTS5 confirmado disponível** no rusqlite 0.40 `bundled` (`libsqlite3-sys` build.rs:129 `-DSQLITE_ENABLE_FTS5`, verificado na fonte antes do implemento).
- **Backfill: inexistente** — nenhuma decisão aceita pré-0008; dados novos convertem pelo `confirm`.
- Sem dependências novas; `main.rs` intocado.
- Review: r1 REPROVADO (histórico não reconstruível) → r2 **APROVADO**.

## Evidências

- `cargo test -p application --locked`: **88 pass** (49 lib + 6 decisions + 19 extração + 14 inbox).
- `cargo test -p storage-sqlite --locked`: **57 pass** (inclui promoção transacional/rollback, reconstrução de revisão, dedup de evidência, FTS reflete revise).
- `cargo test -p ai-provider --locked`: 13 pass + 1 ignorado; `architecture`: 11 pass + 1 fail (`ui/icons.rs:57`, WIP do front).
- rustfmt 0; clippy 0; `cargo check -p desktop-gpui --all-targets` 0; `cargo deny check` 0; `cargo audit` 0; **`Cargo.lock` intocado**.
- Migração 0008: banco vazio + upgrade 6→8 verdes; 0005/0006 sem diff indevido.
- **E2E `jobs-recovery.ps1` PASS + `capture-outbox.ps1` PASS** (re-executados no fix, exe novo rodou sem WDAC).

## Dívida registrada

- **Tela Decisions (lista/busca/detalhe/histórico/teclado)** — sessão de front; contrato: `Decisions::new(store)`, `list`, `detail`, `search`, `revise` + tipos reexportados (`DecisionFilter/DecisionPage/DecisionSummary/DecisionDetail/DecisionProvenance/EvidenceLinkView/DecisionRevision/DecisionSearchHit/SearchQuery/DecisionEdits/DecisionStatus/DecisionsError` + constantes).
- `engineering_decisions.capture_id` sem FK (proveniência via `candidate_id`).
- `evidence_links.position` compacta duplicatas (refs únicas consomem slot).
- Concorrência em `revise` ⇒ perde com `not_found` (sem sinal de conflito mais rico).
- `superseded` e qualquer ação de supersede/delete ficam para spec posterior.
- Campo `assumptions/reconsider_when/scope/consequences` editável por `revise`, mas a UI de edição completa desses campos é polish do front.

## Pendências resolvidas depois do fechamento

- Concorrência em `revise`: `DecisionsError::Conflict` (código `conflict`) quando outra versão foi salva antes; `not_found` só quando a decisão sumiu — `f7be0e0`.
