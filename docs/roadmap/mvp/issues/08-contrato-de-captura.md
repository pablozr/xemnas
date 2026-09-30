# 08 — Gate 2: contrato versionado de captura

**What to build:** Definir Capture Envelope e Source Artifacts em Rust e gerar schema consumível pelo Adapter.

**Blocked by:** 07 — Gate 1: jobs persistidos e recuperáveis.

**Status:** done

- [x] Fixtures válidas, inválidas, grandes, incompletas e incompatíveis são verificadas em Rust e TypeScript.
- [x] O contrato preserva semântica, versão e idempotência.

## Decisões registradas

- O contrato mora em `crates/integration-contracts` (fronteira de integração, não regra de negócio): tipos `CaptureEnvelope`, `CaptureSource`, `ProjectRef`, `SourceArtifact` e `ArtifactKind` (snake_case), todos com `deny_unknown_fields` — evolução é por `schema_version` (§13).
- **Fonte da verdade da validação é o JSON Schema gerado** (draft 2020-12): Rust valida com `jsonschema` (dev-dep, `default-features = false` para não puxar reqwest/tokio) e TypeScript com `Ajv2020` estrito + `ajv-formats`. Os tipos Rust dão acesso tipado **depois** da validação; `const: 1` em `schema_version` e `minItems: 1` em `artifacts` vivem só no schema, com doc-comment exigindo validar antes de desserializar (a ingesta do ticket 09 é quem aplica).
- Sem limites numéricos de tamanho em `content` (o bound é da API de ingesta, §7.3/ticket 09); o fixture `valid-large-large.json` (~20 KB) trava essa decisão.
- `capture_id`/`artifact_id` com padrão de UUID v7, `fingerprint` `^[0-9a-f]{64}$`, `observed_at` `format: date-time` (assertado dos dois lados).
- Artefato versionado em `adapters/opencode/schemas/capture-envelope.schema.json`, gerado por schemars e protegido por teste de drift; regeneração: `cargo test -p integration-contracts --test contract -- --ignored regenerate_capture_envelope_schema`.
- Fixtures compartilhadas em `tests/fixtures/capture/` com **convenção de prefixo** como contrato Rust↔TS (`valid-*`, `invalid-*`, `incomplete-*`, `incompatible-*`), documentada no `README.md` do diretório; conteúdo 100% sintético.
- `fingerprint` = SHA-256 UTF-8 do valor `content` decodificado, **verificado por recálculo nos dois lados** (`sha2` no Rust, `node:crypto` no TS) — não só forma lexical.
- Pacote TS `adapters/opencode` sem `src/` (lógica de adaptação = ticket 10), `node:test` + `tsc` sem runner extra, imports ESM com `createRequire` para `ajv-formats` sob `nodenext`.
- CI ganhou job `contract` (ubuntu, node 24, `npm ci` + `npm test`); job `quality` intacto. `.gitignore` ignora `node_modules/`, `adapters/opencode/dist/`, `*.tsbuildinfo`.

## Evidências

- Review round 1 `REPROVADO` (fingerprints das fixtures válidas não batiam com o SHA-256 real; `schema_version` só travado no schema) → correção nas duas frentes → round 2 **`APROVADO`** sem achados novos (reviewer confirmou os 25 artefatos válidos recalculados).
- `cargo fmt --all -- --check` EXIT **0**
- `cargo clippy --workspace --all-targets -- -D warnings` EXIT **0**
- `cargo test --workspace --locked` EXIT **0** → **88 pass + 1 ignored** (regeneração de schema)
- `cargo audit` EXIT **0** (4 warnings unmaintained já allow-listados)
- `cargo deny check` EXIT **0** → só as 11 `warning[duplicate]` do GPUI (`advisories/bans/licenses/sources ok`)
- `npm ci` EXIT **0** (0 vulnerabilidades) · `npm test` EXIT **0** → **17/17 TypeScript**
- `Cargo.lock` atualizado e versionado (jsonschema e transitivos, `sha2` como dev-dep direta).
- 14 fixtures + README; schema gerado sem `format: uint32`, com `minItems: 1`.

## Dívida registrada

- Rejeição de `schema_version` incompatível só no schema; desserialização tipada aceita qualquer `u32`. Deferido ao ticket 09 (ingesta), como a review sugeriu.
- Padrões de UUID v7/`fingerprint`/`date-time` são expressos em JSON Schema; relações (ex.: fingerprint ↔ content) e canonicalização de `canonical_path` não são — a ingesta (09) precisa recalcular/validar por fora.
- `adapters/opencode/src/` não existe ainda (layout previsto no stack); entra no ticket 10.
