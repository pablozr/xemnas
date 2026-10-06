# Documentação do xemnas

Toda a documentação do produto vive nesta pasta e é versionada junto com as
alterações pertinentes. O `README.md` da raiz é a apresentação do projeto e o
`AGENTS.md` da raiz traz as regras de trabalho dos agentes.

## Mapa

| Pasta | O que tem | Comece por |
| --- | --- | --- |
| [produto/](produto/) | O que o produto é e faz hoje, e o vocabulário | [estado-atual.md](produto/estado-atual.md) (escopo vigente), [CONTEXT.md](produto/CONTEXT.md) (termos do domínio) |
| [arquitetura/](arquitetura/) | Stack, módulos, decisões técnicas e qualidade | [mapa-do-codigo.md](arquitetura/mapa-do-codigo.md) (onde está cada fluxo), [contexto-e-agentes.md](arquitetura/contexto-e-agentes.md) (pack, injeção, MCP e hooks), [desempenho-e-escala.md](arquitetura/desempenho-e-escala.md), [qualidade-do-nucleo.md](arquitetura/qualidade-do-nucleo.md), [stack-e-arquitetura-rust-gpui.md](arquitetura/stack-e-arquitetura-rust-gpui.md), [adr/](arquitetura/adr/) |
| [design/](design/) | Identidade visual, design system e idiomas do app | [VISUAL-IDENTITY.md](design/VISUAL-IDENTITY.md) (regra vigente), [idiomas.md](design/idiomas.md), [design-system-quiet-glass.md](design/design-system-quiet-glass.md) (referência original) |
| [roadmap/](roadmap/) | O que vem a seguir e o que já foi entregue | [README.md](roadmap/README.md), [tickets/](roadmap/tickets/) |
| [operacao/](operacao/) | Como rodar, dados locais, integrações, uso real e avaliações | [operacao-e-referencia.md](operacao/operacao-e-referencia.md), [dogfood-log.md](operacao/dogfood-log.md) (gerado todo dia), [avaliacoes.md](operacao/avaliacoes.md) |
| [pesquisas/](pesquisas/) | Ideias e investigações para o futuro, com status | [README.md](pesquisas/README.md), [backlog-de-ideias.md](pesquisas/backlog-de-ideias.md), [mercado-e-ideias.md](pesquisas/mercado-e-ideias.md) |
| [historico/](historico/) | Documentos substituídos, só como registro (inclui a especificação do MVP) | [README.md](historico/README.md) |
| [assets/](assets/) | Imagens usadas pelo README da raiz | — |

## Precedência

Quando dois documentos divergirem, vale nesta ordem:

1. O código. Documento que diverge dele é corrigido no mesmo commit.
2. Os ADRs em `arquitetura/adr/`, para decisões técnicas e de escopo.
3. `produto/estado-atual.md` para o que o produto faz, e `produto/CONTEXT.md` para os termos.
4. `design/VISUAL-IDENTITY.md` para tudo que é visual; o design system original e
   `apps/desktop-gpui/src/ui/tokens.rs` detalham a mesma direção.
5. Os demais documentos de `arquitetura/` como referência técnica.
6. `roadmap/` e `pesquisas/` descrevem o futuro, não o estado atual. `historico/` é só registro.

## Onde colocar algo novo

- **Decisão técnica ou mudança de escopo:** um ADR novo em `arquitetura/adr/`, e o
  `produto/estado-atual.md` atualizado no mesmo commit se o produto mudou.
- **Trabalho planejado:** um ticket em `roadmap/tickets/`, seguindo o
  [README do roadmap](roadmap/README.md). Ao fechar, a linha vai para "Entregue" e o
  arquivo sai no mesmo commit.
- **Regra visual ou padrão de tela:** `design/VISUAL-IDENTITY.md`, no mesmo commit do código.
- **Pesquisa, investigação ou ideia:** `pesquisas/`, seguindo o [índice](pesquisas/README.md).
  Ao implementar, o duradouro vai para o documento oficial e a pesquisa sai.
- **Como operar, configurar ou diagnosticar:** `operacao/operacao-e-referencia.md`.
- **Resultado de avaliação:** uma seção em `operacao/avaliacoes.md`, não um relatório novo.
- **Imagem para o README da raiz:** `assets/readme/`.

Relatórios de entrega, logs crus e changelogs não entram em `docs/`: o que mudou fica no
histórico do git, e o que vale para frente vai para o documento oficial do assunto.
