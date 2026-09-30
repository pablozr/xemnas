# Fase 3 — contexto recuperável e Context Pack manual

**O que foi construído:** relações entre decisões, Context Claims temporais e o Context Pack manual com exportação, no backend (ADR-0003).

**Status: done (backend; telas com a sessão de front)**

- [x] Substituir uma decisão marca a anterior como `superseded`, registra a relação e não apaga nada.
- [x] Claims tipadas com validade; listar pelas válidas numa data; encerrar sem apagar.
- [x] Pack com decisões vigentes e claims válidas numa data de referência, relevância lexical, citações e orçamento.
- [x] Exportação explícita em Markdown ou JSON com preview byte a byte.

## Decisões registradas

- **Relações** (`domain::relations`): `supersedes`, `depends_on`, `conflicts_with`; sem auto-referência, sem duplicata, `conflicts_with` simétrica e sem ciclo dentro do mesmo tipo. Só entre decisões `accepted` do mesmo projeto. Migration 0010 com índice único que permite um único substituto por decisão; a marcação `superseded` é compare-and-set na mesma transação.
- **Claims** (`domain::claims`): `assumption`, `constraint`, `goal`, `convention`; `valid_from` inclusivo e `valid_until` exclusivo; datas `AAAA-MM-DD` ou `AAAA-MM-DDTHH:MM:SSZ` (`domain::time::Timestamp`). A afirmação é imutável: corrigir é encerrar e criar outra. Decisão de origem opcional, sempre do mesmo projeto. Migration 0011 com FTS5 `claims_fts`.
- **Seleção do pack** (`application::context`): termos úteis da tarefa em OR no FTS5, ordem BM25; decisão vigente = confirmada até a data e sem substituição registrada até ela; claims válidas na data. Ordem: decisões relevantes, claims relevantes, depois restrições e convenções válidas mesmo sem casar. Decisões sem relação com a tarefa não entram. Orçamento de 500 a 50.000 caracteres (padrão 8.000), com contagem do que ficou de fora. O pack não é persistido.
- **Exportação** (`application::export`): `preview_pack` e `write_pack` reutilizam a escrita segura da exportação de decisões.

## Contrato para o front

Todos fazem I/O: rodar fora da thread de UI (ASYNC-001).

| Caso de uso | Chamadas | Erros (`code()`) |
| --- | --- | --- |
| `DecisionRelations::new(store)` | `supersede(nova, antiga)`, `relate(origem, destino, RelationKind)`, `of(decision_id)` | `not_found`, `conflict`, `invalid_relation` |
| `Claims::new(store)` | `create(NewClaim)`, `list(project_id, as_of)`, `retire(claim_id, at)` | `project_not_found`, `invalid_source`, `empty_statement`, `statement_too_long`, `invalid_date`, `inverted_validity`, `already_ended`, `conflict` |
| `ContextPacks::new(store)` | `build_pack(ContextRequest)` (trait `ContextProvider`) | `invalid_request`, `project_not_found` |
| `export` | `preview_pack(&pack, ExportFormat)`, `write_pack(&doc, destino, overwrite)` | `destination_exists`, `destination_invalid`, `io` |

A lista de decisões continua mostrando só as `accepted` por padrão; as `superseded` aparecem com `DecisionFilter { statuses: vec![Superseded], .. }`.

## Evidências

- Testes: `domain` (relações, claims, timestamp), `storage-sqlite/tests/{relations,claims,context_pack}.rs`, `application/tests/pack_export.rs`; suíte de backend com 295 testes verdes, fmt e clippy `-D warnings` limpos.
- Upgrade testado de v9 → v10 e v10 → v11 com dados existentes.

## Pendências registradas

- Telas: substituir/relacionar decisões, criar e encerrar claims, montar e exportar o pack (sessão de front).
- Medir a utilidade do pack no uso real antes de expô-lo a agentes (Fase 5, MCP).
- A relevância é lexical: sinônimos e paráfrases não casam (AD-14; busca semântica só após medir).
- Timeline por projeto: as datas já existem (confirmação, relações, validade das claims); a consulta dedicada fica para quando a tela for desenhada.
