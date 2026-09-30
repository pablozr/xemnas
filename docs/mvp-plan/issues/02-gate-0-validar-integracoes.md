# 02 — Gate 0: validar integrações técnicas

**What to build:** No spike, provar leitura SQLite fora da UI, job com progresso, API Axum autenticada e build Windows.

**Blocked by:** 01 — Gate 0: validar interação e desempenho do GPUI.

**Status:** done

- [x] Trabalho em background não congela a janela.
- [x] Requisição sem token falha e o pacote executa no Windows.

**Evidências (2026-09-28)**

- **SQLite fora da UI thread:** a contagem de capturas roda em `spawn_blocking` sob Tokio,
  com um sampler periódico (`spawn_sampler`) lendo por `Arc<AtomicUsize>`; a UI só faz
  load/swap de atômicos e `cx.notify()`.
- **Job com progresso:** a barra de status mostra `job NN%` e o contador de receipts
  atualiza ao vivo (UIA relê `Decision Inbox, N receipts`).
- **Sem congelamento:** teste `background_work_does_not_freeze_ui` — worker real inserindo
  no SQLite a cada 2 ms enquanto 120 frames são desenhados: `worker_rounds=34`,
  `capture_count=33`, `ui_avg=3,6 ms` (< 16 ms).
- **API autenticada:** `POST /v1/captures` → `401` sem token, `200` com bearer;
  `GET /v1/health` → `200`, só em loopback.
- **Pacote Windows:** `xemnas-spike\target\dist\xemnas-spike-win64.zip` (5,3 MB, só o
  `.exe`). Extraído num diretório limpo e revalidado com `tools\gate0-evidence.ps1` →
  `GATE0 RESULT=PASS`: o pacote executa no Windows.
