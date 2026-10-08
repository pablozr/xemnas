# 29 — Observações de manifest sem churn e com a raiz

**What to build:** Manter as observações descritivas de manifest ([ADR-0011](../../arquitetura/adr/0011-observacoes-descritivas-de-manifests.md)) sem reescrita a cada refresh e cobrindo o manifest da raiz do workspace.

**Blocked by:** nada.

**Status: open**

- [ ] Não regravar a observação quando o manifest não mudou (hash do conteúdo), medindo as escritas por refresh.
- [ ] Ler também o manifest da raiz (workspace, toolchain, MSRV) como observação do componente `workspace`.

**Aceite:** teste de idempotência com contagem de escritas e de cobertura da raiz; sem IA.
