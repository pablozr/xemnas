# ADR-0008: Documentação do projeto como fonte

- **Status:** aceito
- **Data:** 2026-09-30
- **Contexto:** o app só conhecia o que vinha dos diffs capturados. Projetos com `docs/`, `specs/`, ADRs e README pareciam vazios na Visão, embora a intenção do projeto já estivesse escrita.

## Decisão

1. **Indexar localmente a documentação da pasta do projeto** (`application::documents`): `README.md`, `ARCHITECTURE.md`, `DESIGN.md`, `CONTRIBUTING.md`, `AGENTS.md`, `CLAUDE.md`, `SPEC.md` na raiz e Markdown/texto em `docs/`, `doc/`, `documentation/`, `specs/`, `spec/`, `adr/`, `adrs/`, `rfcs/`, `architecture/`, `design/`. Ignora pastas ocultas, `node_modules`, `target`, `dist`, `build`, `vendor`, `out`; no máximo 400 arquivos de até 512 KiB, 6 níveis.
2. **Guardar só o esboço:** título, seções, primeiro parágrafo (600 caracteres), tipo (ADR, especificação, README, guia), tamanho e impressão digital (migration 17, `project_documents`). O texto completo não é copiado.
3. **Documento é fonte, não autoridade.** Entra na Visão como referência citável (`F:` + 8 hex do caminho), dentro de um orçamento (40 documentos, 16 mil caracteres), e o prompt diz que, em conflito, a decisão confirmada vence. Nunca vira decisão ou regra sozinho.
4. **Leitura a cada visita e antes de gerar a Visão;** pasta inacessível mantém a última leitura. Contexto lista os documentos por tipo, com "Ler de novo".

## Consequências

- O consentimento da Visão passa a citar títulos, seções e o primeiro parágrafo da documentação; nada do código.
- Remover o projeto apaga o índice.
- **Documentos viram candidatos (2026-10-01).** Ao gerar a Visão (provedor ativo), até 12 documentos novos ou alterados por vez, ADRs e especificações primeiro, viram capturas com um artefato `document` e um job de análise, pelo mesmo caminho das conversas: consentimento, extração, reconciliação, Revisão e, confirmados, sugestões do Mapa. Cada versão (caminho + impressão digital) entra uma vez só. Os arquivos da captura são o próprio documento e os caminhos de código que ele cita, então o que ele decide se liga aos componentes. O extrator offline ignora documentos (só repetiria os sinais), e o prompt pede só o que o documento afirma como decidido, não planos, passos ou instruções a agentes.

## Alternativas rejeitadas

- **Mandar os arquivos inteiros ao provedor:** caro e arriscado; o esboço basta para resumir.
- **Tratar ADRs como decisões confirmadas:** documentos envelhecem; só o usuário confirma.
