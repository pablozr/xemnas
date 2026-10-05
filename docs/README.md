# Documentação do xemnas

Toda a documentação do produto vive nesta pasta e é versionada junto com as
alterações pertinentes. O `README.md` da raiz é a apresentação do projeto e o
`AGENTS.md` da raiz traz as regras de trabalho dos agentes.

## Mapa

| Pasta | O que tem | Comece por |
| --- | --- | --- |
| [produto/](produto/) | O que o produto é e o escopo aprovado | [MVP-SPEC.md](produto/MVP-SPEC.md) (especificação e escopo), [CONTEXT.md](produto/CONTEXT.md) (vocabulário obrigatório), [visao-consolidada-do-produto.md](produto/visao-consolidada-do-produto.md) (longo prazo) |
| [design/](design/) | Identidade visual e design system do app desktop | [VISUAL-IDENTITY.md](design/VISUAL-IDENTITY.md) (regra vigente), [design-system-quiet-glass.md](design/design-system-quiet-glass.md) (referência original) |
| [arquitetura/](arquitetura/) | Stack, módulos e decisões técnicas | [mapa-do-codigo.md](arquitetura/mapa-do-codigo.md) (arquivos e testes atuais), [stack-e-arquitetura-rust-gpui.md](arquitetura/stack-e-arquitetura-rust-gpui.md), [desempenho-e-escala.md](arquitetura/desempenho-e-escala.md) (performance e baixo custo), [qualidade-do-nucleo.md](arquitetura/qualidade-do-nucleo.md) (portões de assertividade e desempenho do núcleo), [adr/](arquitetura/adr/) (decisões registradas), [historico/](arquitetura/historico/) (propostas superadas) |
| [roadmap/](roadmap/) | Planos e tickets de execução, por fase | [mvp/BRIEFING.md](roadmap/mvp/BRIEFING.md) e [mvp/issues/](roadmap/mvp/issues/) (MVP), [fase-3/](roadmap/fase-3/), [fase-4/](roadmap/fase-4/), [fase-5/](roadmap/fase-5/) |
| [operacao/](operacao/) | Como rodar, dados locais, integrações, limitações e uso real | [operacao-e-referencia.md](operacao/operacao-e-referencia.md), [teste-captura-opencode.md](operacao/teste-captura-opencode.md), [dogfood-log.md](operacao/dogfood-log.md) |
| [pesquisas/](pesquisas/) | Pesquisas e ideias para o futuro, com status | [README.md](pesquisas/README.md) (índice), [memoria-semantica-local-first.md](pesquisas/memoria-semantica-local-first.md) (recuperação local, automação e custos) |
| [assets/](assets/) | Imagens usadas pelo README da raiz | — |

Pesquisas recentes: [escalar grafos e blocos](pesquisas/escalabilidade-renderizacao-fontes.md),
com diagnóstico local, alternativas e benchmark reproduzível; e
[plano de design para a vitrine](pesquisas/plano-design-vitrine.md), com capturas
atuais e [catálogo de referências](pesquisas/design-vitrine-fontes.md).

Expansão do produto: [oportunidades para a memória decisional](pesquisas/oportunidades-produto-memoria-decisional.md),
com dez propostas, fontes de mercado, prioridades e experimentos; pesquisa
exploratória, sem alteração do escopo aprovado.

Pesquisa empírica: [vinte ideias adicionais para dores de desenvolvedores](pesquisas/20-ideias-dores-reais-devs.md),
com quatro catálogos de fontes científicas, limites dos estudos e experimentos
propostos de investigação, compreensão, coordenação e verificação de mudanças.

Síntese para evolução: [melhorias após a avaliação real](pesquisas/melhorias-apos-avaliacao-real.md),
com prioridades, critérios de conclusão e sequência de confiança, contexto e valor.
O [refinamento do próximo ciclo](pesquisas/ideias-para-ciclo-avaliacao-real.md)
acrescenta fontes primárias e sete experimentos localizados de fidelidade,
relevância, cobertura e esforço de revisão; nenhuma implementação foi feita.

## Precedência

Revisão por exceção: [implementação e evidências](operacao/revisao-por-excecao-e-episodios.md),
precedida pelo [piloto comparativo executado](operacao/resultado-piloto-recuperacao.md).
Grupos reduzem repetição; não autorizam normas automaticamente.

Visão visual local: [Xemnas, por dentro](operacao/visao-projeto.html).
Última entrega: [avaliação completa e router IA opcional](operacao/avaliacao-final-e-router-ia.md),
com resultados controlados e [protocolo de produtividade](operacao/experimento-produtividade-assertividade.md).

Modo automático e autoridade das normas: [ADR-0012](arquitetura/adr/0012-modo-automatico-e-autoridade.md)
(regras locais só aceitam com confiança calibrada; o resto vai ao juiz de IA).

Memória descritiva automática: [operação e medições](operacao/memoria-descritiva-e-router.md)
e [ADR-0011](arquitetura/adr/0011-observacoes-descritivas-de-manifests.md), com
manifests, proveniência, invalidação e roteamento determinístico de baixo custo.

Entrega e ressalvas: [lote de confiança, clareza e medição](operacao/lote-confianca-clareza.md),
com builds/capturas, baseline e limites de testes/SAC e arquitetura preexistente.
Contrato: [ADR-0010](arquitetura/adr/0010-qualificadores-e-fronteira-de-envio.md).
A [direção de memória de baixo atrito](pesquisas/direcao-memoria-contexto-baixo-atrito.md)
registra objetivos confirmados e distingue-os da automação ainda não implementada.

Avaliação operacional: [ciclo supervisionado com agentes](operacao/avaliacao-agentes.md),
com piloto GPT-6 Luna medium, controles sem memória/ADR/MCP, evidências e limites.
A [avaliação em repositório real](operacao/avaliacao-repositorio-real.md) reúne cinco críticas
Luna medium, testes no ripgrep, auditoria do core, valor e pendências de cobertura.

Revisão backend sob demanda: [ADR-0009 — revisão consultiva de conhecimento](arquitetura/adr/0009-revisao-consultiva-de-conhecimento.md).

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
  A avaliação de recuperação local segue a
  [metodologia do benchmark semântico](pesquisas/metodologia-benchmark-semantico.md).
- **Como operar, configurar ou diagnosticar:** `operacao/`.
- **Imagem para o README da raiz:** `assets/readme/`.
