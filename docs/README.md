# Documentação do xemnas

Toda a documentação do produto vive nesta pasta e é versionada junto com as
alterações pertinentes. O `README.md` da raiz é a apresentação do projeto e o
`AGENTS.md` da raiz traz as regras de trabalho dos agentes.

## Mapa

| Pasta | O que tem | Comece por |
| --- | --- | --- |
| [produto/](produto/) | O que o produto é e o escopo aprovado | [MVP-SPEC.md](produto/MVP-SPEC.md) (especificação e escopo), [CONTEXT.md](produto/CONTEXT.md) (vocabulário obrigatório), [visao-consolidada-do-produto.md](produto/visao-consolidada-do-produto.md) (longo prazo) |
| [design/](design/) | Identidade visual e design system do app desktop | [VISUAL-IDENTITY.md](design/VISUAL-IDENTITY.md) (regra vigente), [design-system-quiet-glass.md](design/design-system-quiet-glass.md) (referência original) |
| [arquitetura/](arquitetura/) | Stack, módulos e decisões técnicas | [stack-e-arquitetura-rust-gpui.md](arquitetura/stack-e-arquitetura-rust-gpui.md), [adr/](arquitetura/adr/) (decisões registradas), [historico/](arquitetura/historico/) (propostas superadas) |
| [roadmap/](roadmap/) | Planos e tickets de execução, por fase | [mvp/BRIEFING.md](roadmap/mvp/BRIEFING.md) e [mvp/issues/](roadmap/mvp/issues/) (MVP), [fase-3/](roadmap/fase-3/), [fase-4/](roadmap/fase-4/), [fase-5/](roadmap/fase-5/) |
| [operacao/](operacao/) | Como rodar, dados locais, integrações, limitações e uso real | [operacao-e-referencia.md](operacao/operacao-e-referencia.md), [teste-captura-opencode.md](operacao/teste-captura-opencode.md), [dogfood-log.md](operacao/dogfood-log.md) |
| [pesquisas/](pesquisas/) | Pesquisas e ideias para o futuro, com status | [README.md](pesquisas/README.md) (índice), [memoria-semantica-local-first.md](pesquisas/memoria-semantica-local-first.md) (recuperação local, automação e custos) |
| [assets/](assets/) | Imagens usadas pelo README da raiz | — |

## Precedência

Quando dois documentos divergirem, vale nesta ordem:

1. `produto/MVP-SPEC.md` para escopo, e `produto/CONTEXT.md` para os termos do domínio.
2. Os ADRs em `arquitetura/adr/`, que registram decisões posteriores à especificação.
3. `design/VISUAL-IDENTITY.md` para tudo que é visual; o design system original
   e `apps/desktop-gpui/src/ui/tokens.rs` detalham a mesma direção.
4. `arquitetura/stack-e-arquitetura-rust-gpui.md` como referência técnica
   complementar.
5. `produto/visao-consolidada-do-produto.md` e `pesquisas/` descrevem o futuro, não
   o escopo atual. `arquitetura/historico/` é só registro.

## Onde colocar algo novo

- **Decisão técnica ou mudança de escopo:** um ADR novo em `arquitetura/adr/`.
- **Plano de fase ou ticket:** `roadmap/<fase>/`. Ao fechar um ticket, anote no
  próprio ticket o que foi feito e o commit.
- **Regra visual ou padrão de tela:** `design/VISUAL-IDENTITY.md`, no mesmo commit
  do código.
- **Pesquisa, investigação ou ideia futura:** `pesquisas/`, seguindo as regras do
  [índice da pasta](pesquisas/README.md).
- **Como operar, configurar ou diagnosticar:** `operacao/`.
- **Imagem para o README da raiz:** `assets/readme/`.
