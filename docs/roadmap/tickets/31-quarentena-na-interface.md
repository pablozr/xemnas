# 31 — Quarentena de capturas na interface

**What to build:** Mostrar na interface as capturas de pastas não cadastradas que esperam na quarentena da outbox (`unregistered/`) e deixar a pessoa agir sobre elas.

**Blocked by:** nada (o backend entregue em `feat(outbox): quarantine captures of unregistered projects` já expõe `DrainReport.quarantined`, `quarantined_by_root` e `vanished`).

**Status: open**

- [ ] Estado vazio, carregando e erro recuperável seguindo `docs/design/VISUAL-IDENTITY.md`, sem padrão novo de tela.
- [ ] Lista de raízes com contagem (virtualizada), marcando as pastas que não existem mais (`vanished`).
- [ ] Ação "cadastrar esta pasta" que chama o caso de uso de projetos e, em seguida, `outbox::drain` para liberar a quarentena na hora (hoje a liberação só acontece no próximo drain, na abertura do app).
- [ ] Ação "descartar" por raiz, com confirmação, que move os itens para `rejected/` com um código próprio.
- [ ] Textos nos dez idiomas; atalho e entrada na paleta (Ctrl K).

**Aceite:** captura de uma pasta nova aparece na lista, cadastrar a pasta a importa sem reiniciar o app; contagem lida do índice, sem reler os payloads (custo medido com `XEMNAS_PERF=1`).
