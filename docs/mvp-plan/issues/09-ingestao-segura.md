# 09 — Gate 2: ingestão local segura e idempotente

**What to build:** Receber captura autenticada, validar Project, persistir receipt/artefatos e agendar análise.

**Blocked by:** 08 — Gate 2: contrato versionado de captura.

**Status:** done

- [x] API usa somente loopback, token, limite e timeout.
- [x] Repetições não duplicam dados e a resposta não aguarda IA.

## Decisões registradas

- **Aresta `application → integration-contracts`**: o caso de uso consome `CaptureEnvelope` (o contrato é a linguagem da ingesta); `storage-sqlite` não ganhou aresta para contracts — `CaptureRepository` opera com DTOs de linha próprios da application. Guard atualizado (`application→integration-contracts`, `local-api→storage-sqlite` dev para testes, `desktop-gpui→local-api` para o wiring).
- **Deduplicação** (§7.3.5 + §11): receipt com `idempotency_key TEXT UNIQUE` ⇒ replay devolve o receipt existente sem reescrever artifacts/job; artifacts com `UNIQUE(capture_id, kind, fingerprint)` + dedup do payload pelo par calculado. **Fingerprints não deduplicam entre captures** — a identidade do evento é a `idempotency_key` (§10); conteúdo idêntico em eventos distintos é legítimo.
- **Fingerprint conferido na ingesta**: SHA-256 UTF-8 recalculado de cada `content` (`integration-contracts::artifact_fingerprint`, `sha2` promovido a dep normal); divergência ⇒ 422 sem persistir; dedup usa o valor calculado.
- **Agendamento**: job `analyze_capture` (`idempotent: true`, payload = `capture_id`) gravado **na mesma transação** via `JobRepository::insert`; **sem handler até o ticket 12** — fica `queued` por design (`claim_next` mantém kinds não registrados; nunca completa falso).
- **API** (axum 0.8 + tokio, stack aprovado): os 4 endpoints do MVP (`/v1/health`, `/v1/capabilities`, `POST /v1/captures`, `GET /v1/captures/{id}`), **todos com bearer token** (decisão registrada), sem camada CORS, `Origin` presente ⇒ 403 (inclusive loopback), `Content-Type` JSON exigido ⇒ 400, `Idempotency-Key` obrigatório e igual ao envelope ⇒ 400 se ausente/divergente.
- **Limites**: body 4 MiB (⇒ 413) e timeout de requisição 5 s (⇒ 504), ambos configuráveis. O timeout é a **camada mais externa do router**, cobrindo a leitura do corpo (teste de socket com corpo parcial).
- **Validação**: JSON Schema com formats assertados (⇒ 422); erros de schema logam **só** `error_count`/`keyword` — nunca `instance()`/conteúdo do cliente (PRIV-001, testado com marcador secreto).
- **Status**: 400 malformado/chave; 401 token; 403 Origin **ou** projeto não registrado (mesmo código — sem oráculo de existência); 404 receipt; 409 `artifact_id` duplicado (rollback total); 413 corpo; 422 schema/versão/fingerprint; 504 timeout; 500 storage genérico (detalhe só em `tracing`). Corpo de erro `{code, message}` em PT-BR.
- **TOCTOU fechado**: o registro do projeto é reconfirmado **dentro da transação `IMMEDIATE`** que grava receipt+artifacts+job; remoção entre o check e o insert ⇒ 403 e rollback integral.
- **Discovery/token por sessão**: `{runtime}/discovery.json` (`protocol_version`, `port`, `instance_id`) + `{runtime}/api-token` (≥32 bytes, `chmod 600` no unix, ACL do perfil no Windows), removidos no shutdown gracioso; `{runtime}` = diretório pai de `app.db`, injetado pelo composition root; compare em tempo constante manual; token nunca em log.
- **Wiring** (orquestrador, único ponto em `apps/`): `main.rs` sobe o servidor com `ApiServer::start` numa thread própria com runtime Tokio próprio (após o worker de jobs; falha de start só loga e a janela abre) e chama `api.shutdown()` antes de parar o worker.
- **`find_by_location`** com implementação default no trait (varre `list()`), storage sobrescreve com query — evitou editar o fake da UI em `apps/` durante o desenvolvimento backend paralelo.

## Evidências

- Review round 1 `REPROVADO` (4 bloqueantes: timeout não cobrindo o corpo, log de schema vazando conteúdo, TOCTOU no projeto, fingerprint não conferido) → correções → round 2 **`APROVADO`** sem achados novos.
- `cargo fmt --all -- --check` EXIT **0**
- `cargo clippy --workspace --all-targets -- -D warnings` EXIT **0**
- `cargo test --workspace --locked` EXIT **0** → **121 pass + 1 ignored** (25 testes novos na rodada inicial + 8 da correção)
- `cargo audit` EXIT **0** (4 warnings allow-listados)
- `cargo deny check` EXIT **0** → só as 11 `warning[duplicate]` do GPUI (axum 0.8.9, tokio 1.53.1, hyper 1.11.1, tower 0.5.3, getrandom 0.4.3 — nenhuma duplicata nova)
- `tests\e2e\jobs-recovery.ps1` → **RESULT=PASS** (8/8) com a API ligada no binário.
- **Smoke do binário real** (`xemnas.exe` com `XEMNAS_DATA_DIR` em temp): `discovery.json` com porta/protocol/instance + `api-token` de 64 hex; health sem/com token **401/200**; capabilities **200**; POST sem token **401**, com `Origin` **403**, sem chave **400**, chave divergente **400**, chave válida + projeto não registrado **403**; `Get-NetTCPConnection` do processo = **somente 127.0.0.1**.
- Migration `0003_create_captures.sql` testada em banco vazio, banco anterior e re-execução; `idempotency_key UNIQUE` verificado por constraint.

## Dívida registrada

- Handler `analyze_capture` só existe a partir do ticket 12 — jobs ficam `queued` até lá (intencional, documentado).
- Timeout não cobre o parse de headers (hyper) — cobre extração + corpo; slowloris de cabeçalho fica para o hardening do Gate 5.
- Métricas de capturas aceitas/rejeitadas/deduplicação são derivadas de logs estruturados por enquanto; painel de diagnóstico é o ticket 11.
- Redaction/reasoning omission ficam no adapter (ticket 10); a ingesta persiste o que o contrato carrega.
- `GET /v1/capabilities` expõe `max_body_bytes`; políticas de tamanho por artefato (adapter) vêm no ticket 10.
