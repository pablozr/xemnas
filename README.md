# xemnas — Contextual Engineering

Memória decisional local para engenharia de software de apoio a agentes de programação. O produto observa o trabalho no OpenCode, propõe **Decision Candidates** com evidência e transforma confirmação humana em **Engineering Decisions** versionadas e pesquisáveis — tudo local, sem envio externo sem consentimento explícito.

Especificação e arquitetura: [`docs/MVP-SPEC.md`](docs/MVP-SPEC.md), [`docs/CONTEXT.md`](docs/CONTEXT.md), [`docs/stack-e-arquitetura-rust-gpui.md`](docs/stack-e-arquitetura-rust-gpui.md), ADRs em [`docs/adr/`](docs/adr/).

## Instalação e execução

**Requisitos:** Windows; Rust estável (só para buildar a partir do código).

```powershell
cargo build --release --locked -p desktop-gpui --bin xemnas
.\target\release\xemnas.exe
```

Pacote distribuível (ZIP versionado em `dist\`):

```powershell
.\tools\package.ps1          # build release + staging verificado por SHA-256
.\tests\e2e\install-clean.ps1  # prova instalação/desinstalação limpas
```

**Onde ficam os dados** (tudo local):

| Item | Caminho |
| --- | --- |
| Diretório de dados | `XEMNAS_DATA_DIR` (se definido) ou `%LOCALAPPDATA%\xemnas` |
| Banco SQLite | `<dados>\state\app.db` (migrations forward-only, versão atual 8) |
| API local | somente loopback; token por sessão em `<dados>\api-token`; porta em `<dados>\discovery.json` |
| Outbox de capturas | `XEMNAS_OUTBOX_DIR` ou `<dados>\outbox` (`pending/`, `accepted/`, `rejected/`) |

## Integração com o OpenCode

O adapter em [`adapters/opencode/`](adapters/opencode/) é um plugin fino: disparo em idle → reconcile pela API pública do OpenCode → validação do **Capture Envelope** → `POST /v1/captures` (ou escrita na outbox quando o desktop está fechado). Ele não contém regras de domínio e nunca chama modelo.

```powershell
cd adapters/opencode
npm ci
npm test                      # testes de contrato do envelope
npm run send-fixture          # envia fixture pela outbox (modo CLI)
```

Variáveis relevantes: `OPENCODE_URL` (padrão `http://127.0.0.1:4096`), `XEMNAS_DATA_DIR`, `XEMNAS_OUTBOX_DIR`.

## Privacidade

- **Local por padrão.** Nenhum dado sai da máquina sem consentimento explícito e perfil configurado (§13 da spec).
- A API de IA só é usada com consentimento vigente (`preview_hash` verificado) e segredos no cofre do sistema (keyring) — nunca em texto no SQLite.
- Providers externos exigem HTTPS. HTTP é permitido apenas em IPs de loopback, como `http://127.0.0.1:11434/v1` ou `http://[::1]:11434/v1`; URLs com credenciais, query ou fragmento são rejeitadas e redirecionamentos não são seguidos. O consentimento inclui o endpoint completo: consentimentos anteriores à inclusão desse vínculo exigem nova aprovação.
- **Redação acontece na fronteira do adapter** (`adapters/opencode/src/redact.ts`), antes de qualquer persistência.
- Logs nunca contêm token bearer, prompts, conversas ou diffs; falhas de job são sanitizadas; o **diagnóstico exportado é sanitizado por construção** (só estrutura — sem conteúdo de artefatos, decisões, caminhos ou credenciais).
- Exportação de decisão exige ação explícita, preview e destino escolhido pelo usuário; confirmar não escreve nada no repositório e o produto nunca faz commit.

## Recovery

- Jobs interrompidos voltam para `queued` na reinicialização quando idempotentes; capturas na outbox são importadas depois (com desktop fechado inclusive) sem duplicar (chaves de idempotência + dedup por constraint).
- `cargo test` inclui testes de crash/restart; E2Es: `tests\e2e\jobs-recovery.ps1`, `tests\e2e\capture-outbox.ps1`, `tests\e2e\install-clean.ps1`.

## Limitações conhecidas

- A interface (telas Inbox/Decisions/Settings/Diagnostics e navegação por teclado) é entregue em paralelo — o backend dos fluxos já está completo e aprovado.
- Redação de conteúdo existe no adapter, não no motor Rust: um caller local enviando segredo cru via API o persiste.
- Busca é lexical (FTS5) — sem embeddings/vector graph (spec: provar filtros antes de embeddings).
- `superseded` está modelado no schema, sem ação/UI ainda.
- Builds bit-a-bit reproduzíveis não são prometidos (timestamps Windows); o caminho `--locked` + CI é o mesmo.
- Testes de provedor pago são opt-in; a suíte padrão usa fake/fixtures.
- Estado de conclusão do MVP e dogfood: ver [`docs/mvp-plan/issues/20-dogfood-e-conclusao.md`](docs/mvp-plan/issues/20-dogfood-e-conclusao.md) e [`docs/dogfood-log.md`](docs/dogfood-log.md).

## Desenvolvimento (validação)

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo deny check
cargo audit
```

CI (`.github/workflows/ci.yml`): `quality` (fmt, clippy, testes, audit, deny), `contract` (testes TS do adapter) e `package` (ZIP como artifact).
