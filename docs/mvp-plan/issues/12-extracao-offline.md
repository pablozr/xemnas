# 12 — Gate 3: extração offline determinística

**What to build:** Filtrar relevância e produzir Decision Candidates por extractor fake.

**Blocked by:** 11 — Gate 2: outbox, importação e diagnóstico.

**Status:** done

- [x] Fixtures distinguem escolhas duráveis de alterações triviais.
- [x] Captura para candidato é reproduzível e não cria Engineering Decision.

## Decisões registradas

- Migração `0005_create_decision_candidates.sql`: tabela `decision_candidates` (id, project_id FK CASCADE, capture_id FK CASCADE, status CHECK nos 5 estados do §5, question, choice, rationale, signals JSON, confidence REAL 0..1 CHECK, confidence_reason, evidence_refs JSON, diff_summary JSON `{files, artifacts}`, dedup_hash UNIQUE, created_at, updated_at) + índices por capture_id e status. Só `status='pending'` é escrito (demais estados são o CHECK antecipado do §5; tabela nasce aqui porque §11 a exige).
- Duas passagens (§7.4): `filter_relevant(&DecisionEvidence) -> Vec<RelevanceSignal>` puro e determinístico → só com sinais é que `FakeCandidateExtractor.extract` roda. `run_extraction` grava `ExtractionReport`.
- Veto antes de positivos (§6): sem `diff_hunk` a evidência é vetada (prosa pura); marcadores triviais explícitos (`formatting`, `whitespace`, `lint`, `rename`, `comment only`, …); diff test-only; diff comment-only. Precedência: **sinal estrutural de produção prevalece sobre o veto** — mas estrutural exige: artefato `kind == "diff_hunk"`, linha `+` única (`+++` descartado), arquivo não-test, linha não-vazia e **não-comentário** (`is_comment_prefix`), e DDL real (`create/alter/drop table`, `add column`, `create index`, `migration`, `endpoint` — `INSERT`/`UPDATE` não contam), ou token de segurança, ou dependência nova em manifest. Sufixo `.sql` nunca é forte sozinho; comentários de manifest (`+# optional = "1"`) não são dependência.
- `FakeCandidateExtractor` determinístico: templates fixos, `confidence` = função pura dos sinais (clamp ≤0.95), sem tempo/aleatoriedade/ordem instável. Reprodutível: duas execuções → propostas idênticas campo a campo (exceto id/created_at).
- Validação estruturada estrita antes de QUALQUER insert: lote inteiro validado (`validate_proposal` + `validate_diff_summary`); campos não-vazios, confidence finita em 0..=1, `proposal.signals` subconjunto não-vazio dos detectados, `evidence_refs` ⊂ artefatos da evidência, `diff_summary` no formato `{files:[≤500 strings], artifacts:n}`; falha ⇒ `Err` sanitizado e zero linhas.
- Canonicidade: sinais canonizados (ordenados por `as_str`, dedup) e a MESMA lista da proposta alimenta a coluna `signals` e o `dedup_hash = artifact_fingerprint("{capture_id}|{question}|{choice}|{signals ordenados}")` — `INSERT OR IGNORE` por constraint (não checagem em memória) garante idempotência.
- Evidência limitada e determinística: `load_evidence` (receipt → projeto → checkpoint → `capture_artifacts ORDER BY artifact_id LIMIT 200`, content ≤64 KiB, `Ok(None)` se faltar captura/projeto). Tabela real é `capture_artifacts` (0003), não `source_artifacts`.
- Handler: `jobs.register(ANALYZE_CAPTURE_KIND, …)` em `main.rs` logo após `Jobs::new` (antes do share/worker); erro do handler ⇒ job `failed` com mensagem fixa (nunca conteúdo de artifact); log só contagens (PRIV-001); receipt/artifacts nunca são afetados por falha de extração (§7.4) e o job idempotente é reexecutável.
- `application` segue sem `tracing`/storage/rede: trait `ExtractionStore` em application, impl em `storage-sqlite`, composição só em `main.rs` (ARCH-001).
- `engineering_decisions` não existe até o ticket 16 ⇒ teste de "não cria Engineering Decision" assert `status='pending'` + zero efeitos colaterais.
- Review: r1 REPROVADO (vetos do §6 ausentes; saída não validada/sinais não canônicos) → r2 REPROVADO (veto absoluto matava fortes reais; validação parcial de diff_summary/evidence_refs/signals) → r3 REPROVADO (`.sql` sozinho forte, comentários disparavam, não restrito a diff_hunk) → r4 REPROVADO (comentário de manifest contava como dependência) → r5 **APROVADO**.

## Evidências

- `cargo test -p application --locked`: **49 pass** (39 lib + 10 integração), 0 fail (após correções finais).
- Sweep por pacote (domain, integration-contracts, local-api, telemetry): **51 pass + 1 ignorado**; storage-sqlite: 7+9+5+11 verdes na rodada do backend pós-fix e 7/7 lib pós-fix meu (`projects`/paths ocasionais bloqueados por SAC — ambiente; código de storage intocado após aquela rodada verde).
- Total executável do workspace: **~137 pass + 1 ignorado** (antes do ticket: 118+1).
- clippy via `clippy-driver` (`cargo-clippy.exe` bloqueado por SAC): 0 diagnósticos; `rustfmt --check` em todos os arquivos tocados: 0; `cargo fmt` só com diffs em `apps/**` (front).
- Guard ARCH: 11/12 (falha única `icons.rs:61` da frente).
- `cargo audit`=0, `cargo deny check`=0, `Cargo.lock`/`Cargo.toml` intocados (zero dependências novas).
- E2E `jobs-recovery.ps1` = PASS (8 asserts, pós-fix); `capture-outbox.ps1` = PASS (31, após o wiring do handler).
- Fixtures: **12** em `crates/application/tests/fixtures/extraction/` — 3 duráveis (migração, dependência, migração+nota de formatação) e 9 triviais/incidentais (formatação, teste, termos incidentais, test-only com "token", 2 diretórios, comment-only `.sql`, whitespace `.sql`, prosa forte, comentário de manifest), todas com fingerprint sha256 válido do envelope §10.

## Dívida registrada

- `engineering_decisions` ausente: teste de "não cria decisão" é indireto (nasce na tabela no ticket 16).
- Veto/positivo dependem de pistas textuais do envelope (sem diff semântico) — limitação declarada no doc-comment; réguas conservadoras.
- `INSERT OR IGNORE` engole violação de CHECK por desenho; CHECK coberto por teste com `INSERT` cru.
- Perfil real de IA, retries/backoff e proveniência do assessment: tickets 13/14 (fora deste ticket).
- API/inbox de candidatos: ticket 15.
- Ambiente: SAC (`os error 4551`) bloqueia exes de teste intermitentemente/por path; `cargo-fmt.exe`/`cargo-clippy.exe` também bloqueados (workarounds: `rustfmt` direto e `clippy-driver` como wrapper); exes apagados por workaround perdem reputação e podem ficar bloqueados até novo conteúdo.
