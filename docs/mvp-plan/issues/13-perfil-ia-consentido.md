# 13 — Gate 3: AI Execution Profile real e consentido

**What to build:** Configurar um provider real atrás de CandidateExtractor, além do fake.

**Blocked by:** 12 — Gate 3: extração offline determinística.

**Status:** done

- [x] Chamadas externas começam desligadas e exigem preview/consentimento.
- [x] Segredos usam armazenamento seguro e saída estruturada é validada estritamente.

## Decisões registradas

- **Provider único real escolhido: `OpenAICompatibleProvider`** (§12 "apenas um provider real de menor esforço"): cobre Ollama/LM Studio/endpoints locais e OpenAI-compatíveis na nuvem via `POST /chat/completions`. Novo crate `crates/ai-provider` — ARCH-001 ban HTTP em `application`/`domain`; guard de arquitetura registrado de propósito (`ai-provider => [application]`, `desktop-gpui += ai-provider`), como o próprio guard exige.
- **Modelo de perfil vive fora do SQLite** (§11 lista só tabelas de domínio): `FileProfileStore` em `data_dir/settings/ai-profile.json` (`XEMNAS_DATA_DIR` respeitado), `deny_unknown_fields`, escrita atômica; caminho injetado pelo composition root. Boot com `load_or_seed` cria perfil offline default (kind Fake, externo desligado) — §7 linha 404 ("usar fake/offline sem configurar provedor") e a inbox segue funcionando sem IA configurada.
- **Consentimento vinculado à config aprovada**: `consent_status(profile)` é a ÚNICA fonte de verdade (predicate público em `application::profile`), exigida por `choose_extractor` e pelo guard de defesa em profundidade de `extract()`: kind externo + `external_calls_enabled` + consent presente + **`preview_hash == preview_hash(build_preview(profile))`**. Qualquer mutação pós-grant (endpoint, model, limites, kind) invalida o consentimento e exige nova prévia. Perfil Fake puro segue `OfflineFake`; intent externo com consent stale/forjado ⇒ `ExternalBlocked` (conservador: nunca rede, nunca fallback silencioso).
- **Preview determinístico sem conteúdo**: `build_preview` lista categorias de artefato (`user_text`, `assistant_text`, `diff_hunk`, `tool_summary`), limite por item (`max_input_chars`), nota de redaction-on-ingest (o conteúdo já vem redacionado e limitado da ingestão §10/§13) e host/modelo — sem enviar amostra de texto na prévia. `grant_consent` só grava com preview atual + secret presente no keystore.
- **Segredos**: porta `SecretStore` em application; `KeyringSecretStore` em `ai-provider` (keyring 3, Windows Credential Manager, service `xemnas.ai-profile`, account = profile.id). Segredo nunca no arquivo de perfil, em `Debug`, log ou mensagem de erro. Pino `keyring =3.6.2` por bloqueio do build script `windows_x86_64_msvc 0.53.1`/`windows-sys 0.60` pelo WDAC do host (3.6.2 usa `windows-sys 0.59` já construído no workspace).
- **Saída estrita em dois estágios**: envelope real `choices[0].message.content` (tolera campos legítimos OpenAI `id/created/usage/…`, exige choices não-vazio + content presente) → `content` parseado como `ModelEnvelope` com `deny_unknown_fields` (aqui campos extras do modelo falham) → propostas passam pela validação estrita de lote do ticket 12 (`confidence` 0..1, `evidence_refs` ⊂ artefatos, `diff_summary` com paths reais) ⇒ lote inteiro rejeitado e zero linhas. `signals` da proposta sempre são os sinais locais do filtro (o modelo não decide relevância).
- **Limites/erros**: input truncado por `max_input_chars` (teto global 64 KiB), resposta ≤512 KiB, timeout connect 5 s / total 90 s, `temperature 0`, `response_format: json_object`; erros sanitizados PT-BR (status/tipo, nunca corpo/URL/secret). Falha do provedor não toca receipt/artifacts (§12).
- **Wiring** (`main.rs`): handler `ANALYZE_CAPTURE_KIND` recarrega o perfil a cada job (mudança de Settings sem restart) e casa `choose_extractor`: `OfflineFake` → fake; `ExternalBlocked` → job completed com report vazio + log sanitizado; `ExternalEnabled` → secret via keystore (ausente ⇒ job failed com diagnóstico) + `OpenAiCompatibleExtractor` dentro de `run_extraction`. Guard extra em `extract()` impede requisição mesmo se a composição errar.
- **HTTP client**: `reqwest` 0.12 blocking + `default-tls` (native-tls→schannel no Windows, MIT) — evita MPL-2.0 do `webpki-roots` fora do allow-list do `cargo-deny`; handler roda na worker thread std do jobs (sem runtime tokio ativo).
- Review: r1 REPROVADO (consent não vinculado à config; envelope errado para APIs OpenAI-compatíveis reais) → r2 **APROVADO**.

## Evidências

- `cargo test -p application --locked`: **58 pass** (48 lib + 10 extração), 0 fail — inclui matriz `choose_extractor` e consent bindado (mutação de endpoint/model/limites/kind ⇒ `ExternalBlocked`).
- `cargo test -p ai-provider --locked`: **8 pass + 1 ignorado**; teste `#[ignore]` do keyring real executado à mão: **1 pass** (roundtrip na Credential Manager: set/get/delete).
- Testes de segurança do provider (mock HTTP loopback, `TcpListener`): consentimento desligado/stale/forjado ⇒ **0 requisições**; happy path com envelope real; saída hostil (`confidence 1.5`, `evidence_ref` inexistente, path inexistente, campo extra dentro do content) ⇒ lote rejeitado, 0 linhas; envelopes malformados (choices vazio, content ausente/nulo/não-JSON) ⇒ Err sanitizado; HTTP 429/500 ⇒ só status.
- `cargo test -p architecture --locked`: **11 pass + 1 fail** — falha única `no_color_literals_outside_tokens` em `apps/desktop-gpui/src/ui/icons.rs:57`, WIP da sessão de UI concorrente (mesma falha 11/12 desde o ticket 12, linha deslocou 61→57); registro do crate novo no guard aprovado.
- clippy (clippy-driver wrapper) 0 diagnósticos em `application`, `ai-provider` e `desktop-gpui` (avisos de `missing docs` restantes são dos arquivos novos do front: `app.rs:18`, `ui/search_field.rs:23`); `rustfmt --check` 0 nos arquivos tocados.
- `cargo deny check` → **0** (advisories/bans/licenses/sources ok); `cargo audit` → **0** (3 unmaintained allow-listados pré-existentes).
- `cargo check -p desktop-gpui --all-targets` → 0 (wiring do `main.rs` type-checked após o WIP da UI assentar; inicialmente bloqueado por `app.rs:231` com `on_go_home`/`on_go_projects` inexistentes — corrigido pela sessão de UI às 12:42).
- E2E: `jobs-recovery.ps1` **RESULT=PASS** (8 asserts, 0); `capture-outbox.ps1` **RESULT=PASS** (31 asserts, 0) — ambos com o binário novo (boot com `load_or_seed` do perfil incluído).
- `Cargo.lock`: mudança aditiva (reqwest/native-tls/schannel/keyring/…); conjunto de dependências estável pós-fix (`cargo metadata --locked` exit 0).

## Dívida registrada

- **Teste de conexão/"teste com resposta estruturada"** (§7 linha 403) e a **tela Settings → IA e privacidade** ficam para o front (sessão dedicada) + próximo ticket de wiring; a façade pública `AiSettings` já expõe `preview`/`grant`/`revoke`/`set_secret`/`status` para consumo.
- E2E não cobre o caminho de provedor externo real (mock HTTP é nível de crate); dogfood (ticket 18) exercitará com endpoint local real.
- `KeyringSecretStore` é Windows-only por construção; provider local sem `Authorization` aceito (secret vazio ⇒ header omitido).
- Perfil fora do SQLite significa que não há histórico/audit trail de consentimento além do próprio arquivo (proveniência completa = ticket 14).
- Provider nativo OpenAI e OpenCodeGateway continuam fora (§12: apenas um provider real no MVP).
- Ambiente: WDAC bloqueia build scripts de `windows-sys 0.60` (motivo do pino do keyring); SAC segue bloqueando exes de teste intermitentemente.

## Pendências resolvidas depois do fechamento

- Teste com resposta estruturada: `ai_provider::test_connection` envia só evidência sintética fixa pelo mesmo gate de consentimento e valida a resposta; nada é persistido — `c6712d8`. A tela continua com o front.
