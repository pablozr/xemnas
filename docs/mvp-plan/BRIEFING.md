# Briefing de implementação do MVP

## Objetivo e aceite

Provar, no Windows, o ciclo OpenCode → Capture Envelope → persistência deduplicada → extração → Decision Inbox → confirmação humana → Engineering Decision pesquisável e exportável, sem interromper o trabalho nem enviar dados externos sem consentimento.

O aceite global são os 20 critérios de `../MVP-SPEC.md`, seção 17.

## Contexto e decisões verificadas

- Diretório: raiz do Project, ainda greenfield; não há código ou manifests.
- Fontes: `../MVP-SPEC.md` (autoridade), `../CONTEXT.md` (vocabulário), `../design-system-quiet-glass.md` (UI) e `../stack-e-arquitetura-rust-gpui.md` (complementar).
- `../arquitetura-proposta-app-desktop.md` é histórico e não deve ser usado para escolher .NET/Avalonia.
- Gate 0 decide explicitamente aceitar GPUI ou trocar **somente** a camada de UI. Gate 1 só começa após essa decisão.

## Contratos e restrições

- Rust gera JSON Schema; o Adapter OpenCode TypeScript só consome o contrato versionado.
- UI usa casos de uso Rust; HTTP local é exclusivamente a fronteira de adapters.
- `domain` não conhece GPUI, SQLite, HTTP, OpenCode ou providers de IA.
- Capture Envelope e Source Artifact não são Decision Candidate; IA não confirma Engineering Decision.
- Outbox é transporte, não banco; captura persiste antes de análise e falha de IA não a perde.
- API: apenas loopback, bearer token, limites, timeout, schemas estritos e sem CORS permissivo.
- Sem mutação automática do Project, arquivos ou commits. Dados externos só após consentimento.

## Padrão e validação esperados

Os primeiros tickets criam os comandos. A linha mínima prevista é: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo audit`, `cargo deny check` e validação TypeScript/contrato/E2E relevante.

Cada ticket entrega comportamento demonstrável, testes proporcionais, erros/estados vazios aplicáveis, logs sanitizados e documentação/ADR quando mudar uma decisão.
