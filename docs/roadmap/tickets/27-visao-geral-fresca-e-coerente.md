# 27 — Visão geral fresca e coerente com o código

**What to build:** Fazer a visão do projeto acompanhar o que mudou, em vez de ficar com a foto do dia em que foi gerada (`docs/arquitetura/adr/0007-visao-do-projeto-gerada-pela-ia.md`).

**Blocked by:** nada; é independente do ADR-0016.

**Status: open**

- [ ] A visão gerada ficou com `decisions 0` e sem regenerar depois de decisões, regras e documentos entrarem: regenerar quando decisões, regras ou documentos mudam (com o teto de chamadas do modo automático).
- [ ] Validar os fluxos que a visão descreve contra o grafo de dependências declarado (manifests) e contra os testes de arquitetura, sem IA: um fluxo que cita uma dependência que não existe é sinalizado.

**Aceite:** cada item com teste de assertividade e de desempenho (`python tools/core-quality.py`).
