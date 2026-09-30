# 07 — Gate 1: jobs persistidos e recuperáveis

**What to build:** Enfileirar, executar e diagnosticar trabalho assíncrono com estados explícitos.

**Blocked by:** 04 — Gate 1: fundação modular e validação contínua; 06 — Gate 1: cadastro vertical de Projects.

**Status:** done

- [x] Jobs idempotentes interrompidos retornam à fila.
- [x] Falha ou panic não derruba a UI nem deixa estado parcial.

## Decisões registradas

- **Sem Tokio nesta rodada.** O spec manda o runtime Tokio rodar HTTP, importação, extração e chamadas de modelo — todos no Gate 2. O worker é uma `std::thread` dedicada (`xemnas-jobs`), acordada por condvar com sondagem de 50 ms e parada por `WorkerHandle::{stop,join}`. Nenhuma crate nova entrou.
- **Fila única de escrita SQLite.** Uma única conexão `Arc<Mutex<Connection>>` compartilhada por projects e jobs, com `journal_mode=WAL`, `foreign_keys=ON` e `busy_timeout` aplicados em toda abertura (spec §9 e §11). Leituras usam a mesma conexão; as conexões curtas de leitura previstas no spec entram com a API local no Gate 2, e abstruí-las agora seria especulação. `Mutex::lock` nunca com `unwrap`: envenenamento é recuperado com `into_inner`.
- **`ProjectStore` virou `SqliteStore`** (`crates/storage-sqlite/src/store.rs`), um tipo que implementa `ProjectRepository` e `JobRepository` sobre a mesma conexão; `projects.rs` e `jobs.rs` guardam só as implementações.
- **Estados e transições (spec §11).** `queued|running|completed|failed|cancelled` com `CHECK` no SQL; `claim_next` atômico (CAS, `attempts+1`, FIFO, só entre kinds com handler registrado) e `transition` compare-and-set dos dois lados. Kind sem handler permanece `queued` — nunca desaparece nem falha em silêncio. `enqueue` rejeita kind não registrado.
- **Recuperação no boot, antes do worker.** `running` + idempotente → `queued`; `running` + não idempotente → `failed` com diagnóstico fixo em PT-BR (reprocessamento manual vem no ticket 11). **Se `recover()` falhar, o worker não sobe** e a janela abre em estado degradado: sem reconciliação não há consumo da fila.
- **Nenhum handler de produto registrado.** O primeiro consumidor real é o ticket 09 ("agendar análise"); registrar trabalho inventado aqui seria escopo indevido. A superfície de diagnóstico desta rodada é `Jobs::list()` (mais recentes primeiro), que a tela de Diagnostics do ticket 11 consome — nenhuma tela nova, NavigationRail continua adiada.
- **Nada de texto livre chega a `last_error`.** O erro do handler é o enum fechado `JobFailure` (hoje só `Failed`, com mensagem fixa); panic grava `PANIC_MESSAGE`; interrupção grava `INTERRUPTED_NON_IDEMPOTENT`. Variantes novas entram apenas com mensagem fixa documentada, no ticket que precisar.
- **Panic é sanitizado no hook.** `catch_unwind` isola o handler na thread do worker, e `install_panic_sanitizer()` (chamada em `main.rs` após `telemetry::init()`) suprime o payload do panic **só dentro da janela do handler** e delega ao hook anterior fora dela. `write_sanitized_panic_report` não tem parameter de payload algum — só thread, location e texto fixo.
- **CAS `Ok(false)` vira `InvalidTransition`**, não emite evento `Finished` e encerra o worker: sem estado terminal falso e sem execução duplicada; um job deixado `running` por erro de storage é reconciliado pela recuperação transacional do próximo boot.
- **Observabilidade sem dependência nova:** `Jobs::observe_with`/`JobEvent` permite à composition root logar `job_id`, `kind`, `state`, `attempts` via `tracing` mantendo `application` sem essa crate.
- **Clock RFC3339 centralizado** em `crates/application/src/clock.rs` (a duplicação entre `projects` e `jobs` foi extraída junto com os testes de data).

## Evidências

```
cargo fmt --all -- --check                                    EXIT=0  warn=0
cargo clippy --workspace --all-targets -- -D warnings         EXIT=0  warn=0
cargo test --workspace --locked                               EXIT=0  80 testes
cargo audit                                                   EXIT=0  4 unmaintained (rustsec advisory)
cargo deny check                                              EXIT=0  11 warning[duplicate] do GPUI (aceitos)
powershell -File tests\e2e\jobs-recovery.ps1                  EXIT=0  RESULT=PASS (8/8)
```

- **Arquivos:** novos `crates/application/src/{jobs.rs,clock.rs}`, `crates/storage-sqlite/src/{store.rs,jobs.rs,migrations/0002_create_jobs.sql}`, `crates/storage-sqlite/tests/jobs.rs`, `tests/e2e/jobs-recovery.ps1`; alterados `crates/application/src/{lib.rs,projects.rs}`, `crates/storage-sqlite/src/{lib.rs,projects.rs}`, `crates/storage-sqlite/tests/projects.rs`, `apps/desktop-gpui/src/main.rs`. **Nenhum `Cargo.toml`/`Cargo.lock`, nenhuma tela (`screens/**`, `ui/**`).**
- **E2E no produto real** (`tests/e2e/jobs-recovery.ps1`, no repositório): semeia `XEMNAS_DATA_DIR` novo com 3 jobs (running+idempotente, running+não idempotente, queued) e 1 project, sobe o `xemnas.exe`, confere janela viva durante a recuperação, mata o processo e consulta o banco: idempotente → `queued`, não idempotente → `failed` com diagnóstico, `queued` intacto, `projects` intocado (8/8 asserts).
- **Testes de falha (TEST-001):** panic de handler isolado (flag de sanitização ligada só durante o handler e restaurada depois), relatório sanitizado sem payload, marcador secreto ausente de `last_error` e dos eventos para `Err` e panic (unidade e SQLite real), CAS perdido em desfecho terminal e em requeue sem evento `Finished`, recuperação transacional idempotente, cancel só de `queued`, claim atômico, migração em banco novo e reaberto, worker processa e encerra sem vazar thread.
- **Review:** duas rodadas `REPROVADO` (panic vazando pelo hook padrão; worker subia após falha de `recover`; `last_error` com string livre do handler) → corrigidas → **`APROVADO`**, com veredito de que nenhum canal de texto livre sobrou para persistência, log ou evento, e de que os dois critérios estão comprovados ponta a ponta.
