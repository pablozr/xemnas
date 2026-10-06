# Avaliações do núcleo: o que mostraram

Resumo de seis avaliações feitas em 03–04/10/2026, com agentes de modelo e
dados sintéticos ou descartáveis. Os relatórios completos, logs, runner e
capturas foram removidos; ficam no histórico do Git (commits anteriores a esta
página). Todas medem **recuperação e contratos técnicos**; nenhuma mede
produtividade humana, tempo humano ativo ou custo faturado. Testes verdes
sozinhos não aprovam o produto: erro crítico (decisão substituída tratada como
vigente, justificativa inventada) reprova a fidelidade da tarefa.

## 1. Piloto supervisionado RelayDesk (03/10)

- **Pergunta:** um agente novo recupera motivos que não estão no código?
- **Método:** cenário sintético (PostgreSQL substituído por SQLite individual
  offline, busca FTS5, sem regra de retenção); runner em Rust sobre os casos de
  uso reais, extrator fake e confirmação por script; três agentes novos (sem
  memória, ADR equivalente, Xemnas via MCP) com as mesmas cinco perguntas.
- **Resultado:** sem memória declarou as lacunas (abstenção correta); ADR e
  Xemnas recuperaram os mesmos quatro assuntos, distinguiram a decisão
  substituída e não inventaram retenção. O ADR usou menos passos. Consultas MCP
  de 22–35 ms (relatadas pelo agente). Ingestão repetida não duplicou registros.
  Busca por "localizar por significado" não achou nada, enquanto "sinônimos"
  achou a decisão FTS5: dependência de vocabulário lexical.
- **Limites:** extração fake com escolhas fornecidas por script (não prova
  qualidade de extração por IA); sem tempo total comparável; o agente com Xemnas
  precisou de ajuda com o transporte JSON-RPC por terminal.
- **Mudou no produto:** nada diretamente; motivou o experimento 5.

## 2. Repositório real: ripgrep (03/10)

- **Pergunta:** o fluxo completo funciona num projeto real?
- **Método:** clone raso do ripgrep, uma decisão local do avaliador (preservar a
  delegação cruzada de `Override::num_ignores/num_whitelists`), release do
  desktop operado pelo coordenador, cinco críticos GPT-6 Luna medium (onboarding,
  captura/revisão, mapa, recuperação, operações), OpenCode real, provider real
  por bridge loopback, controle ADR e leitura estática do núcleo. 20 capacidades
  inventariadas; a maioria exercitada em cenário estreito.
- **Resultado:** o Xemnas recuperou regra, motivo e ressalva por MCP e por
  `file_context` (após vínculo manual no mapa); uma sessão OpenCode real recebeu
  a decisão injetada e a citou. Outbox pending→accepted, replay sem duplicar e
  retry de job falho (2 tentativas) funcionaram. Extração arquitetural real
  funcionou numa fixture explícita; a conversa de rotina local virou `detail`
  (confiança 0,99, significância 0,1) e não gerou candidato. Sem vantagem de
  conteúdo sobre um ADR bem escrito.
- **Achados:** candidato fake parecia lista de sinais e precisou ser reescrito;
  consulta irrelevante ("cor do terminal") trouxe falso positivo; busca em
  inglês não achou a regra em português; mapa inicial só com pastas e zero
  relações; Visão saiu em espanhol e com etapas sem entidades; `file_context`
  vazio sem vínculo; fila vazia não distingue "nada durável" de "não
  processado". Riscos estáticos: adoção confirma antes de gravar relações,
  chave de idempotência sem comparar payload, sugestões de claims sem
  `redact_secrets` (corrigido depois: o texto passa por `external::protected_text`;
  os outros dois não foram reconferidos em 06/10).
- **Limites:** cinco críticos são o mesmo modelo, não pesquisa de usabilidade; o
  teste escrito no clone não rodou (Controle de Aplicativo, erro 4551);
  `review_time_ms` = 225 s é contador do app, não tempo humano; o bridge de
  provider não é integração de produto.
- **Mudou no produto:** as prioridades (descarte legível, privacidade de todos
  os envios, qualidade de retrieval com negativos, onboarding, limites do
  mapa/Visão, preservar escopo "local/upstream") alimentaram o item 3.

## 3. Lote de confiança, clareza e medição (03/10)

- **Pergunta:** fechar as lacunas do item 2 e criar uma linha de base de busca.
- **Mudou no produto:** redação comum antes de qualquer envio externo;
  autorização revalidada antes de chamadas e retries; prévia de consentimento
  ampliada (documentos, conhecimento revisado, mapa; o hash muda); qualificadores
  de atribuição/alcance/validação; CAS contra snapshot velho; pacote com
  unidades inteiras e omissões registradas; progresso e assessment por tentativa;
  Revisão e Diagnóstico com capturas recentes. Migrations 21–24.
- **Medição:** corpus de 30 consultas (PT/EN/distratores). Pack/compact300:
  precisão 17/26 (65,4%), recall 17/18 (94,4%), negativos contaminados 2/15
  (13,3%); `build_pack` p50 366 µs, p95 631 µs. Tokens estimados por caracteres.
- **Limites:** sintético, não prova melhoria; na época a arquitetura passava
  11/12 por falha preexistente de cores em `ui/material.rs` e `ui/wallpaper.rs`.

## 4. Memória descritiva e router de relevância (04/10)

- **Pergunta:** o app lê fatos declarados (Cargo.toml, package.json) sem IA e
  sem virar norma?
- **Mudou no produto:** observações descritivas de manifests (ADR-0011) com
  cache persistente, refresh agendado e worker local; seleção por tarefa/arquivo
  sem vínculo normativo; fonte incerta vira abstenção, não remoção. Juiz de
  relevância opcional (consentimento `context_routing`): só filtra ambiguidades
  de até seis candidatos, cache de 1 h, dois pendentes por projeto, oito
  solicitações lógicas/dia, cooldown de 60 s. Migrations 28–29.
- **Medição:** 16/16 cenários descritivos; fatos exigidos 11/11; pertinentes
  11/14 (78,6%); negativos limpos 3/3. Por escopo (30 amostras, p50/p95 em µs):
  refresh cold 4840/6843, warm 5820/7021, `build_pack` 474/649, render 18/32.
  Em outra rodada, `build_pack` descritivo p50 185/p95 327 µs e normativo
  420/725 µs. Busca normativa inalterada (17/26, 17/18, 2/15).
- **Limites:** o juiz fake prova infraestrutura, não acerto de modelo; só
  `build_pack` foi medido nessa rodada; latência de provider real não medida.

## 5. Experimento N/A/X de recuperação (04/10)

- **Pergunta:** com fontes brutas (N), ADR/memória (A) ou Xemnas (X), um agente
  novo responde corretamente a seis tarefas (SQLite vigente, FTS5 sem sinônimos,
  escolha local não é norma upstream, serde declarado "2" sem versão instalada,
  retenção/cor não definidas, falha de leitura não prova remoção)?
- **Método:** protocolo pré-registrado com oráculo C/J/E/L; piloto adaptado (X
  por snapshots exportados, não API ao vivo; três sujeitos, dois lotes de três
  tarefas). O corpus literal fica em
  `crates/storage-sqlite/tests/fixtures/comparative_protocol.md`, lido por
  `comparative_materials.rs`.
- **Resultado:** N, A e X acertaram 6/6 encaminhamentos (síntese do
  coordenador, sem pontuação cega por campo). Chamadas de ferramenta: N 26,
  A 24, X 32; X não mostrou vantagem de passos nem de correção sobre ADR.
- **Limites:** sem pontuação C/J/E/L, sem tempo por tarefa, modelo dos sujeitos
  não registrado; tempo humano e custo de manter a memória não medidos.
- **Mudou no produto:** motivou reduzir repetição e custo de consulta (item 6).

## 6. Revisão por exceção e episódios (04/10)

- **Mudou no produto:** natureza (description/inference/normative/unknown) na
  mesma chamada de extração; inferência nunca é apresentada como norma;
  descrição só é verificada por correspondência exata com observação elegível;
  ocorrências equivalentes viram uma oportunidade (grupos estritos por projeto,
  tipo, conteúdo, qualificadores e caminhos), com a confirmação criando um alvo
  e um ledger; índice dirty incremental; proveniência imutável por captura e
  episódios de diff (`diff_hunk`; truncado = desconhecido). Migrations 30–34.
- **Medição:** count+list p50/p95 de 5,7/7,6 ms com 800 ocorrências e 57,9/65,7
  ms com 8000 (30 amostras). 800 ocorrências → uma oportunidade é redução
  potencial de repetição, não ganho de produtividade.
- **Limites:** sem validação GUI da apresentação de grupos; classificação de
  natureza não testada com provider real consentido.

## Ainda sem prova

Produtividade e tempo humano ativo; custo líquido de manter memória versus ADR;
injeção ao vivo comparada em tarefas de desenvolvimento; qualidade de extração
com provider real em fixtures públicas; aprovação por conversa (exige contrato
verificável de autoridade, escopo e revisão). O uso real é medido em
[dogfood-log.md](dogfood-log.md).
