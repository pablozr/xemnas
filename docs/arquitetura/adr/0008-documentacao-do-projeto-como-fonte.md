# ADR-0008: Documentação do projeto como fonte

- **Status:** Vigente. (registro original: aceito)
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

- **Só as partes centrais seguem (2026-10-02).** Mandar o documento inteiro ao extrator gerava ruído na Revisão: documentação é longa e quase toda descritiva (instalação, uso, planos, changelog). `documents::digest` escolhe, sem modelo e sem custo, o que vale ler: de um ADR, contexto, decisão, consequências e alternativas (e só se o status for aceito); de um README, guia ou especificação, as seções cujo título anuncia uma escolha (arquitetura, decisões, convenções, restrições, princípios, consequências, alternativas, stack) ou cujo texto tem frases que afirmam uma regra ("decidimos", "em vez de", "nunca", "must not"). Instalação, uso, licença, exemplos e roteiro ficam de fora, e código é trocado por uma nota. Um documento sem parte central (ou de caminho `changelog`, `roadmap`, `pesquisa`, tradução) não é enfileirado. O digest tem até 6.000 caracteres (antes 40.000), e dentro de cada tipo os documentos que mais falam em decisões vêm primeiro.

## Alternativas rejeitadas

- **Mandar os arquivos inteiros ao provedor:** caro e arriscado; o esboço basta para resumir, e o digest basta para extrair.
- **Pedir ao modelo que escolha o que importa:** gastaria tokens justamente no que se quer cortar; as regras determinísticas são auditáveis e de custo zero. Aprender com o que a pessoa dispensa (por caminho e título de seção) fica como evolução, medida pela taxa de dispensa.
- **Tratar ADRs como decisões confirmadas:** documentos envelhecem; só o usuário confirma.
