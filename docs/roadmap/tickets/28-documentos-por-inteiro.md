# 28 — Documentos por inteiro

**What to build:** Extrair de um documento tudo o que ele decide, não só as primeiras propostas.

**Blocked by:** nada; usa o índice de arquivos do ADR-0016.

**Status: open**

- [ ] `PROPOSE_PER_RUN = 12` em `crates/application/src/documents.rs`, chamado só em `overview.rs`: um ADR com decisões numeradas sai com zero candidatos. Medir quantas decisões numeradas ficam de fora e subir o teto com o custo medido.
- [ ] Seções "Refinements", "Later decision" e "superseded by" viram relação e não enunciado solto.
- [ ] Claims sem sujeito (o enunciado não diz a quem vale) e justificativa virando enunciado: rejeitar na extração.

**Aceite:** corpus rotulado de documentos com precisão e cobertura, e a contagem de candidatos por ADR; sem aumento de chamadas além do medido.
