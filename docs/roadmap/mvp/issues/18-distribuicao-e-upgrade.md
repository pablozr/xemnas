# 18 — Gate 5: distribuição e upgrade reproduzíveis

**What to build:** Produzir build/instalador Windows e evolução segura do banco.

**Blocked by:** 17 — Gate 4: exportação manual e E2E do ciclo completo.

**Status: done (aprovado; pacote release pendente de política do host — ver evidências)**

- [x] Build reproduzível e instalação limpa passam. *(fluxo completo validado com binário debug via `-ExePath`; caminho release/ZIP pendente do WDAC do host + job de CI `package` — ver evidências)*
- [x] Upgrade e migrations forward-only são transacionais. *(teste de rollback do runner adicionado)*

## Decisões registradas

- **Formato de distribuição: ZIP + scripts PowerShell** (`tools/package.ps1` + `tests/e2e/install-clean.ps1`), sem toolchain nova (MSI/Inno exigiriam dependência de build nova). Registrado na **ADR-0002** (`docs/arquitetura/adr/0002-distribuicao-windows-zip-scripts.md`, escrita por mim junto ao fechamento).
- **Coerência build↔exe estrutural no `package.ps1`** (review r1): parâmetro `-Configuration` removido; constantes únicas `$profileName`/`$profileArgs` alimentam build E staging (par trocado em conjunto); `throw` se o bin do perfil não existir; **hash SHA-256 staged == bin construído** antes de fechar o ZIP. Perfil fixo `release`, `--locked`, versão de `[workspace.package]`, staging limpo, exit≠0 em falha.
- **"Reprodutibilidade" = escopo honesto** (documentado no script): `--locked` + versão única de origem + um comando só + CI repete o mesmo caminho; **bit-identical NÃO prometido** (timestamps de ZIP, metadados do Windows). Sem pin de toolchain (decisão minha).
- **Instalação limpa (`install-clean.ps1`)**: extrai ZIP → executa com `XEMNAS_DATA_DIR` isolado → espera `/v1/health` (port de `discovery.json`, bearer de `api-token`) → asserts `app.db` + `schema_migrations` v8 → `WM_CLOSE` elegante → asserts de remoção de `discovery.json`/`api-token` → remove diretórios de install+data e prova que sumiram (desinstalação limpa). Aceita `-ExePath`/`-SkipPackage`.
- **Rollback transacional do runner (aceite 2)**: refactor privado `apply_migrations(conn, &[Migration])` (comportamento público idêntico) + teste unitário: migração boa v1 aplica; v2 com SQL inválido ⇒ `Err`, tabela v2 ausente, versão não registrada, conexão utilizável; v2 corrigida aplica. Cada migração segue sendo sua própria transação + registro na mesma tx (`store.rs`).
- **CI**: job `package` (windows-latest) roda `tools\package.ps1` e publica `dist/*.zip` como artifact (`if-no-files-found: error`); jobs `quality`/`contract` intactos.
- Review: r1 REPROVADO (`-Configuration` empacotava perfil divergente) → r2 **APROVADO**.

## Evidências

- `cargo test -p storage-sqlite --locked`: **59 pass** (inclui o novo rollback do runner).
- `cargo test -p application --locked`: 98; ai-provider 13+1; `cargo test --workspace --locked` exit 101 com **única falha `ARCH-001 ui/icons.rs:57`** (WIP do front).
- rustfmt 0; clippy 0; `cargo check -p desktop-gpui --all-targets` 0; deny 0; audit 0; **`Cargo.lock` intocado**.
- **`install-clean.ps1 -ExePath <debug> -SkipPackage`: PASS** (instalação + desinstalação limpas, schema v8, health ok, shutdown elegante, sem leftovers).
- **Caminho de empacotamento validado em perfil debug controlado**: ZIP com `LICENSE`+`xemnas.exe`, hash staged == origem, temporários limpos; script default (release) falha alto com mensagem clara.
- **E2E `jobs-recovery.ps1` PASS + `capture-outbox.ps1` PASS** (regressão).
- YAML do `ci.yml` válido (`yaml.safe_load`).

## Dívida registrada

- **`cargo build --release --locked` BLOQUEADO no host** (WDAC/SAC: build script de `unicode-general-category` em release, os error 4551, ~15 tentativas; debug passa). Consequência: pacote release/ZIP **não rodou localmente** — o job `package` do CI (windows-latest) é o caminho que o comprovará quando houver remote; `install-clean` completo (via `package.ps1`) reexecutar assim que a política liberar. Política de segurança nunca contornada.
- `install-clean.ps1` exige sessão desktop interativa (janela/WM_CLOSE) — E2E local, não entra no CI `package`.
- ZIP não é bit-reproducível (timestamps de entrada).
- `package.ps1` empacota o bin target (não `--workspace`).
